//! Request-level tests for the module management surface (P5).
//!
//! - TR-05-003: `POST /modules/register` returns an id + persists; duplicate
//!   `name+version` rejected; invalid manifest → `422`.
//! - TR-05-002: a manifest whose signatures are untrusted/invalid is rejected.
//! - TR-05-006: `PUT /modules/{id}/config` validates against the schema.
//! - Admin-gated (Cedar): a non-admin is denied.

use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue};
use serde_json::{json, Value};
use serial_test::serial;
use superapp_core::{app::App, models::_entities::users, modules::manifest::Manifest};

use crate::support;

async fn seed_user(db: &sea_orm::DatabaseConnection, email: &str, role: &str) {
    users::ActiveModel {
        email: ActiveValue::set(email.to_string()),
        name: ActiveValue::set(email.split('@').next().unwrap().to_string()),
        password: ActiveValue::set("!oidc!".to_string()),
        role: ActiveValue::set(role.to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

#[tokio::test]
#[serial]
async fn register_signed_manifest_returns_id_and_rejects_duplicate() {
    request::<App, _, _>(|request, ctx| async move {
        seed_user(&ctx.db, "boss@example.com", "admin").await;
        let manifest = support::signed_manifest_json("billing", "1.0.0");

        let res = request
            .post("/api/v1/modules/register")
            .add_header("authorization", &support::bearer("boss@example.com"))
            .text(&manifest)
            .await;
        assert_eq!(res.status_code(), 200, "body: {}", res.text());
        let body: Value = res.json();
        assert_eq!(body["data"]["name"], json!("billing"));
        assert!(body["data"]["id"].as_str().is_some());

        // Duplicate name+version → 409.
        let dup = request
            .post("/api/v1/modules/register")
            .add_header("authorization", &support::bearer("boss@example.com"))
            .text(&manifest)
            .await;
        assert_eq!(dup.status_code(), 409);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn register_rejects_invalid_manifest_with_422() {
    request::<App, _, _>(|request, ctx| async move {
        seed_user(&ctx.db, "boss@example.com", "admin").await;
        let mut m = support::sample_manifest("bad", "1.0.0");
        m.endpoints[0].method = "FETCH".into(); // unsupported method
        support::sign_manifest(&mut m);
        let res = request
            .post("/api/v1/modules/register")
            .add_header("authorization", &support::bearer("boss@example.com"))
            .text(serde_json::to_string(&m).unwrap())
            .await;
        assert_eq!(res.status_code(), 422);
        assert_eq!(res.content_type(), "application/problem+json");
        let body: Value = res.json();
        assert!(body["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["pointer"].as_str().unwrap().contains("method")));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn register_rejects_untrusted_signature_with_403() {
    request::<App, _, _>(|request, ctx| async move {
        seed_user(&ctx.db, "boss@example.com", "admin").await;
        // Validly sign, then tamper the code so the signature no longer matches.
        let mut m: Manifest =
            serde_json::from_str(&support::signed_manifest_json("evil", "1.0.0")).unwrap();
        m.endpoints
            .push(superapp_core::modules::manifest::Endpoint {
                method: "DELETE".into(),
                path: "/items/{id}".into(),
                permission: None,
            });
        let res = request
            .post("/api/v1/modules/register")
            .add_header("authorization", &support::bearer("boss@example.com"))
            .text(serde_json::to_string(&m).unwrap())
            .await;
        assert_eq!(res.status_code(), 403);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn register_requires_admin() {
    request::<App, _, _>(|request, ctx| async move {
        seed_user(&ctx.db, "bob@example.com", "user").await;
        let res = request
            .post("/api/v1/modules/register")
            .add_header("authorization", &support::bearer("bob@example.com"))
            .text(support::signed_manifest_json("billing", "1.0.0"))
            .await;
        assert_eq!(res.status_code(), 403);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn config_validated_against_schema() {
    request::<App, _, _>(|request, ctx| async move {
        seed_user(&ctx.db, "boss@example.com", "admin").await;
        let reg = request
            .post("/api/v1/modules/register")
            .add_header("authorization", &support::bearer("boss@example.com"))
            .text(support::signed_manifest_json("billing", "1.0.0"))
            .await;
        let id = reg.json::<Value>()["data"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Valid config (has required `currency`) → 200.
        let ok = request
            .put(&format!("/api/v1/modules/{id}/config"))
            .add_header("authorization", &support::bearer("boss@example.com"))
            .text(json!({ "currency": "EUR" }).to_string())
            .await;
        assert_eq!(ok.status_code(), 200, "body: {}", ok.text());

        // Invalid config (missing required `currency`) → 422.
        let bad = request
            .put(&format!("/api/v1/modules/{id}/config"))
            .add_header("authorization", &support::bearer("boss@example.com"))
            .text(json!({ "nope": 1 }).to_string())
            .await;
        assert_eq!(bad.status_code(), 422);
    })
    .await;
}
