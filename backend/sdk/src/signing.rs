//! Module-author signing helper (TR-05-002, TR-09-006).
//!
//! A module author generates (or loads) an ed25519 keypair and signs their
//! manifest's [`code_artifact_bytes`](crate::manifest::Manifest::code_artifact_bytes)
//! — the exact bytes the core's `signing::verify` checks — then distributes
//! their public key to the core operator to add to `modules.trusted_signers`
//! (TR-05-009).

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

use crate::manifest::{Manifest, Signature};

/// A module author's signing keypair.
pub struct ModuleSigner {
    key: SigningKey,
}

impl ModuleSigner {
    /// Generate a fresh keypair.
    #[must_use]
    pub fn generate() -> Self {
        Self {
            key: SigningKey::generate(&mut OsRng),
        }
    }

    /// Load a keypair from a base64-encoded 32-byte seed (e.g. from a CI
    /// secret).
    ///
    /// # Errors
    /// If `seed_b64` is not valid base64 or not exactly 32 bytes.
    pub fn from_seed_base64(seed_b64: &str) -> Result<Self, String> {
        let bytes = STANDARD.decode(seed_b64).map_err(|e| e.to_string())?;
        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "signing seed must be 32 bytes".to_string())?;
        Ok(Self {
            key: SigningKey::from_bytes(&seed),
        })
    }

    /// The public key, base64-encoded — hand this to the core operator to add
    /// under `modules.trusted_signers`.
    #[must_use]
    pub fn public_key_base64(&self) -> String {
        STANDARD.encode(self.key.verifying_key().to_bytes())
    }

    /// Sign `manifest`'s code artifact under `signer_id`, appending the
    /// signature to `manifest.signatures` and returning the mutated manifest.
    #[must_use]
    pub fn sign(&self, mut manifest: Manifest, signer_id: impl Into<String>) -> Manifest {
        let sig = self.key.sign(&manifest.code_artifact_bytes());
        manifest.signatures.push(Signature {
            signer: signer_id.into(),
            algorithm: "ed25519".into(),
            value: STANDARD.encode(sig.to_bytes()),
        });
        manifest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_and_round_trips_a_public_key() {
        let signer = ModuleSigner::generate();
        let m = Manifest::new("reference", "1.0.0");
        let signed = signer.sign(m, "reference-ci");
        assert_eq!(signed.signatures.len(), 1);
        assert_eq!(signed.signatures[0].signer, "reference-ci");
        assert_eq!(signed.signatures[0].algorithm, "ed25519");
        assert!(!signer.public_key_base64().is_empty());
    }

    #[test]
    fn signing_does_not_change_the_code_artifact() {
        let signer = ModuleSigner::generate();
        let m = Manifest::new("reference", "1.0.0");
        let before = m.code_artifact_bytes();
        let signed = signer.sign(m, "reference-ci");
        assert_eq!(before, signed.code_artifact_bytes());
    }
}
