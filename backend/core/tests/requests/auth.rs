//! Request-level tests for the P4 auth surface.
//!
//! - TR-04-002: a protected route (`/api/v1/auth/me`) returns `401` without a
//!   valid bearer token and `2xx` with one; loco's native auth endpoints are
//!   no longer exposed.
//! - TR-04-005: the admin routes are Cedar-gated — a regular user is denied
//!   `403`, an admin is allowed.
//! - FR-07-004 support: `/api/v1/auth/capabilities` reports the toggles.

use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue};
use serde_json::{json, Value};
use serial_test::serial;
use superapp_core::{app::App, models::_entities::users};

use crate::support;

/// Insert a user directly (bypassing OIDC) so protected routes can resolve it.
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
async fn capabilities_are_public_and_report_toggles() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/v1/auth/capabilities").await;
        assert_eq!(res.status_code(), 200);
        let body: Value = res.json();
        assert_eq!(body["success"], json!(true));
        // Test profile: toggle off, no OIDC configured.
        assert_eq!(body["data"]["self_registration_enabled"], json!(false));
        assert_eq!(body["data"]["oidc_configured"], json!(false));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn native_loco_auth_endpoints_are_not_exposed() {
    request::<App, _, _>(|request, ctx| async move {
        // loco's scaffolded register endpoint is no longer wired (TR-04-002):
        // the route is unhandled (loco's fallback answers, not our JSON auth
        // controller) and — decisively — it provisions no user.
        let res = request
            .post("/api/auth/register")
            .json(&json!({"email":"native@nope.com","password":"p","name":"n"}))
            .await;
        assert_ne!(
            res.content_type(),
            "application/json",
            "native register must not be served by our JSON auth controller"
        );
        assert!(
            superapp_core::models::users::Model::find_by_email(&ctx.db, "native@nope.com")
                .await
                .is_err(),
            "native registration must not create a user"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn me_returns_401_without_token() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/v1/auth/me").await;
        assert_eq!(res.status_code(), 401);
        assert_eq!(res.content_type(), "application/problem+json");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn me_returns_401_with_garbage_token() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request
            .get("/api/v1/auth/me")
            .add_header("authorization", "Bearer not-a-jwt")
            .await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn me_returns_200_with_valid_token() {
    request::<App, _, _>(|request, ctx| async move {
        seed_user(&ctx.db, "alice@example.com", "user").await;
        let res = request
            .get("/api/v1/auth/me")
            .add_header("authorization", &support::bearer("alice@example.com"))
            .await;
        assert_eq!(res.status_code(), 200);
        let body: Value = res.json();
        assert_eq!(body["data"]["email"], json!("alice@example.com"));
        assert_eq!(body["data"]["role"], json!("user"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn admin_route_denies_regular_user_but_allows_admin() {
    request::<App, _, _>(|request, ctx| async move {
        seed_user(&ctx.db, "bob@example.com", "user").await;
        seed_user(&ctx.db, "boss@example.com", "admin").await;

        // Regular user → Cedar denies with 403.
        let denied = request
            .post("/api/v1/admin/allowlist")
            .add_header("authorization", &support::bearer("bob@example.com"))
            .json(&json!({ "email": "newcomer@example.com" }))
            .await;
        assert_eq!(denied.status_code(), 403);
        assert_eq!(denied.content_type(), "application/problem+json");

        // Admin → allowed (200) and the email is now allow-listed.
        let allowed = request
            .post("/api/v1/admin/allowlist")
            .add_header("authorization", &support::bearer("boss@example.com"))
            .json(&json!({ "email": "newcomer@example.com" }))
            .await;
        assert_eq!(allowed.status_code(), 200);

        let list = request
            .get("/api/v1/admin/allowlist")
            .add_header("authorization", &support::bearer("boss@example.com"))
            .await;
        assert_eq!(list.status_code(), 200);
        let body: Value = list.json();
        let emails: Vec<&str> = body["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["email"].as_str().unwrap())
            .collect();
        assert!(emails.contains(&"newcomer@example.com"));
    })
    .await;
}
