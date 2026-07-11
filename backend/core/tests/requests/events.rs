//! Request-level test for the SSE endpoint gate (TR-06-001).
//!
//! The unauthenticated case returns `401` immediately. Live event delivery on
//! the long-lived connection is proven by the hub unit tests
//! (`events::hub::tests`), which a request-harness `.await` cannot observe
//! (the stream never terminates).

use loco_rs::testing::prelude::*;
use serial_test::serial;
use superapp_core::app::App;

#[tokio::test]
#[serial]
async fn event_stream_requires_authentication() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/v1/events/stream").await;
        assert_eq!(res.status_code(), 401);
        assert_eq!(res.content_type(), "application/problem+json");
    })
    .await;
}
