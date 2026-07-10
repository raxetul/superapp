//! Integration tests for the baseline `/api/v1` controller.
//!
//! Covers TR-03-001 (versioned base + baseline route + success envelope),
//! TR-03-004 (validation → RFC 9457 `422`), and the request-id response
//! header aspect of TR-03-005.

use loco_rs::testing::prelude::*;
use serde_json::{json, Value};
use serial_test::serial;
use superapp_core::app::App;

#[tokio::test]
#[serial]
async fn ping_returns_200_success_envelope_with_request_id_header() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/v1/ping").await;

        // TR-03-001: baseline route under /api/v1 returns 200.
        assert_eq!(res.status_code(), 200);
        // Success envelope is application/json.
        assert_eq!(res.content_type(), "application/json");
        // TR-03-005: per-request correlation id echoed in a response header.
        assert!(
            res.maybe_header("x-request-id").is_some(),
            "x-request-id response header must be present"
        );

        // House success envelope shape.
        let body: Value = res.json();
        assert_eq!(body["success"], json!(true));
        assert_eq!(body["data"]["status"], json!("ok"));
        assert_eq!(body["message"], json!("pong"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn echo_rejects_invalid_body_with_rfc9457_problem() {
    request::<App, _, _>(|request, _ctx| async move {
        // Empty message violates the `min = 1` rule.
        let res = request
            .post("/api/v1/echo")
            .json(&json!({ "message": "" }))
            .await;

        // TR-03-004: validation failure → 422 as application/problem+json.
        assert_eq!(res.status_code(), 422);
        assert_eq!(res.content_type(), "application/problem+json");

        let body: Value = res.json();
        assert_eq!(body["status"], json!(422));
        assert_eq!(body["type"], json!("https://superapp/errors/validation"));
        // The errors extension member carries a JSON Pointer to the bad field.
        assert_eq!(body["errors"][0]["pointer"], json!("/message"));
        assert!(body["errors"][0]["detail"].is_string());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn echo_accepts_valid_body_with_success_envelope() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request
            .post("/api/v1/echo")
            .json(&json!({ "message": "hello" }))
            .await;

        // TR-03-004: valid input → 2xx.
        assert_eq!(res.status_code(), 200);
        assert_eq!(res.content_type(), "application/json");

        let body: Value = res.json();
        assert_eq!(body["success"], json!(true));
        assert_eq!(body["data"]["message"], json!("hello"));
    })
    .await;
}
