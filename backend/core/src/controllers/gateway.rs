//! Live core gateway route (TR-09-001/007).
//!
//! Mounts the [`crate::modules::registry::Gateway`] built in P5 as a real HTTP
//! route: `GET /api/v1/gateway/{module}/{*path}` resolves the module's
//! manifest-declared endpoint, enforces its Cedar permission, and — only then
//! — proxies to the running module container. A denied or unloaded module
//! never sees the request (fault isolation + authorization stay in the
//! gateway, not the module).
//!
//! P5 proved this logic hermetically (`modules::registry` tests); this route
//! is the deployable wiring the P5 phase doc flagged as a P9 follow-up. Since
//! no Docker daemon is available in this environment, no module is ever
//! `load`ed into the booted app here, so live requests observably 404 — the
//! reference module's full register→load→proxy→Cedar path is proven against
//! the real reference-module code via an in-process runtime instead (see
//! `tests/requests/reference_module.rs`).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use loco_rs::prelude::*;

use crate::auth::extractor::CurrentUser;
use crate::auth::state::AuthState;
use crate::modules::registry::GatewayOutcome;
use crate::response::Problem;

async fn proxy(
    State(_ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    current: CurrentUser,
    method: Method,
    Path((module, path)): Path<(String, String)>,
) -> Response {
    let principal = format!("User::\"{}\"", current.user.email);
    let full_path = format!("/{path}");
    match state
        .gateway
        .handle(&principal, &module, method.as_str(), &full_path)
        .await
    {
        GatewayOutcome::Proxied { status, body } => {
            let status = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
            (status, body).into_response()
        }
        GatewayOutcome::Forbidden => Problem::new(StatusCode::FORBIDDEN)
            .with_type("https://superapp/errors/forbidden")
            .detail("permission denied for this module route")
            .into_response(),
        GatewayOutcome::NotFound => Problem::new(StatusCode::NOT_FOUND)
            .detail(format!("no such module route: {module}{full_path}"))
            .into_response(),
        GatewayOutcome::Unavailable(reason) => Problem::new(StatusCode::BAD_GATEWAY)
            .with_type("https://superapp/errors/module-unavailable")
            .detail(reason)
            .into_response(),
    }
}

/// Routes mounted under the versioned API base.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/gateway")
        .add("/{module}/{*path}", get(proxy))
}
