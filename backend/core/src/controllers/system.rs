//! System endpoints: liveness, readiness, and Prometheus metrics.
//!
//! * `GET /health`  — liveness (TR-03-006): `200` while the process is up.
//! * `GET /ready`   — readiness (TR-03-006): `200` when every probed
//!   dependency is reachable, else `503` as an RFC 9457 problem whose
//!   `dependencies` extension member lists each dependency's status.
//! * `GET /metrics` — Prometheus exposition (TR-03-007).
//!
//! These are mounted at the root (unversioned) as is conventional for
//! operational endpoints.

use axum::http::{header, StatusCode};
use loco_rs::prelude::*;
use serde::Serialize;

use crate::{
    health::{all_up, collect_readiness, DepStatus},
    metrics::{METRICS, PROMETHEUS_CONTENT_TYPE},
    response::{Problem, Success},
};

/// Liveness payload.
#[derive(Debug, Serialize)]
pub struct Liveness {
    /// Always `"ok"`.
    pub status: &'static str,
}

/// Readiness payload (success case).
#[derive(Debug, Serialize)]
pub struct Readiness {
    /// Always `true` in the success case.
    pub ready: bool,
    /// Per-dependency reachability.
    pub dependencies: Vec<DepStatus>,
}

/// `GET /health` — liveness probe.
#[debug_handler]
pub async fn health() -> Success<Liveness> {
    Success::new(Liveness { status: "ok" })
}

/// `GET /ready` — readiness probe.
#[debug_handler]
pub async fn ready(State(ctx): State<AppContext>) -> Response {
    let statuses = collect_readiness(&ctx).await;

    if all_up(&statuses) {
        return Success::new(Readiness {
            ready: true,
            dependencies: statuses,
        })
        .into_response();
    }

    let down: Vec<&str> = statuses
        .iter()
        .filter(|d| !d.is_up())
        .map(|d| d.name.as_str())
        .collect();

    let mut problem = Problem::new(StatusCode::SERVICE_UNAVAILABLE)
        .with_type("https://superapp/errors/not-ready")
        .detail(format!("unreachable dependencies: {}", down.join(", ")));
    // RFC 9457 extension member with the full per-dependency breakdown.
    if let Ok(value) = serde_json::to_value(&statuses) {
        problem.extensions.insert("dependencies".to_string(), value);
    }
    problem.into_response()
}

/// `GET /metrics` — Prometheus text exposition.
#[debug_handler]
pub async fn metrics() -> Response {
    (
        [(header::CONTENT_TYPE, PROMETHEUS_CONTENT_TYPE)],
        METRICS.render(),
    )
        .into_response()
}

/// Root-mounted system routes.
pub fn routes() -> Routes {
    Routes::new()
        .add("/health", get(health))
        .add("/ready", get(ready))
        .add("/metrics", get(metrics))
}
