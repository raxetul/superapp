//! Reference module (TR-09-007) — a minimal, **real** module built with the
//! backend SDK (`superapp-module-sdk`, TR-09-001), demonstrating:
//!
//! - registration: [`manifest`] is a valid, signable `Manifest`
//! - a route: `GET /items`, gated by the Cedar permission `reference:read`
//! - schema-validated config: `greeting` is a required string
//! - lifecycle + health: the SDK's `ModuleServer` standard contract
//!   (`/health`, `/ready`, `/sdk`, `/manifest`, `/config`)
//!
//! `backend/core`'s integration tests run this exact router in-process (no
//! Docker) to prove register→load→Cedar-gated-proxy→health end-to-end against
//! real reference-module code (see `tests/requests/reference_module.rs`).

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use superapp_module_sdk::{Manifest, ModuleServer};

pub const NAME: &str = "reference";
pub const VERSION: &str = "1.0.0";
/// Cedar permission gating `GET /items` at the core gateway (TR-05-007).
pub const READ_PERMISSION: &str = "reference:read";

/// The reference module's manifest (TR-09-004/09-007).
#[must_use]
pub fn manifest() -> Manifest {
    Manifest::new(NAME, VERSION)
        .endpoint("GET", "/items", Some(READ_PERMISSION))
        .permission(READ_PERMISSION)
        .config_schema(config_schema())
}

/// JSON Schema for this module's configuration (TR-05-006).
#[must_use]
pub fn config_schema() -> Value {
    json!({
        "type": "object",
        "required": ["greeting"],
        "properties": {
            "greeting": { "type": "string", "minLength": 1 }
        }
    })
}

async fn items() -> Json<Value> {
    Json(json!({
        "items": [{ "id": 1, "label": "hello from the reference module" }]
    }))
}

/// The full HTTP service contract this module exposes: the SDK's standard
/// endpoints plus the manifest-declared `GET /items` business route.
#[must_use = "build the router and pass it to axum::serve"]
pub fn router() -> Router {
    ModuleServer::new(manifest())
        .initial_config(json!({ "greeting": "hello" }))
        .merge(Router::new().route("/items", get(items)))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_valid_and_carries_the_read_permission() {
        let m = manifest();
        assert!(m.is_valid());
        assert!(m.permissions.contains(&READ_PERMISSION.to_string()));
        assert_eq!(m.endpoints[0].permission.as_deref(), Some(READ_PERMISSION));
    }

    #[test]
    fn config_schema_requires_a_greeting() {
        let errs = superapp_module_sdk::config_schema::validate(&config_schema(), &json!({}));
        assert!(errs.iter().any(|e| e.pointer.contains("greeting")));
    }

    #[tokio::test]
    async fn router_serves_health_sdk_and_items() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router()).await.unwrap();
        });
        let base = format!("http://{addr}");

        let health = reqwest::get(format!("{base}/health")).await.unwrap();
        assert_eq!(health.status(), 200);

        let sdk: Value = reqwest::get(format!("{base}/sdk"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(sdk["sdkVersion"], json!(superapp_module_sdk::SDK_VERSION));

        let items: Value = reqwest::get(format!("{base}/items"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(items["items"][0]["id"], json!(1));
    }
}
