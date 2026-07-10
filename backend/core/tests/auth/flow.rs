//! End-to-end login/refresh flow tests (TR-04-001, TR-04-003, TR-04-004),
//! driven with in-memory fakes for the network seams plus the isolated test DB.

use loco_rs::testing::prelude::*;
use serial_test::serial;
use superapp_core::{
    app::App,
    auth::{
        oidc::{FakeOidcProvider, OidcTokens},
        refresh::InMemoryRefreshStore,
        service::{complete_login, refresh_session},
        token::TokenValidator,
    },
};

use crate::support;

fn validator() -> TokenValidator {
    TokenValidator::from_jwks_json(support::JWKS_JSON, support::ISSUER, support::AUDIENCE).unwrap()
}

/// Login completes: code → validated token → first user provisioned as admin →
/// refresh session opened. Then refresh rotates the handle and invalidates the
/// old one (reuse is rejected).
#[tokio::test]
#[serial]
async fn login_then_refresh_rotates_and_invalidates_old_handle() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    let email = "alice@example.com";
    let oidc = FakeOidcProvider::with_code(
        "auth-code",
        OidcTokens {
            access_token: support::access_token(email),
            refresh_token: Some("rauthy-rt-1".into()),
            expires_in_secs: Some(900),
        },
    )
    .and_refresh(
        "rauthy-rt-1",
        OidcTokens {
            access_token: support::access_token(email),
            refresh_token: Some("rauthy-rt-2".into()),
            expires_in_secs: Some(900),
        },
    );
    let refresh = InMemoryRefreshStore::default();
    let validator = validator();

    // Complete login (self-registration off; first user → admin bootstrap).
    let session = complete_login(
        &oidc,
        &validator,
        &refresh,
        db,
        false,
        "auth-code",
        "verifier",
    )
    .await
    .expect("login completes");
    assert_eq!(session.email, email);
    assert_eq!(session.role, "admin"); // first user bootstrap (TR-04-004)
    let handle1 = session.refresh_handle.clone();
    assert!(!handle1.is_empty());

    // Refresh rotates to a new handle.
    let refreshed = refresh_session(&oidc, &validator, &refresh, db, &handle1)
        .await
        .expect("refresh succeeds");
    assert_ne!(refreshed.refresh_handle, handle1);
    assert_eq!(refreshed.email, email);

    // The old handle can no longer be used (rotation consumed it).
    let reused = refresh_session(&oidc, &validator, &refresh, db, &handle1).await;
    assert!(reused.is_err(), "reused refresh handle must be rejected");
}

/// A second, unknown identity is denied onboarding when the toggle is off and
/// the email is not allow-listed (TR-04-013) — surfaced as a login error.
#[tokio::test]
#[serial]
async fn login_denied_for_unknown_identity_when_toggle_off() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    // Seed a bootstrap admin so the next identity is not the "first" user.
    superapp_core::auth::provisioning::provision(
        db,
        &superapp_core::auth::provisioning::ProvisionInput {
            email: "admin@example.com".into(),
            name: None,
            self_registration_enabled: false,
        },
    )
    .await
    .unwrap();

    let stranger = "stranger@example.com";
    let oidc = FakeOidcProvider::with_code(
        "code-2",
        OidcTokens {
            access_token: support::access_token(stranger),
            refresh_token: Some("rt".into()),
            expires_in_secs: Some(900),
        },
    );
    let refresh = InMemoryRefreshStore::default();

    let result = complete_login(&oidc, &validator(), &refresh, db, false, "code-2", "v").await;
    assert!(
        result.is_err(),
        "unknown identity must be denied onboarding"
    );
}
