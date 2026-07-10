//! Authentication endpoints (P4) — Rauthy OIDC, replacing loco's native auth.
//!
//! Routes (under `/api/v1/auth`):
//! - `GET  /capabilities` — public; reports the self-registration toggle and
//!   whether OIDC is configured (backs the frontend's conditional UI,
//!   FR-07-004).
//! - `GET  /login` — begins the authorization-code flow (TR-04-001).
//! - `POST /callback` — completes login: exchange code → validate token →
//!   provision → open refresh session (TR-04-001/004/010).
//! - `POST /refresh` — rotates the session (TR-04-003).
//! - `POST /logout` — revokes the refresh handle.
//! - `GET  /me` — the current user (protected; TR-04-002).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;

use crate::auth::extractor::CurrentUser;
use crate::auth::service::{complete_login, refresh_session, LoginError, SessionTokens};
use crate::auth::state::AuthState;
use crate::extractors::ValidatedJson;
use crate::response::{Problem, Success};

/// `GET /auth/capabilities` — public auth capabilities.
#[derive(Debug, Serialize)]
pub struct Capabilities {
    /// Whether self-registration is enabled (TR-04-011).
    pub self_registration_enabled: bool,
    /// Whether an OIDC provider (Rauthy) is configured.
    pub oidc_configured: bool,
}

async fn capabilities(Extension(state): Extension<Arc<AuthState>>) -> Success<Capabilities> {
    Success::new(Capabilities {
        self_registration_enabled: state.self_registration_enabled,
        oidc_configured: state.oidc_configured,
    })
}

/// `GET /auth/login` — begin the authorization-code flow.
#[derive(Debug, Serialize)]
pub struct LoginStart {
    pub authorize_url: String,
    pub state: String,
    pub pkce_verifier: String,
    pub nonce: String,
}

async fn login(
    Extension(state): Extension<Arc<AuthState>>,
) -> Result<Success<LoginStart>, Problem> {
    let oidc = state.oidc.as_ref().ok_or_else(|| {
        Problem::new(StatusCode::SERVICE_UNAVAILABLE).detail("OIDC is not configured")
    })?;
    let redirect = oidc
        .authorize_url()
        .map_err(|e| Problem::new(StatusCode::BAD_GATEWAY).detail(e.to_string()))?;
    Ok(Success::new(LoginStart {
        authorize_url: redirect.url,
        state: redirect.csrf_state,
        pkce_verifier: redirect.pkce_verifier,
        nonce: redirect.nonce,
    }))
}

/// Body of `POST /auth/callback`.
#[derive(Debug, Deserialize, Validate)]
pub struct CallbackParams {
    #[validate(length(min = 1, message = "authorization code is required"))]
    pub code: String,
    #[validate(length(min = 1, message = "pkce_verifier is required"))]
    pub pkce_verifier: String,
}

/// The session payload returned on login/refresh.
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub access_token: String,
    pub refresh_handle: String,
    pub expires_in_secs: Option<u64>,
    pub email: String,
    pub role: String,
}

impl From<SessionTokens> for SessionResponse {
    fn from(t: SessionTokens) -> Self {
        Self {
            access_token: t.access_token,
            refresh_handle: t.refresh_handle,
            expires_in_secs: t.expires_in_secs,
            email: t.email,
            role: t.role,
        }
    }
}

async fn callback(
    State(ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    ValidatedJson(params): ValidatedJson<CallbackParams>,
) -> Result<Success<SessionResponse>, Problem> {
    let (oidc, validator) = require_oidc(&state)?;
    let tokens = complete_login(
        oidc.as_ref(),
        validator,
        state.refresh.as_ref(),
        &ctx.db,
        state.self_registration_enabled,
        &params.code,
        &params.pkce_verifier,
    )
    .await
    .map_err(login_error_to_problem)?;
    Ok(Success::new(SessionResponse::from(tokens)).message("logged in"))
}

/// Body of `POST /auth/refresh` and `POST /auth/logout`.
#[derive(Debug, Deserialize, Validate)]
pub struct RefreshParams {
    #[validate(length(min = 1, message = "refresh_handle is required"))]
    pub refresh_handle: String,
}

async fn refresh(
    State(ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    ValidatedJson(params): ValidatedJson<RefreshParams>,
) -> Result<Success<SessionResponse>, Problem> {
    let (oidc, validator) = require_oidc(&state)?;
    let tokens = refresh_session(
        oidc.as_ref(),
        validator,
        state.refresh.as_ref(),
        &ctx.db,
        &params.refresh_handle,
    )
    .await
    .map_err(login_error_to_problem)?;
    Ok(Success::new(SessionResponse::from(tokens)).message("refreshed"))
}

async fn logout(
    Extension(state): Extension<Arc<AuthState>>,
    ValidatedJson(params): ValidatedJson<RefreshParams>,
) -> Result<Success<serde_json::Value>, Problem> {
    state
        .refresh
        .revoke(&params.refresh_handle)
        .await
        .map_err(|e| Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(e.to_string()))?;
    Ok(Success::new(json!({ "revoked": true })).message("logged out"))
}

/// `GET /auth/me` — the current authenticated user (protected route).
async fn me(current: CurrentUser) -> Success<serde_json::Value> {
    Success::new(json!({
        "pid": current.user.pid,
        "email": current.user.email,
        "name": current.user.name,
        "role": current.user.role().as_str(),
    }))
}

/// Require both an OIDC provider and a token validator to be wired.
#[allow(clippy::type_complexity)]
fn require_oidc(
    state: &Arc<AuthState>,
) -> Result<
    (
        Arc<dyn crate::auth::oidc::OidcProvider>,
        &crate::auth::token::TokenValidator,
    ),
    Problem,
> {
    let oidc = state.oidc.clone().ok_or_else(|| {
        Problem::new(StatusCode::SERVICE_UNAVAILABLE).detail("OIDC is not configured")
    })?;
    let validator = state.validator.as_deref().ok_or_else(|| {
        Problem::new(StatusCode::SERVICE_UNAVAILABLE).detail("token validation is not configured")
    })?;
    Ok((oidc, validator))
}

/// Map a [`LoginError`] to an RFC 9457 problem with the right status.
fn login_error_to_problem(e: LoginError) -> Problem {
    use crate::auth::oidc::OidcError;
    use crate::auth::provisioning::ProvisionError;
    use crate::auth::refresh::RefreshError;
    match e {
        LoginError::Token(err) => Problem::new(StatusCode::UNAUTHORIZED).detail(err.to_string()),
        LoginError::Provision(ProvisionError::NotAllowed(_)) => Problem::new(StatusCode::FORBIDDEN)
            .detail("onboarding is not permitted for this identity"),
        LoginError::Provision(ProvisionError::MissingEmail) => {
            Problem::new(StatusCode::UNAUTHORIZED).detail("token carried no email claim")
        }
        LoginError::Provision(ProvisionError::Db(err)) => {
            Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(err.to_string())
        }
        LoginError::Refresh(RefreshError::UnknownHandle) => {
            Problem::new(StatusCode::UNAUTHORIZED).detail("refresh handle is invalid or expired")
        }
        LoginError::Refresh(err) => {
            Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(err.to_string())
        }
        LoginError::Oidc(OidcError::NotConfigured) => {
            Problem::new(StatusCode::SERVICE_UNAVAILABLE).detail("OIDC is not configured")
        }
        LoginError::Oidc(err) => Problem::new(StatusCode::BAD_GATEWAY).detail(err.to_string()),
    }
}

/// Routes mounted under the versioned API base.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/auth")
        .add("/capabilities", get(capabilities))
        .add("/login", get(login))
        .add("/callback", post(callback))
        .add("/refresh", post(refresh))
        .add("/logout", post(logout))
        .add("/me", get(me))
}
