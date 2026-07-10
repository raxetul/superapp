//! Shared, typed HTTP response models.
//!
//! Two mutually-exclusive shapes, per `project.md` and the project's API rules
//! (see `CLAUDE.md`):
//!
//! * **Success (2xx)** — the *house envelope* `{ success, data, message,
//!   pagination }` served as `application/json` ([`Success`]).
//! * **Error (non-2xx)** — an **RFC 9457 Problem Details** document served as
//!   `application/problem+json` ([`Problem`]).
//!
//! Both implement [`IntoResponse`], so controllers return these typed values
//! directly and never hand-roll error JSON.

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// RFC 9457 problem-detail media type.
pub const PROBLEM_JSON: &str = "application/problem+json";

/// The house success envelope for all 2xx responses.
///
/// `success` is always `true`; errors use [`Problem`] instead. `message` and
/// `pagination` are omitted from the wire form when absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Success<T> {
    /// Always `true` — discriminates the success envelope from an error body.
    pub success: bool,
    /// The response payload.
    pub data: T,
    /// Optional human-readable message.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
    /// Present only for paginated collection responses.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub pagination: Option<Pagination>,
}

impl<T> Success<T> {
    /// A `200`-style envelope wrapping `data`, with no message or pagination.
    #[must_use]
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            data,
            message: None,
            pagination: None,
        }
    }

    /// Attach a human-readable message.
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Attach pagination metadata (for collection responses).
    #[must_use]
    pub fn pagination(mut self, pagination: Pagination) -> Self {
        self.pagination = Some(pagination);
        self
    }

    /// Render as a response with an explicit status code (e.g. `201 Created`).
    #[must_use]
    pub fn into_response_with_status(self, status: StatusCode) -> Response
    where
        T: Serialize,
    {
        (status, Json(self)).into_response()
    }
}

impl<T: Serialize> IntoResponse for Success<T> {
    fn into_response(self) -> Response {
        // `Json` sets `content-type: application/json`.
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Pagination metadata for collection responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pagination {
    /// 1-based page number.
    pub page: u64,
    /// Items per page.
    pub per_page: u64,
    /// Total number of items across all pages.
    pub total_items: u64,
    /// Total number of pages.
    pub total_pages: u64,
}

/// A single field-level validation failure, carried in [`Problem::errors`].
///
/// `pointer` is a JSON Pointer (RFC 6901) into the request body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldError {
    /// JSON Pointer into the request body, e.g. `/email`.
    pub pointer: String,
    /// Human-readable description of the failure.
    pub detail: String,
}

impl FieldError {
    /// Construct a field error for `pointer` with `detail`.
    pub fn new(pointer: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            pointer: pointer.into(),
            detail: detail.into(),
        }
    }
}

/// An [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) Problem Details object.
///
/// Serialized as `application/problem+json`. `type` defaults to `about:blank`
/// and `title` to the status code's canonical reason phrase. `errors` is an
/// extension member carrying field-level validation failures (empty ⇒ omitted).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    /// URI reference identifying the problem type; `about:blank` when generic.
    #[serde(rename = "type")]
    pub type_: String,
    /// Short, human-readable summary of the problem type.
    pub title: String,
    /// HTTP status code, mirrored into the body.
    pub status: u16,
    /// Occurrence-specific, human-readable explanation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub detail: Option<String>,
    /// URI reference identifying the specific occurrence (e.g. request path).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub instance: Option<String>,
    /// Extension member: per-field validation failures (omitted when empty).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<FieldError>,
    /// Arbitrary additional RFC 9457 extension members, flattened to the top
    /// level of the document (empty ⇒ nothing emitted).
    #[serde(flatten, default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

impl Problem {
    /// A generic problem for `status`: `type = about:blank`, `title =` the
    /// canonical reason phrase.
    #[must_use]
    pub fn new(status: StatusCode) -> Self {
        Self {
            type_: "about:blank".to_string(),
            title: status.canonical_reason().unwrap_or("Unknown").to_string(),
            status: status.as_u16(),
            detail: None,
            instance: None,
            errors: Vec::new(),
            extensions: serde_json::Map::new(),
        }
    }

    /// Set the problem `type` URI (and, by convention, a stable `title`).
    #[must_use]
    pub fn with_type(mut self, type_: impl Into<String>) -> Self {
        self.type_ = type_.into();
        self
    }

    /// Override the `title`.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the occurrence-specific `detail`.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the `instance` URI (typically the request path).
    #[must_use]
    pub fn instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// Attach field-level validation errors (extension member).
    #[must_use]
    pub fn with_errors(mut self, errors: Vec<FieldError>) -> Self {
        self.errors = errors;
        self
    }

    /// The status as an [`axum::http::StatusCode`].
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = serde_json::to_vec(&self).unwrap_or_else(|_| {
            br#"{"type":"about:blank","title":"Internal Server Error","status":500}"#.to_vec()
        });
        (status, [(header::CONTENT_TYPE, PROBLEM_JSON)], body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::{json, Value};

    #[tokio::test]
    async fn success_envelope_serializes_to_house_shape() {
        let resp = Success::new(json!({ "id": 1 }))
            .message("ok")
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["success"], json!(true));
        assert_eq!(v["data"], json!({ "id": 1 }));
        assert_eq!(v["message"], json!("ok"));
        // pagination omitted when absent
        assert!(v.get("pagination").is_none());
    }

    #[test]
    fn success_omits_optional_members_when_absent() {
        let v = serde_json::to_value(Success::new(json!({}))).unwrap();
        assert!(v.get("message").is_none());
        assert!(v.get("pagination").is_none());
        assert_eq!(v["success"], json!(true));
    }

    #[test]
    fn success_includes_pagination_when_present() {
        let s = Success::new(vec![1, 2, 3]).pagination(Pagination {
            page: 1,
            per_page: 3,
            total_items: 9,
            total_pages: 3,
        });
        let v = serde_json::to_value(s).unwrap();
        assert_eq!(v["pagination"]["total_pages"], json!(3));
    }

    #[tokio::test]
    async fn problem_serializes_to_rfc9457_with_problem_json_content_type() {
        let resp = Problem::new(StatusCode::NOT_FOUND)
            .detail("no such widget")
            .instance("/api/v1/widgets/9")
            .into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some(PROBLEM_JSON)
        );
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["type"], json!("about:blank"));
        assert_eq!(v["title"], json!("Not Found"));
        assert_eq!(v["status"], json!(404));
        assert_eq!(v["detail"], json!("no such widget"));
        assert_eq!(v["instance"], json!("/api/v1/widgets/9"));
        // errors extension omitted when empty
        assert!(v.get("errors").is_none());
    }

    #[test]
    fn problem_carries_field_errors_extension_member() {
        let p = Problem::new(StatusCode::UNPROCESSABLE_ENTITY)
            .with_type("https://superapp/errors/validation")
            .with_errors(vec![FieldError::new("/email", "must be a valid email")]);
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["status"], json!(422));
        assert_eq!(v["type"], json!("https://superapp/errors/validation"));
        assert_eq!(v["errors"][0]["pointer"], json!("/email"));
        assert_eq!(v["errors"][0]["detail"], json!("must be a valid email"));
    }

    #[test]
    fn problem_title_defaults_to_canonical_reason() {
        assert_eq!(
            Problem::new(StatusCode::UNPROCESSABLE_ENTITY).title,
            "Unprocessable Entity"
        );
        assert_eq!(
            Problem::new(StatusCode::SERVICE_UNAVAILABLE).title,
            "Service Unavailable"
        );
    }
}
