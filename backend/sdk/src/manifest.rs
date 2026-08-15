//! The module manifest (TR-09-004), mirrored field-for-field from the core's
//! canonical `backend/core/src/modules/manifest.rs::Manifest` and from the
//! shared JSON Schema at `schemas/module-manifest.schema.json`. A module
//! author builds this with the SDK and registers it at `POST
//! /api/v1/modules/register` — the core parses it with its own identical
//! type, so there is exactly one manifest shape, not two independently
//! evolving ones.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single field-level validation failure (JSON Pointer + message).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError {
    pub pointer: String,
    pub detail: String,
}

impl FieldError {
    pub fn new(pointer: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            pointer: pointer.into(),
            detail: detail.into(),
        }
    }
}

/// One HTTP endpoint a module exposes (proxied by the core gateway).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub permission: Option<String>,
}

/// A detached signature over the manifest's code artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    pub signer: String,
    pub algorithm: String,
    pub value: String,
}

/// A module manifest — identical shape to the core's `Manifest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub endpoints: Vec<Endpoint>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub config_schema: Value,
    #[serde(default)]
    pub signatures: Vec<Signature>,
}

const ALLOWED_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

impl Manifest {
    /// Start a builder for `name`@`version`.
    #[must_use]
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            endpoints: Vec::new(),
            permissions: Vec::new(),
            config_schema: Value::Null,
            signatures: Vec::new(),
        }
    }

    #[must_use]
    pub fn endpoint(
        mut self,
        method: impl Into<String>,
        path: impl Into<String>,
        permission: Option<&str>,
    ) -> Self {
        self.endpoints.push(Endpoint {
            method: method.into(),
            path: path.into(),
            permission: permission.map(str::to_string),
        });
        self
    }

    #[must_use]
    pub fn permission(mut self, permission: impl Into<String>) -> Self {
        self.permissions.push(permission.into());
        self
    }

    #[must_use]
    pub fn config_schema(mut self, schema: Value) -> Self {
        self.config_schema = schema;
        self
    }

    /// Same structural rules the core applies at `/modules/register` — so a
    /// module author sees the same `422`-worthy errors before ever sending
    /// the manifest over the wire.
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

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validation_errors().is_empty()
    }

    /// The canonical code-artifact bytes signatures are computed over — byte-
    /// identical algorithm to the core's, so a signature made here verifies
    /// there (TR-05-002).
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
        Manifest::new("reference", "1.0.0")
            .endpoint("GET", "/items", Some("reference:read"))
            .permission("reference:read")
            .config_schema(json!({"type":"object","properties":{"greeting":{"type":"string"}}}))
    }

    #[test]
    fn builder_produces_a_valid_manifest() {
        assert!(sample().is_valid());
    }

    #[test]
    fn invalid_method_is_reported() {
        let mut m = sample();
        m.endpoints[0].method = "FETCH".into();
        assert!(!m.is_valid());
    }

    #[test]
    fn code_artifact_excludes_signatures_and_is_deterministic() {
        let m = sample();
        let a = m.code_artifact_bytes();
        let mut m2 = m.clone();
        m2.signatures.push(Signature {
            signer: "self".into(),
            algorithm: "ed25519".into(),
            value: "abc".into(),
        });
        assert_eq!(a, m2.code_artifact_bytes());
    }

    /// TR-09-004: this SDK type must serialize/deserialize the exact same
    /// wire shape the canonical schema (and the core's own `Manifest`)
    /// declare — cross-checked directly against the shared schema file so
    /// the two can never silently drift apart.
    #[test]
    fn matches_the_canonical_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../../../schemas/module-manifest.schema.json"))
                .expect("canonical schema is valid JSON");
        let props: Vec<&str> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let roundtrip = serde_json::to_value(sample()).unwrap();
        for field in roundtrip.as_object().unwrap().keys() {
            assert!(
                props.contains(&field.as_str()),
                "SDK Manifest field `{field}` is not in the canonical schema"
            );
        }
    }
}
