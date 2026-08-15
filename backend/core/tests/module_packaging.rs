//! TR-09-006: the Node.js packaging/signing script produces a signed
//! manifest the **real** loader (`signing::verify`) accepts.
//!
//! This spawns `scripts/module-sdk/package-module.mjs` as a subprocess (the
//! same tool a module author runs), rather than reimplementing signing in
//! Rust — proving the actual shipped tool's output is accepted, not a
//! reimplementation of it. Skips itself (rather than failing) if `node` is
//! unavailable, matching the project's honesty rule about environment
//! constraints.

use std::path::PathBuf;
use std::process::Command;

use superapp_core::modules::manifest::Manifest;
use superapp_core::modules::signing::{self, TrustStore};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root resolves")
}

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn packaging_script_output_is_accepted_by_the_real_loader() {
    if !node_available() {
        eprintln!("skipping: node is not available in this environment");
        return;
    }

    let root = repo_root();
    let script = root.join("scripts/module-sdk/package-module.mjs");
    let dir = std::env::temp_dir().join(format!("superapp-packaging-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    // 1. Generate a signing keypair with the real CLI tool.
    let key_path = dir.join("signer.pem");
    let keygen = Command::new("node")
        .arg(&script)
        .arg("generate-key")
        .arg("--out")
        .arg(&key_path)
        .output()
        .expect("run generate-key");
    assert!(
        keygen.status.success(),
        "generate-key failed: {}",
        String::from_utf8_lossy(&keygen.stderr)
    );
    let stdout = String::from_utf8_lossy(&keygen.stdout);
    let public_key_b64 = stdout
        .lines()
        .last()
        .expect("generate-key prints the public key on its last line")
        .trim()
        .to_string();

    // 2. Write a manifest fixture and sign it with the real CLI tool.
    let manifest_path = dir.join("manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::json!({
            "name": "packaged",
            "version": "1.0.0",
            "endpoints": [{"method": "GET", "path": "/items", "permission": "packaged:read"}],
            "permissions": ["packaged:read"],
            "config_schema": {"type": "object"}
        })
        .to_string(),
    )
    .unwrap();
    let signed_path = dir.join("signed.json");
    let sign_out = Command::new("node")
        .arg(&script)
        .arg("sign")
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--key")
        .arg(&key_path)
        .arg("--signer")
        .arg("packaging-ci")
        .arg("--out")
        .arg(&signed_path)
        .output()
        .expect("run sign");
    assert!(
        sign_out.status.success(),
        "sign failed: {}",
        String::from_utf8_lossy(&sign_out.stderr)
    );

    // 3. The real loader accepts it once the printed public key is trusted.
    let signed_json = std::fs::read_to_string(&signed_path).unwrap();
    let manifest = Manifest::from_json(&signed_json).expect("signed manifest parses");
    assert!(manifest.is_valid());

    let mut trust = TrustStore::new();
    trust
        .add_base64("packaging-ci", &public_key_b64)
        .expect("the CLI's printed public key is valid base64");
    assert_eq!(
        signing::verify(&manifest, &trust),
        Ok("packaging-ci".to_string())
    );

    std::fs::remove_dir_all(&dir).ok();
}
