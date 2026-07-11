//! Module management endpoints (P5): registration, configuration, and health.
//! All are admin-gated (Cedar). Signature verification (TR-05-002) runs at
//! registration; config is validated against the module's JSON schema
//! (TR-05-006).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Extension;
use loco_rs::prelude::*;
use serde::Serialize;
use serde_json::{json, Value};

use crate::auth::extractor::CurrentUser;
use crate::auth::state::AuthState;
use crate::controllers::admin::require_admin;
use crate::models::modules::Model as ModuleModel;
use crate::modules::config_schema;
use crate::modules::manifest::Manifest;
use crate::modules::signing;
use crate::response::{Problem, Success};

const VALIDATION_TYPE: &str = "https://superapp/errors/validation";

#[derive(Debug, Serialize)]
pub struct Registered {
    pub id: String,
    pub name: String,
    pub version: String,
}

/// `POST /api/v1/modules/register` — verify + validate + persist a manifest.
async fn register(
    State(ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    current: CurrentUser,
    body: String,
) -> Result<Success<Registered>, Problem> {
    require_admin(&state, &current).await?;

    let manifest = Manifest::from_json(&body).map_err(|e| {
        Problem::new(StatusCode::BAD_REQUEST).detail(format!("invalid manifest JSON: {e}"))
    })?;

    // Structural validation (TR-05-003): 422 with per-field errors.
    let errors = manifest.validation_errors();
    if !errors.is_empty() {
        return Err(Problem::new(StatusCode::UNPROCESSABLE_ENTITY)
            .with_type(VALIDATION_TYPE)
            .detail("manifest validation failed")
            .with_errors(errors));
    }

    // Signature verification against the trust store (TR-05-002). Rejections
    // are audit-logged and denied.
    if let Err(e) = signing::verify(&manifest, &state.trust) {
        tracing::warn!(
            target: "modules.audit",
            module = %manifest.name,
            version = %manifest.version,
            reason = %e,
            "rejected module registration: signature verification failed",
        );
        return Err(Problem::new(StatusCode::FORBIDDEN)
            .with_type("https://superapp/errors/module-untrusted")
            .detail(format!("signature verification failed: {e}")));
    }

    match ModuleModel::register(&ctx.db, &manifest).await {
        Ok(model) => Ok(Success::new(Registered {
            id: model.pid.to_string(),
            name: model.name,
            version: model.version,
        })
        .message("module registered")),
        Err(ModelError::EntityAlreadyExists {}) => {
            Err(Problem::new(StatusCode::CONFLICT).detail(format!(
                "module {}@{} is already registered",
                manifest.name, manifest.version
            )))
        }
        Err(e) => Err(Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(e.to_string())),
    }
}

/// `PUT /api/v1/modules/{id}/config` — validate config against the module's
/// schema, then persist it.
async fn set_config(
    State(ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    current: CurrentUser,
    Path(id): Path<String>,
    body: String,
) -> Result<Success<Value>, Problem> {
    require_admin(&state, &current).await?;

    let module = ModuleModel::find_by_pid(&ctx.db, &id)
        .await
        .map_err(|_| Problem::new(StatusCode::NOT_FOUND).detail(format!("no such module: {id}")))?;
    let manifest = module
        .parsed_manifest()
        .map_err(|e| Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(e.to_string()))?;

    let config: Value = serde_json::from_str(&body).map_err(|e| {
        Problem::new(StatusCode::BAD_REQUEST).detail(format!("invalid config JSON: {e}"))
    })?;

    let errors = config_schema::validate(&manifest.config_schema, &config);
    if !errors.is_empty() {
        return Err(Problem::new(StatusCode::UNPROCESSABLE_ENTITY)
            .with_type(VALIDATION_TYPE)
            .detail("config validation failed")
            .with_errors(errors));
    }

    module
        .set_config(&ctx.db, &body)
        .await
        .map_err(|e| Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(e.to_string()))?;
    Ok(Success::new(json!({ "applied": true })).message("config updated"))
}

/// `GET /api/v1/modules/{id}/health` — per-module health/status.
async fn health(
    State(ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    current: CurrentUser,
    Path(id): Path<String>,
) -> Result<Success<Value>, Problem> {
    require_admin(&state, &current).await?;

    let module = ModuleModel::find_by_pid(&ctx.db, &id)
        .await
        .map_err(|_| Problem::new(StatusCode::NOT_FOUND).detail(format!("no such module: {id}")))?;

    // If the module is loaded in the runtime, probe it; otherwise report its
    // persisted lifecycle status.
    let health = if state.registry.is_loaded(&module.name) {
        format!("{:?}", state.registry.health(&module.name).await).to_lowercase()
    } else {
        module.status.clone()
    };
    Ok(Success::new(json!({
        "id": module.pid,
        "name": module.name,
        "health": health,
    })))
}

/// Routes mounted under the versioned API base.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/modules")
        .add("/register", post(register))
        .add("/{id}/config", put(set_config))
        .add("/{id}/health", get(health))
}
