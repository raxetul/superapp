//! [`ModuleServer`] — the Rust implementation of the module **service
//! contract** (TR-09-001): the fixed set of HTTP endpoints every module
//! container exposes to the core, regardless of what business routes it also
//! serves.
//!
//! | Route | Purpose |
//! |---|---|
//! | `GET /health` | liveness/readiness probe the core polls before marking the module ready and serving its routes (TR-05-004/005) |
//! | `GET /ready` | alias of `/health`, for readiness-probe-only deployments |
//! | `GET /sdk` | `{ "sdkVersion": "…" }` — checked by the core at load (TR-09-005) |
//! | `GET /manifest` | the module's own manifest, for operator inspection |
//! | `GET /config` / `PUT /config` | current config / schema-validated config update (TR-05-006) |
//!
//! Business routes (the manifest's declared `endpoints`) are supplied via
//! [`ModuleServer::merge`] and are proxied by the core gateway, not called
//! directly by the SDK.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::Value;

use crate::config_schema;
use crate::manifest::Manifest;
use crate::version::SDK_VERSION;

#[derive(Clone)]
struct ContractState {
    manifest: Arc<Manifest>,
    config: Arc<Mutex<Value>>,
}

/// Builds the full HTTP service contract for a module.
pub struct ModuleServer {
    manifest: Manifest,
    initial_config: Value,
    business_routes: Router,
}

impl ModuleServer {
    #[must_use]
    pub fn new(manifest: Manifest) -> Self {
        Self {
            manifest,
            initial_config: Value::Null,
            business_routes: Router::new(),
        }
    }

    /// Seed the module's starting configuration (normally validated against
    /// `manifest.config_schema` by the caller before this).
    #[must_use]
    pub fn initial_config(mut self, config: Value) -> Self {
        self.initial_config = config;
        self
    }

    /// Mount the module's own business routes (the manifest's declared
    /// `endpoints`) alongside the standard contract endpoints.
    #[must_use]
    pub fn merge(mut self, routes: Router) -> Self {
        self.business_routes = self.business_routes.merge(routes);
        self
    }

    /// Build the final [`axum::Router`] to serve.
    #[must_use = "build the router and pass it to axum::serve"]
    pub fn build(self) -> Router {
        let state = ContractState {
            manifest: Arc::new(self.manifest),
            config: Arc::new(Mutex::new(self.initial_config)),
        };
        Router::new()
            .route("/health", get(health))
            .route("/ready", get(health))
            .route("/sdk", get(sdk_info))
            .route("/manifest", get(get_manifest))
            .route("/config", get(get_config).put(put_config))
            .with_state(state)
            .merge(self.business_routes)
    }
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn sdk_info() -> impl IntoResponse {
    Json(serde_json::json!({ "sdkVersion": SDK_VERSION }))
}

async fn get_manifest(State(state): State<ContractState>) -> impl IntoResponse {
    Json((*state.manifest).clone())
}

async fn get_config(State(state): State<ContractState>) -> impl IntoResponse {
    Json(state.config.lock().unwrap().clone())
}

async fn put_config(
    State(state): State<ContractState>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    let errors = config_schema::validate(&state.manifest.config_schema, &body);
    if !errors.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "errors": errors })),
        )
            .into_response();
    }
    *state.config.lock().unwrap() = body;
    StatusCode::OK.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get as axum_get;
    use serde_json::json;
    use tokio::net::TcpListener;

    async fn serve(server: ModuleServer) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, server.build()).await.unwrap();
        });
        format!("http://{addr}")
    }

    fn manifest() -> Manifest {
        Manifest::new("reference", "1.0.0").config_schema(json!({
            "type": "object",
            "required": ["greeting"],
            "properties": { "greeting": { "type": "string" } }
        }))
    }

    #[tokio::test]
    async fn health_and_ready_report_ok() {
        let base = serve(ModuleServer::new(manifest())).await;
        let client = reqwest::Client::new();
        assert_eq!(
            client
                .get(format!("{base}/health"))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        assert_eq!(
            client
                .get(format!("{base}/ready"))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
    }

    #[tokio::test]
    async fn sdk_endpoint_reports_the_sdk_version() {
        let base = serve(ModuleServer::new(manifest())).await;
        let body: Value = reqwest::get(format!("{base}/sdk"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(body["sdkVersion"], json!(SDK_VERSION));
    }

    #[tokio::test]
    async fn manifest_endpoint_returns_the_registered_manifest() {
        let base = serve(ModuleServer::new(manifest())).await;
        let body: Value = reqwest::get(format!("{base}/manifest"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(body["name"], json!("reference"));
    }

    #[tokio::test]
    async fn config_is_validated_against_the_manifest_schema() {
        let base =
            serve(ModuleServer::new(manifest()).initial_config(json!({"greeting":"hi"}))).await;
        let client = reqwest::Client::new();

        let ok = client
            .get(format!("{base}/config"))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();
        assert_eq!(ok["greeting"], json!("hi"));

        let rejected = client
            .put(format!("{base}/config"))
            .json(&json!({"nope": 1}))
            .send()
            .await
            .unwrap();
        assert_eq!(rejected.status(), 422);

        let accepted = client
            .put(format!("{base}/config"))
            .json(&json!({"greeting": "hello"}))
            .send()
            .await
            .unwrap();
        assert_eq!(accepted.status(), 200);
    }

    #[tokio::test]
    async fn business_routes_are_served_alongside_the_contract() {
        let business = Router::new().route("/items", axum_get(|| async { "items" }));
        let base = serve(ModuleServer::new(manifest()).merge(business)).await;
        let res = reqwest::get(format!("{base}/items")).await.unwrap();
        assert_eq!(res.status(), 200);
        assert_eq!(res.text().await.unwrap(), "items");
    }
}
