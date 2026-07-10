//! API-key model integration tests (TR-04-009): mint → authenticate → revoke.

use loco_rs::testing::prelude::*;
use serial_test::serial;
use superapp_core::{app::App, models::api_keys::Model as ApiKey};

#[tokio::test]
#[serial]
async fn mint_authenticate_then_revoke() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    // Mint a key; the plaintext is returned once.
    let (key, plaintext) = ApiKey::create(db, "sample-module").await.unwrap();
    assert!(plaintext.starts_with("sk_"));
    assert!(key.is_active());

    // A valid key authenticates.
    let authed = ApiKey::authenticate(db, &plaintext).await.unwrap();
    assert_eq!(authed.id, key.id);
    assert_eq!(authed.name, "sample-module");

    // An unknown/garbage key is rejected.
    assert!(ApiKey::authenticate(db, "sk_nope_nope").await.is_err());
    assert!(ApiKey::authenticate(db, "not-even-a-key").await.is_err());

    // Revoke, then the same key no longer authenticates (keys are revocable).
    ApiKey::revoke(db, &key.prefix).await.unwrap();
    assert!(
        ApiKey::authenticate(db, &plaintext).await.is_err(),
        "revoked key must not authenticate"
    );
}
