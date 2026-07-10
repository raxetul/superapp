//! Request extractors for authentication (TR-04-002, TR-04-009).
//!
//! - [`AuthedClaims`] validates the `Authorization: Bearer <jwt>` Rauthy access
//!   token against the injected [`TokenValidator`] and yields its claims. A
//!   protected route that takes this extractor returns `401` without a valid
//!   token and proceeds (`2xx`) with one — regardless of whether the user
//!   logged in via SSO or username/password (TR-04-010).
//! - [`CurrentUser`] additionally resolves the provisioned application user by
//!   email.
//! - [`ApiKey`] authenticates service-to-service callers via `X-API-Key`
//!   (TR-04-009).
//!
//! The [`TokenValidator`] arrives by injection (an Axum `Extension` layered at
//! the composition root), so tests inject a validator built from a known JWKS.

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header::AUTHORIZATION, StatusCode};
use loco_rs::app::AppContext;

use crate::auth::token::{Claims, TokenValidator};
use crate::models::{api_keys, users};
use crate::response::Problem;

/// Header carrying a service-to-service API key.
pub const API_KEY_HEADER: &str = "x-api-key";

fn unauthorized(detail: &str) -> Problem {
    Problem::new(StatusCode::UNAUTHORIZED)
        .with_type("https://superapp/errors/unauthorized")
        .detail(detail.to_string())
}

/// Pull a bearer token out of the `Authorization` header.
fn bearer(parts: &Parts) -> Option<String> {
    let raw = parts.headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = raw.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("bearer") && !token.is_empty() {
        Some(token.to_string())
    } else {
        None
    }
}

/// Validated Rauthy access-token claims for the current request.
#[derive(Debug, Clone)]
pub struct AuthedClaims(pub Claims);

impl FromRequestParts<AppContext> for AuthedClaims {
    type Rejection = Problem;

    async fn from_request_parts(
        parts: &mut Parts,
        _ctx: &AppContext,
    ) -> Result<Self, Self::Rejection> {
        // The validator is injected at the composition root. Its absence means
        // auth is not configured → fail closed.
        let validator = parts
            .extensions
            .get::<Arc<TokenValidator>>()
            .ok_or_else(|| unauthorized("authentication is not configured"))?;
        let token = bearer(parts).ok_or_else(|| unauthorized("missing bearer token"))?;
        let claims = validator
            .validate(&token)
            .map_err(|e| unauthorized(&e.to_string()))?;
        Ok(AuthedClaims(claims))
    }
}

/// The provisioned application user behind a validated token.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    /// The application user record.
    pub user: users::Model,
    /// The validated token claims.
    pub claims: Claims,
}

impl FromRequestParts<AppContext> for CurrentUser {
    type Rejection = Problem;

    async fn from_request_parts(
        parts: &mut Parts,
        ctx: &AppContext,
    ) -> Result<Self, Self::Rejection> {
        let AuthedClaims(claims) = AuthedClaims::from_request_parts(parts, ctx).await?;
        let email = claims
            .email()
            .ok_or_else(|| unauthorized("token carries no email claim"))?;
        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .map_err(|_| unauthorized("user is not provisioned"))?;
        Ok(CurrentUser { user, claims })
    }
}

/// An authenticated service-to-service caller (TR-04-009).
#[derive(Debug, Clone)]
pub struct ApiKey(pub api_keys::Model);

impl FromRequestParts<AppContext> for ApiKey {
    type Rejection = Problem;

    async fn from_request_parts(
        parts: &mut Parts,
        ctx: &AppContext,
    ) -> Result<Self, Self::Rejection> {
        let presented = parts
            .headers
            .get(API_KEY_HEADER)
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| unauthorized("missing X-API-Key"))?;
        let key = api_keys::Model::authenticate(&ctx.db, presented)
            .await
            .map_err(|_| unauthorized("invalid or revoked API key"))?;
        Ok(ApiKey(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    fn parts_with_auth(header: Option<&str>) -> Parts {
        let mut builder = Request::builder().uri("/");
        if let Some(h) = header {
            builder = builder.header(AUTHORIZATION, h);
        }
        builder.body(()).unwrap().into_parts().0
    }

    #[test]
    fn bearer_parsing_is_scheme_insensitive_and_rejects_junk() {
        assert_eq!(
            bearer(&parts_with_auth(Some("Bearer abc"))).as_deref(),
            Some("abc")
        );
        assert_eq!(
            bearer(&parts_with_auth(Some("bearer abc"))).as_deref(),
            Some("abc")
        );
        assert_eq!(bearer(&parts_with_auth(Some("Basic abc"))), None);
        assert_eq!(bearer(&parts_with_auth(Some("Bearer "))), None);
        assert_eq!(bearer(&parts_with_auth(None)), None);
    }

    #[test]
    fn unauthorized_problem_is_401() {
        let p = unauthorized("nope");
        assert_eq!(p.status, 401);
        assert_eq!(p.detail.as_deref(), Some("nope"));
    }
}
// The full 401-without-token / 2xx-with-token gate (TR-04-002) and the
// `X-API-Key` gate (TR-04-009) are proven end-to-end against the real app in
// `tests/requests/auth.rs` and `tests/requests/api_key.rs`.
