//! Baseline `/api/v1` controller.
//!
//! Establishes the versioned API base (TR-03-001) and provides reference
//! endpoints that exercise the shared response contract: [`ping`] returns the
//! house success envelope, and [`echo`] demonstrates request validation
//! rejecting with an RFC 9457 problem (TR-03-004) via [`ValidatedJson`].

use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    extractors::ValidatedJson,
    response::{Problem, Success},
};

/// Payload of the baseline liveness/version ping.
#[derive(Debug, Serialize)]
pub struct Pong {
    /// Always `"ok"`.
    pub status: &'static str,
    /// API version string.
    pub version: &'static str,
}

/// `GET /api/v1/ping` — baseline route proving the versioned API base is up.
///
/// Returns `200` with the house success envelope.
#[debug_handler]
pub async fn ping() -> Success<Pong> {
    Success::new(Pong {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
    .message("pong")
}

/// Request body for [`echo`], with validation rules.
#[derive(Debug, Deserialize, Validate)]
pub struct EchoRequest {
    /// Must be a non-empty string of at most 280 chars.
    #[validate(length(min = 1, max = 280, message = "must be between 1 and 280 characters"))]
    pub message: String,
}

/// Echoed payload.
#[derive(Debug, Serialize)]
pub struct EchoResponse {
    /// The echoed message.
    pub message: String,
}

/// `POST /api/v1/echo` — reference validated endpoint.
///
/// Returns `200` + house envelope for a valid body, or `422`
/// `application/problem+json` with per-field `errors` for an invalid one.
#[debug_handler]
pub async fn echo(
    ValidatedJson(body): ValidatedJson<EchoRequest>,
) -> Result<Success<EchoResponse>, Problem> {
    Ok(Success::new(EchoResponse {
        message: body.message,
    }))
}

/// Routes mounted under the versioned API base.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1")
        .add("/ping", get(ping))
        .add("/echo", post(echo))
}
