//! Login / refresh orchestration (TR-04-001, TR-04-003, TR-04-004, TR-04-010).
//!
//! These functions tie together the injected collaborators — the OIDC provider
//! (network), the token validator (JWKS), the refresh store (Redis), and the
//! database (provisioning) — behind their abstractions. They contain no I/O of
//! their own beyond calling those seams, so tests drive them with in-memory
//! fakes plus the isolated test DB.

use sea_orm::DatabaseConnection;

use crate::auth::oidc::{OidcError, OidcProvider};
use crate::auth::provisioning::{provision, ProvisionError, ProvisionInput};
use crate::auth::refresh::{RefreshError, RefreshRecord, RefreshTokens};
use crate::auth::token::{TokenError, TokenValidator};

/// The session material returned to a client after login/refresh. The client
/// gets a short-lived access token plus an opaque `refresh_handle` (never the
/// raw Rauthy refresh token, which stays in Redis).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTokens {
    /// The Rauthy access token (a JWT the client sends as a bearer).
    pub access_token: String,
    /// Opaque handle the client presents to `/auth/refresh`.
    pub refresh_handle: String,
    /// Access-token lifetime in seconds, when known.
    pub expires_in_secs: Option<u64>,
    /// The authenticated user's email (identity key).
    pub email: String,
    /// The user's role (`admin`/`user`).
    pub role: String,
}

/// Failures during login/refresh, each mapped to an HTTP status by the caller.
#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    /// OIDC exchange/refresh failed (upstream/config) → `502`/`401`.
    #[error(transparent)]
    Oidc(#[from] OidcError),
    /// The access token failed validation → `401`.
    #[error("invalid access token: {0}")]
    Token(#[from] TokenError),
    /// Onboarding denied or email missing → `403`.
    #[error(transparent)]
    Provision(#[from] ProvisionError),
    /// The refresh handle was unknown/expired/reused → `401`.
    #[error(transparent)]
    Refresh(#[from] RefreshError),
}

/// Complete the authorization-code flow: exchange the code, validate the
/// resulting access token, provision the user by email, and open a refresh
/// session.
///
/// # Errors
/// See [`LoginError`].
pub async fn complete_login(
    oidc: &dyn OidcProvider,
    validator: &TokenValidator,
    refresh: &dyn RefreshTokens,
    db: &DatabaseConnection,
    self_registration_enabled: bool,
    code: &str,
    pkce_verifier: &str,
) -> Result<SessionTokens, LoginError> {
    let tokens = oidc.exchange_code(code, pkce_verifier).await?;
    let claims = validator.validate(&tokens.access_token)?;
    let email = claims
        .email()
        .ok_or(ProvisionError::MissingEmail)?
        .to_string();

    let user = provision(
        db,
        &ProvisionInput {
            email: email.clone(),
            name: claims.name.clone(),
            self_registration_enabled,
        },
    )
    .await?;

    let handle = refresh
        .issue(&RefreshRecord {
            user_id: user.email.clone(),
            refresh_token: tokens.refresh_token.unwrap_or_default(),
        })
        .await?;

    let role = user.role().as_str().to_string();
    Ok(SessionTokens {
        access_token: tokens.access_token,
        refresh_handle: handle,
        expires_in_secs: tokens.expires_in_secs,
        email: user.email,
        role,
    })
}

/// Refresh a session: look up the stored Rauthy refresh token by handle,
/// exchange it upstream, validate the new access token, and rotate the handle
/// (so the presented one can never be replayed).
///
/// # Errors
/// See [`LoginError`]; a reused/expired handle is [`LoginError::Refresh`].
pub async fn refresh_session(
    oidc: &dyn OidcProvider,
    validator: &TokenValidator,
    refresh: &dyn RefreshTokens,
    db: &DatabaseConnection,
    refresh_handle: &str,
) -> Result<SessionTokens, LoginError> {
    let record = refresh
        .get(refresh_handle)
        .await?
        .ok_or(RefreshError::UnknownHandle)?;

    let tokens = oidc.refresh(&record.refresh_token).await?;
    // Validate the freshly-minted access token before handing it back.
    validator.validate(&tokens.access_token)?;

    let new_handle = refresh
        .rotate(
            refresh_handle,
            &RefreshRecord {
                user_id: record.user_id.clone(),
                refresh_token: tokens.refresh_token.unwrap_or_default(),
            },
        )
        .await?;

    // Reflect the user's *current* role (it may have changed since login).
    let role = crate::models::users::Model::find_by_email(db, &record.user_id)
        .await
        .map(|u| u.role().as_str().to_string())
        .unwrap_or_default();

    Ok(SessionTokens {
        access_token: tokens.access_token,
        refresh_handle: new_handle,
        expires_in_secs: tokens.expires_in_secs,
        email: record.user_id,
        role,
    })
}
