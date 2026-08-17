//! Request-level tests for the live core gateway route (TR-09-001/007).
//!
//! No Docker daemon is available in this environment, so no module is ever
//! `load`ed into the booted app's `AuthState::registry` here — proving that
//! is `modules::registry`'s job (hermetic, in-process runtime) and
//! `reference_module.rs` (real reference-module code). This test proves the
//! route itself is wired, authenticated, and reports `404` for an unloaded
//! module rather than panicking or 500ing.

use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue};
use serial_test::serial;
use superapp_core::app::App;
use superapp_core::models::_entities::users;

async fn seed_user(db: &sea_orm::DatabaseConnection, email: &str) {
    users::ActiveModel {
        email: ActiveValue::set(email.to_string()),
        name: ActiveValue::set(email.split('@').next().unwrap().to_string()),
        password: ActiveValue::set("!oidc!".to_string()),
        role: ActiveValue::set("user".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

#[tokio::test]
#[serial]
async fn unloaded_module_route_is_a_clean_404() {
    request::<App, _, _>(|request, ctx| async move {
        seed_user(&ctx.db, "bob@example.com").await;
        let res = request
            .get("/api/v1/gateway/reference/items")
            .add_header("authorization", &crate::support::bearer("bob@example.com"))
            .await;
        assert_eq!(res.status_code(), 404);
        assert_eq!(res.content_type(), "application/problem+json");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn gateway_route_requires_authentication() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/v1/gateway/reference/items").await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}
