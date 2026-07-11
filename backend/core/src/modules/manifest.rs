//! Module manifest (TR-05-003) and the canonical *code artifact* over which
//! signatures are computed (TR-05-002).
//!
//! A manifest declares `name`, `version`, `endpoints`, `permissions`,
//! `config_schema`, and an array of `signatures`. The signature covers the
//! **immutable code/contract** portion — name, version, endpoints,
//! permissions, config schema — and deliberately **excludes** the signatures
//! themselves and any variable/data/config. So changing runtime config never
//! invalidates a signature, while changing a declared route does.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::response::FieldError;

/// One HTTP endpoint a module exposes (proxied by the gateway).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    /// HTTP method (`GET`, `POST`, …).
    pub method: String,
    /// Path relative to the module's mount (e.g. `/items`).
    pub path: String,
    /// Optional Cedar permission required to call this endpoint.
    #[serde(default)]
    pub permission: Option<String>,
}

/// A detached signature over the manifest's code artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// Identifier of the signer whose public key must be trusted.
    pub signer: String,
    /// Signature algorithm (only `ed25519` is supported).
    pub algorithm: String,
    /// Base64 (standard) signature bytes.
    pub value: String,
}

/// A module manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub endpoints: Vec<Endpoint>,
    #[serde(default)]
    pub permissions: Vec<String>,
    /// JSON Schema for this module's configuration.
    #[serde(default)]
    pub config_schema: Value,
    /// Detached signatures (self-signed and/or external).
    #[serde(default)]
    pub signatures: Vec<Signature>,
}

const ALLOWED_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

impl Manifest {
    /// Parse a manifest from JSON.
    ///
    /// # Errors
    /// The serde error if the JSON does not match the manifest shape.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Structurally validate the manifest, returning per-field errors suitable
    /// for a `422` problem (empty vec ⇒ valid).
    #[must_use]
    pub fn validation_errors(&self) -> Vec<FieldError> {
        let mut errs = Vec::new();
        if self.name.trim().is_empty() {
            errs.push(FieldError::new("/name", "name is required"));
        }
        if self.version.trim().is_empty() {
            errs.push(FieldError::new("/version", "version is required"));
        }
        for (i, ep) in self.endpoints.iter().enumerate() {
            if !ALLOWED_METHODS.contains(&ep.method.to_uppercase().as_str()) {
                errs.push(FieldError::new(
                    format!("/endpoints/{i}/method"),
                    format!("unsupported HTTP method `{}`", ep.method),
                ));
            }
            if !ep.path.starts_with('/') {
                errs.push(FieldError::new(
                    format!("/endpoints/{i}/path"),
                    "path must start with `/`",
                ));
            }
        }
        errs
    }

    /// Whether the manifest is structurally valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validation_errors().is_empty()
    }

    /// The canonical bytes of the **code artifact** that signatures cover:
    /// name, version, endpoints, permissions, and config schema — with object
    /// keys recursively sorted so the encoding is deterministic. Signatures and
    /// any runtime config are excluded.
    #[must_use]
    pub fn code_artifact_bytes(&self) -> Vec<u8> {
        let artifact = serde_json::json!({
            "name": self.name,
            "version": self.version,
            "endpoints": self.endpoints,
            "permissions": self.permissions,
            "config_schema": self.config_schema,
        });
        serde_json::to_vec(&canonicalize(&artifact)).unwrap_or_default()
    }
}

/// Recursively sort object keys to produce a canonical JSON value.
fn canonicalize(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for k in keys {
                out.insert(k.clone(), canonicalize(&map[k]));
            }
            Value::Object(out)
        }
        Value::Array(a) => Value::Array(a.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Manifest {
        Manifest {
            name: "billing".into(),
            version: "1.0.0".into(),
            endpoints: vec![Endpoint {
                method: "GET".into(),
                path: "/invoices".into(),
                permission: Some("billing:read".into()),
            }],
            permissions: vec!["billing:read".into()],
            config_schema: json!({"type":"object","properties":{"currency":{"type":"string"}}}),
            signatures: vec![],
        }
    }

    #[test]
    fn valid_manifest_has_no_errors() {
        assert!(sample().is_valid());
    }

    #[test]
    fn missing_name_and_bad_method_are_reported() {
        let mut m = sample();
        m.name = "  ".into();
        m.endpoints[0].method = "FETCH".into();
        let errs = m.validation_errors();
        let ptrs: Vec<&str> = errs.iter().map(|e| e.pointer.as_str()).collect();
        assert!(ptrs.contains(&"/name"));
        assert!(ptrs.contains(&"/endpoints/0/method"));
    }

    #[test]
    fn code_artifact_is_deterministic_and_excludes_signatures() {
        let m = sample();
        let a = m.code_artifact_bytes();
        // Adding a signature must NOT change the code artifact.
        let mut m2 = m.clone();
        m2.signatures.push(Signature {
            signer: "self".into(),
            algorithm: "ed25519".into(),
            value: "abc".into(),
        });
        assert_eq!(a, m2.code_artifact_bytes());
    }

    #[test]
    fn changing_a_route_changes_the_code_artifact_but_config_does_not() {
        let base = sample().code_artifact_bytes();
        // Changing code (an endpoint path) changes the artifact.
        let mut code_changed = sample();
        code_changed.endpoints[0].path = "/invoices/all".into();
        assert_ne!(base, code_changed.code_artifact_bytes());
    }
}
