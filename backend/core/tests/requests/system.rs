//! Integration tests for the system endpoints.
//!
//! Covers TR-03-006 (`/health` liveness, `/ready` readiness) and TR-03-007
//! (`/metrics` Prometheus exposition). The readiness *aggregation* (503 when a
//! dependency is down) is unit-tested in `superapp_core::health`; here we
//! assert the live endpoints against the test profile (DB-only readiness).

use loco_rs::testing::prelude::*;
use serde_json::{json, Value};
use serial_test::serial;
use superapp_core::app::App;

#[tokio::test]
#[serial]
async fn health_liveness_returns_200() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/health").await;
        assert_eq!(res.status_code(), 200);
        let body: Value = res.json();
        assert_eq!(body["success"], json!(true));
        assert_eq!(body["data"]["status"], json!("ok"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ready_returns_200_when_dependencies_reachable() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/ready").await;
        // Test profile probes only the (reachable) PGlite database.
        assert_eq!(res.status_code(), 200);
        let body: Value = res.json();
        // Readiness payload is wrapped in the house success envelope.
        assert_eq!(body["success"], json!(true));
        assert_eq!(body["data"]["ready"], json!(true));
        let deps = body["data"]["dependencies"]
            .as_array()
            .expect("dependencies array");
        assert!(deps
            .iter()
            .any(|d| d["name"] == json!("postgres") && d["state"] == json!("up")));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn metrics_endpoint_exposes_prometheus_http_metrics() {
    request::<App, _, _>(|request, _ctx| async move {
        // Generate at least one counted request first.
        let _ = request.get("/api/v1/ping").await;

        let res = request.get("/metrics").await;
        assert_eq!(res.status_code(), 200);
        assert!(
            res.content_type().starts_with("text/plain"),
            "metrics must be Prometheus text format, got {}",
            res.content_type()
        );
        let body = res.text();
        assert!(body.contains("# TYPE http_requests_total counter"));
        assert!(body.contains("http_requests_total{"));
    })
    .await;
}
