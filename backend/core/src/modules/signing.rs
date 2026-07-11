//! Module signature verification and self-signing key bootstrap
//! (TR-05-002, TR-05-009).
//!
//! Each module manifest carries an **array** of ed25519 signatures over its
//! [code artifact](super::manifest::Manifest::code_artifact_bytes). The loader
//! accepts the module iff **at least one** signature from a **trusted** signer
//! validates. The backend generates a self-signing keypair on first startup
//! and persists it; the trust store holds that self key plus any configured
//! external signer keys.

use std::collections::HashMap;
use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

/// Signer id of the backend's own self-signing key.
pub const SELF_SIGNER: &str = "self";

/// Why signature verification rejected a module.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VerifyError {
    /// The manifest carried no signatures at all.
    #[error("module has no signatures")]
    NoSignatures,
    /// No signature from a trusted signer validated.
    #[error("no valid signature from a trusted signer")]
    NoTrustedSignature,
}

/// Errors from key persistence.
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("key I/O error: {0}")]
    Io(String),
    #[error("malformed key material: {0}")]
    Malformed(String),
}

/// Public-key trust store keyed by signer id.
#[derive(Default, Clone)]
pub struct TrustStore {
    keys: HashMap<String, VerifyingKey>,
}

impl TrustStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Trust `signer`'s public key.
    pub fn add(&mut self, signer: impl Into<String>, key: VerifyingKey) {
        self.keys.insert(signer.into(), key);
    }

    /// Trust `signer`'s base64 (standard) 32-byte public key.
    ///
    /// # Errors
    /// [`KeyError::Malformed`] if the key is not valid base64 / not 32 bytes.
    pub fn add_base64(&mut self, signer: impl Into<String>, b64: &str) -> Result<(), KeyError> {
        let bytes = STANDARD
            .decode(b64)
            .map_err(|e| KeyError::Malformed(e.to_string()))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| KeyError::Malformed("public key must be 32 bytes".into()))?;
        let key = VerifyingKey::from_bytes(&arr).map_err(|e| KeyError::Malformed(e.to_string()))?;
        self.add(signer, key);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, signer: &str) -> Option<&VerifyingKey> {
        self.keys.get(signer)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// The backend's self-signing keypair.
pub struct SelfSigner {
    key: SigningKey,
}

impl SelfSigner {
    /// Generate a fresh keypair.
    #[must_use]
    pub fn generate() -> Self {
        Self {
            key: SigningKey::generate(&mut OsRng),
        }
    }

    /// Load the persisted key at `path`, generating and persisting one on first
    /// startup if it does not yet exist.
    ///
    /// # Errors
    /// [`KeyError`] on I/O or malformed stored material.
    pub fn load_or_generate(path: &Path) -> Result<Self, KeyError> {
        if path.exists() {
            let b64 = std::fs::read_to_string(path).map_err(|e| KeyError::Io(e.to_string()))?;
            let bytes = STANDARD
                .decode(b64.trim())
                .map_err(|e| KeyError::Malformed(e.to_string()))?;
            let seed: [u8; 32] = bytes
                .try_into()
                .map_err(|_| KeyError::Malformed("signing seed must be 32 bytes".into()))?;
            Ok(Self {
                key: SigningKey::from_bytes(&seed),
            })
        } else {
            let signer = Self::generate();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| KeyError::Io(e.to_string()))?;
            }
            std::fs::write(path, STANDARD.encode(signer.key.to_bytes()))
                .map_err(|e| KeyError::Io(e.to_string()))?;
            Ok(signer)
        }
    }

    /// The public verifying key.
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }

    /// The public key as base64 (for distributing to trust stores).
    #[must_use]
    pub fn public_key_base64(&self) -> String {
        STANDARD.encode(self.verifying_key().to_bytes())
    }

    /// Sign `msg`, returning the base64 signature.
    #[must_use]
    pub fn sign_base64(&self, msg: &[u8]) -> String {
        STANDARD.encode(self.key.sign(msg).to_bytes())
    }

    /// A trust store seeded with this self key under [`SELF_SIGNER`].
    #[must_use]
    pub fn trust_store(&self) -> TrustStore {
        let mut store = TrustStore::new();
        store.add(SELF_SIGNER, self.verifying_key());
        store
    }
}

/// Verify a module's signatures against the trust store. Returns the id of the
/// trusted signer whose signature validated.
///
/// # Errors
/// [`VerifyError::NoSignatures`] if there are none; [`VerifyError::NoTrustedSignature`]
/// if none from a trusted signer validate over the code artifact.
pub fn verify(
    manifest: &super::manifest::Manifest,
    trust: &TrustStore,
) -> Result<String, VerifyError> {
    if manifest.signatures.is_empty() {
        return Err(VerifyError::NoSignatures);
    }
    let msg = manifest.code_artifact_bytes();
    for sig in &manifest.signatures {
        if !sig.algorithm.eq_ignore_ascii_case("ed25519") {
            continue;
        }
        let Some(vk) = trust.get(&sig.signer) else {
            continue; // untrusted signer
        };
        let Ok(raw) = STANDARD.decode(&sig.value) else {
            continue;
        };
        let Ok(signature) = Signature::from_slice(&raw) else {
            continue;
        };
        if vk.verify(&msg, &signature).is_ok() {
            return Ok(sig.signer.clone());
        }
    }
    Err(VerifyError::NoTrustedSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::manifest::{Endpoint, Manifest, Signature as ManifestSig};
    use serde_json::json;

    fn manifest() -> Manifest {
        Manifest {
            name: "billing".into(),
            version: "1.0.0".into(),
            endpoints: vec![Endpoint {
                method: "GET".into(),
                path: "/invoices".into(),
                permission: None,
            }],
            permissions: vec!["billing:read".into()],
            config_schema: json!({"type":"object"}),
            signatures: vec![],
        }
    }

    fn self_sign(signer: &SelfSigner, m: &mut Manifest, signer_id: &str) {
        let value = signer.sign_base64(&m.code_artifact_bytes());
        m.signatures.push(ManifestSig {
            signer: signer_id.into(),
            algorithm: "ed25519".into(),
            value,
        });
    }

    #[test]
    fn accepts_module_with_one_trusted_signature() {
        let signer = SelfSigner::generate();
        let mut m = manifest();
        self_sign(&signer, &mut m, SELF_SIGNER);
        assert_eq!(
            verify(&m, &signer.trust_store()),
            Ok(SELF_SIGNER.to_string())
        );
    }

    #[test]
    fn rejects_module_with_no_signatures() {
        let signer = SelfSigner::generate();
        assert_eq!(
            verify(&manifest(), &signer.trust_store()),
            Err(VerifyError::NoSignatures)
        );
    }

    #[test]
    fn rejects_untrusted_signer() {
        // Signed by a key that is NOT in the verifier's trust store.
        let foreign = SelfSigner::generate();
        let mut m = manifest();
        self_sign(&foreign, &mut m, "acme-ci");
        let empty = TrustStore::new();
        assert_eq!(verify(&m, &empty), Err(VerifyError::NoTrustedSignature));
    }

    #[test]
    fn changing_code_after_signing_invalidates_signature() {
        let signer = SelfSigner::generate();
        let mut m = manifest();
        self_sign(&signer, &mut m, SELF_SIGNER);
        // Tamper the code (add an endpoint) — the existing signature no longer
        // matches the code artifact.
        m.endpoints.push(Endpoint {
            method: "DELETE".into(),
            path: "/invoices/{id}".into(),
            permission: None,
        });
        assert_eq!(
            verify(&m, &signer.trust_store()),
            Err(VerifyError::NoTrustedSignature)
        );
    }

    #[test]
    fn at_least_one_trusted_signature_among_many_suffices() {
        let trusted = SelfSigner::generate();
        let foreign = SelfSigner::generate();
        let mut m = manifest();
        // First an untrusted signer, then the trusted self key.
        self_sign(&foreign, &mut m, "acme-ci");
        self_sign(&trusted, &mut m, SELF_SIGNER);
        assert_eq!(
            verify(&m, &trusted.trust_store()),
            Ok(SELF_SIGNER.to_string())
        );
    }

    #[test]
    fn load_or_generate_persists_and_reloads_same_key() {
        let dir = std::env::temp_dir().join(format!("superapp-key-{}", uuid::Uuid::new_v4()));
        let path = dir.join("self_signing.key");
        let a = SelfSigner::load_or_generate(&path).unwrap();
        let b = SelfSigner::load_or_generate(&path).unwrap();
        assert_eq!(a.public_key_base64(), b.public_key_base64());
        std::fs::remove_dir_all(&dir).ok();
    }
}
