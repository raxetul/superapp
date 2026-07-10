//! Provisioning / bootstrap / toggle / allow-list integration tests
//! (TR-04-004, TR-04-011, TR-04-012, TR-04-013) against the isolated test DB.

use loco_rs::testing::prelude::*;
use serial_test::serial;
use superapp_core::{
    app::App,
    auth::provisioning::{provision, ProvisionError, ProvisionInput},
    models::{allowlisted_emails::Model as AllowlistedEmail, role::Role},
};

fn input(email: &str, self_reg: bool) -> ProvisionInput {
    ProvisionInput {
        email: email.to_string(),
        name: Some("Test User".to_string()),
        self_registration_enabled: self_reg,
    }
}

/// TR-04-004: the first authenticated user becomes admin — even with the
/// toggle off — and provisioning is idempotent by email.
#[tokio::test]
#[serial]
async fn first_user_is_admin_and_provisioning_is_idempotent() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    // Toggle OFF, email NOT allow-listed — bootstrap still makes an admin.
    let first = provision(db, &input("boss@corp.example", false))
        .await
        .expect("first user provisioned");
    assert_eq!(first.role(), Role::Admin);
    assert_eq!(first.email, "boss@corp.example");

    // Repeat login for the same email returns the same record (no duplicate).
    let again = provision(db, &input("BOSS@corp.example", false))
        .await
        .expect("idempotent");
    assert_eq!(again.id, first.id);
    assert_eq!(again.role(), Role::Admin);
}

/// TR-04-013: with the toggle off, a valid identity whose email is not
/// allow-listed is rejected and no account is created; once allow-listed it
/// onboards at least privilege.
#[tokio::test]
#[serial]
async fn toggle_off_denies_then_allowlist_permits_least_privilege() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    // Seed a bootstrap admin so the next user is no longer "first".
    provision(db, &input("admin@corp.example", false))
        .await
        .unwrap();

    // Not allow-listed, toggle off → denied, no account.
    let denied = provision(db, &input("stranger@corp.example", false)).await;
    assert!(matches!(denied, Err(ProvisionError::NotAllowed(_))));
    assert!(
        superapp_core::models::users::Model::find_by_email(db, "stranger@corp.example")
            .await
            .is_err(),
        "denied identity must not be provisioned"
    );

    // Admin allow-lists the email → it can now onboard at least privilege.
    AllowlistedEmail::add(db, "stranger@corp.example", Some("admin@corp.example"))
        .await
        .unwrap();
    let user = provision(db, &input("stranger@corp.example", false))
        .await
        .expect("allow-listed onboards");
    assert_eq!(user.role(), Role::User);
}

/// TR-04-011 / TR-04-012: with self-registration enabled, a previously-unknown
/// user is auto-provisioned at the least-privilege role (never admin).
#[tokio::test]
#[serial]
async fn self_registration_enabled_creates_least_privilege() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    // Bootstrap admin first.
    let admin = provision(db, &input("admin@corp.example", true))
        .await
        .unwrap();
    assert_eq!(admin.role(), Role::Admin);

    // Toggle ON → unknown user self-onboards at least privilege.
    let user = provision(db, &input("newbie@corp.example", true))
        .await
        .expect("self-onboarded");
    assert_eq!(user.role(), Role::User);
    assert!(!user.role().is_admin());
}
