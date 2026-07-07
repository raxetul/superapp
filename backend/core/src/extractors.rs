//! Request extractors that enforce the project's error contract.
//!
//! [`ValidatedJson`] deserializes a JSON body and runs `validator` rules,
//! rejecting with an RFC 9457 [`Problem`](crate::response::Problem) instead of
//! ad-hoc JSON: malformed bodies become `400`, and validation failures become
//! `422` whose `errors` extension member lists the offending fields.

use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    Json,
};
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors};

use crate::response::{FieldError, Problem};

/// Problem `type` URI for request-validation failures.
pub const VALIDATION_TYPE: &str = "https://superapp/errors/validation";

/// Extractor that deserializes `T` from the JSON body and validates it.
///
/// Rejection is always a [`Problem`]: `400` for a malformed/again wrong-content
/// body, `422` for a well-formed body that fails validation.
#[derive(Debug, Clone, Copy)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Problem;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(problem_from_json_rejection)?;
        value.validate().map_err(|e| problem_from_validation(&e))?;
        Ok(Self(value))
    }
}

/// Map a serde/axum JSON rejection to a `400` problem.
fn problem_from_json_rejection(rejection: JsonRejection) -> Problem {
    Problem::new(StatusCode::BAD_REQUEST).detail(rejection.body_text())
}

/// Convert `validator` errors into a `422` problem whose `errors` extension
/// member carries one entry per invalid field (JSON Pointer + message).
///
/// Public so controllers/other extractors can reuse the exact mapping.
#[must_use]
pub fn problem_from_validation(errors: &ValidationErrors) -> Problem {
    let mut field_errors: Vec<FieldError> = Vec::new();
    for (field, errs) in errors.field_errors() {
        for err in errs {
            let detail = err
                .message
                .as_ref()
                .map(std::string::ToString::to_string)
                .unwrap_or_else(|| format!("invalid value ({})", err.code));
            field_errors.push(FieldError::new(format!("/{field}"), detail));
        }
    }
    // Deterministic ordering for stable tests and client display.
    field_errors.sort_by(|a, b| a.pointer.cmp(&b.pointer).then(a.detail.cmp(&b.detail)));

    Problem::new(StatusCode::UNPROCESSABLE_ENTITY)
        .with_type(VALIDATION_TYPE)
        .with_title("Unprocessable Entity")
        .detail("Request validation failed")
        .with_errors(field_errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, Deserialize, Validate)]
    struct SignupDto {
        #[validate(email)]
        email: String,
        #[validate(length(min = 8, message = "must be at least 8 characters"))]
        password: String,
    }

    #[test]
    fn valid_dto_produces_no_errors() {
        let dto = SignupDto {
            email: "a@b.com".into(),
            password: "longenough".into(),
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn invalid_dto_maps_to_422_problem_with_field_pointers() {
        let dto = SignupDto {
            email: "not-an-email".into(),
            password: "short".into(),
        };
        let err = dto.validate().unwrap_err();
        let problem = problem_from_validation(&err);

        assert_eq!(problem.status, 422);
        assert_eq!(problem.type_, VALIDATION_TYPE);

        let v = serde_json::to_value(&problem).unwrap();
        // Two invalid fields, each a JSON Pointer into the body.
        let pointers: Vec<&str> = v["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["pointer"].as_str().unwrap())
            .collect();
        assert!(pointers.contains(&"/email"));
        assert!(pointers.contains(&"/password"));
        // Custom message is preserved.
        assert_eq!(
            v["errors"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["pointer"] == json!("/password"))
                .unwrap()["detail"],
            json!("must be at least 8 characters")
        );
    }
}
