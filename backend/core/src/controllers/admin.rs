//! Admin management endpoints (P4): the email allow-list and user-role
//! management. Every route is gated by the Cedar enforcement point
//! (`admin.access` on `AdminPanel::"main"`, TR-04-005): a non-admin is denied
//! `403` before any state changes, an admin is allowed.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::Extension;
use cedar_policy::Context;
use loco_rs::prelude::*;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::auth::extractor::CurrentUser;
use crate::auth::state::AuthState;
use crate::extractors::ValidatedJson;
use crate::models::allowlisted_emails::{
    Entity as AllowlistedEmailEntity, Model as AllowlistedEmail,
};
use crate::models::{role::Role, users};
use crate::response::{Problem, Success};

const ADMIN_PANEL: &str = "AdminPanel::\"main\"";
const ADMIN_ACTION: &str = "Action::\"admin.access\"";

/// Enforce that the current user may access the admin panel (Cedar). Returns a
/// `403` problem on denial.
async fn require_admin(state: &Arc<AuthState>, current: &CurrentUser) -> Result<(), Problem> {
    let principal = format!("User::\"{}\"", current.user.email);
    let decision = state
        .enforcer
        .enforce(&principal, ADMIN_ACTION, ADMIN_PANEL, Context::empty())
        .await
        .map_err(|e| Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(e.to_string()))?;
    if decision.allowed {
        Ok(())
    } else {
        Err(Problem::new(StatusCode::FORBIDDEN)
            .with_type("https://superapp/errors/forbidden")
            .detail("administrator privileges are required"))
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct AllowlistParams {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct AllowlistEntry {
    pub email: String,
}

/// `POST /admin/allowlist` — add an email to the allow-list.
async fn allowlist_add(
    State(ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    current: CurrentUser,
    ValidatedJson(params): ValidatedJson<AllowlistParams>,
) -> Result<Success<AllowlistEntry>, Problem> {
    require_admin(&state, &current).await?;
    let entry = AllowlistedEmail::add(&ctx.db, &params.email, Some(&current.user.email))
        .await
        .map_err(|e| Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(e.to_string()))?;
    Ok(Success::new(AllowlistEntry { email: entry.email }).message("allow-listed"))
}

/// `POST /admin/allowlist/remove` — remove an email from the allow-list.
async fn allowlist_remove(
    State(ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    current: CurrentUser,
    ValidatedJson(params): ValidatedJson<AllowlistParams>,
) -> Result<Success<serde_json::Value>, Problem> {
    require_admin(&state, &current).await?;
    let removed = AllowlistedEmail::remove(&ctx.db, &params.email)
        .await
        .map_err(|e| Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(e.to_string()))?;
    Ok(Success::new(serde_json::json!({ "removed": removed })))
}

/// `GET /admin/allowlist` — list allow-listed emails.
async fn allowlist_list(
    State(ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    current: CurrentUser,
) -> Result<Success<Vec<AllowlistEntry>>, Problem> {
    require_admin(&state, &current).await?;
    let rows = AllowlistedEmailEntity::find()
        .all(&ctx.db)
        .await
        .map_err(|e| Problem::new(StatusCode::INTERNAL_SERVER_ERROR).detail(e.to_string()))?;
    Ok(Success::new(
        rows.into_iter()
            .map(|r| AllowlistEntry { email: r.email })
            .collect(),
    ))
}

#[derive(Debug, Deserialize, Validate)]
pub struct SetRoleParams {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(custom(function = "validate_role"))]
    pub role: String,
}

fn validate_role(role: &str) -> Result<(), validator::ValidationError> {
    match role {
        "admin" | "user" => Ok(()),
        _ => Err(validator::ValidationError::new("invalid_role")),
    }
}

#[derive(Debug, Serialize)]
pub struct UserRole {
    pub email: String,
    pub role: String,
}

/// `PUT /admin/users/role` — set a user's role (admin/user).
async fn set_role(
    State(ctx): State<AppContext>,
    Extension(state): Extension<Arc<AuthState>>,
    current: CurrentUser,
    ValidatedJson(params): ValidatedJson<SetRoleParams>,
) -> Result<Success<UserRole>, Problem> {
    require_admin(&state, &current).await?;
    let role = Role::from_stored(&params.role);
    let user = users::Model::set_role(&ctx.db, &params.email, role)
        .await
        .map_err(|_| {
            Problem::new(StatusCode::NOT_FOUND).detail(format!("no such user: {}", params.email))
        })?;
    let role_str = user.role().as_str().to_string();
    Ok(Success::new(UserRole {
        email: user.email,
        role: role_str,
    })
    .message("role updated"))
}

/// Routes mounted under the versioned API base.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/admin")
        .add("/allowlist", get(allowlist_list))
        .add("/allowlist", post(allowlist_add))
        .add("/allowlist/remove", post(allowlist_remove))
        .add("/users/role", put(set_role))
}
