//! Email-keyed user provisioning, admin bootstrap, self-registration toggle
//! and allow-list gate (TR-04-004, TR-04-011, TR-04-012, TR-04-013).
//!
//! The onboarding rules, in order:
//! 1. A user already known by email → return it (idempotent; repeat logins
//!    never duplicate — TR-04-004).
//! 2. The very first authenticated user → **admin** (bootstrap), regardless of
//!    the self-registration toggle (TR-04-004).
//! 3. Otherwise, if self-registration is enabled → create with the
//!    **least-privilege** role (TR-04-012).
//! 4. Otherwise (toggle off, the default), if an admin has **allow-listed** the
//!    email → create with the least-privilege role (TR-04-013).
//! 5. Otherwise → **deny**, creating no account (TR-04-013).

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, DatabaseConnection, PaginatorTrait};
use uuid::Uuid;

use crate::models::{
    allowlisted_emails::{self, Model as AllowlistedEmail},
    role::Role,
    users,
};

/// The provisioning decision for a not-yet-seen (or seen) identity. Pure of
/// I/O so it can be exhaustively unit-tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The email is already provisioned; return the existing user.
    ReturnExisting,
    /// Bootstrap: create this user as the administrator.
    CreateAdmin,
    /// Create this user with the least-privilege role.
    CreateLeastPrivilege,
    /// Deny: not permitted to onboard; create nothing.
    Deny,
}

impl Decision {
    /// The role a create-decision assigns (`None` for non-creating decisions).
    #[must_use]
    pub fn role(self) -> Option<Role> {
        match self {
            Decision::CreateAdmin => Some(Role::Admin),
            Decision::CreateLeastPrivilege => Some(Role::LEAST_PRIVILEGE),
            Decision::ReturnExisting | Decision::Deny => None,
        }
    }
}

/// The core onboarding rule (pure). See the module docs for the ordered rules.
#[must_use]
pub fn decide(
    exists: bool,
    is_first_user: bool,
    self_registration_enabled: bool,
    allowlisted: bool,
) -> Decision {
    if exists {
        return Decision::ReturnExisting;
    }
    if is_first_user {
        return Decision::CreateAdmin;
    }
    if self_registration_enabled || allowlisted {
        return Decision::CreateLeastPrivilege;
    }
    Decision::Deny
}

/// Why provisioning failed.
#[derive(Debug, thiserror::Error)]
pub enum ProvisionError {
    /// The identity is not permitted to onboard (toggle off + not allow-listed).
    /// Maps to `403 Forbidden`.
    #[error("email `{0}` is not permitted to onboard")]
    NotAllowed(String),
    /// A required claim (email) was missing from the token.
    #[error("token carried no usable email claim")]
    MissingEmail,
    /// Database error.
    #[error(transparent)]
    Db(#[from] loco_rs::model::ModelError),
}

/// Inputs to a provisioning attempt, gathered from the validated token.
#[derive(Debug, Clone)]
pub struct ProvisionInput {
    /// The OIDC `email` claim (identity key).
    pub email: String,
    /// Optional display name.
    pub name: Option<String>,
    /// Startup self-registration toggle value (TR-04-011).
    pub self_registration_enabled: bool,
}

/// Provision (or resolve) a user for an authenticated identity, applying the
/// bootstrap / toggle / allow-list rules.
///
/// # Errors
/// [`ProvisionError::NotAllowed`] when onboarding is denied, or DB errors.
pub async fn provision(
    db: &DatabaseConnection,
    input: &ProvisionInput,
) -> Result<users::Model, ProvisionError> {
    let email = allowlisted_emails::normalize(&input.email);
    if email.is_empty() {
        return Err(ProvisionError::MissingEmail);
    }

    let existing = users::Model::find_by_email(db, &email).await.ok();
    let exists = existing.is_some();
    let is_first_user = users::Entity::find()
        .count(db)
        .await
        .map_err(loco_rs::model::ModelError::from)?
        == 0;
    let allowlisted = AllowlistedEmail::is_allowlisted(db, &email).await?;

    let decision = decide(
        exists,
        is_first_user,
        input.self_registration_enabled,
        allowlisted,
    );

    match decision {
        Decision::ReturnExisting => Ok(existing.expect("exists implies Some")),
        Decision::CreateAdmin | Decision::CreateLeastPrivilege => {
            let role = decision.role().expect("create decision has a role");
            create_user(db, &email, input.name.as_deref(), role).await
        }
        Decision::Deny => Err(ProvisionError::NotAllowed(email)),
    }
}

/// Insert a new OIDC-backed user. The password column is set to an unusable
/// sentinel — the backend never authenticates passwords locally (Rauthy owns
/// credentials); `pid`/`api_key` are filled by the model's `before_save`.
async fn create_user(
    db: &DatabaseConnection,
    email: &str,
    name: Option<&str>,
    role: Role,
) -> Result<users::Model, ProvisionError> {
    // `name` must satisfy the model validator (>= 2 chars); fall back to the
    // email local-part, then the email.
    let name = name
        .map(str::to_string)
        .filter(|n| n.trim().len() >= 2)
        .unwrap_or_else(|| {
            let local = email.split('@').next().unwrap_or(email);
            if local.len() >= 2 {
                local.to_string()
            } else {
                email.to_string()
            }
        });

    let model = users::ActiveModel {
        email: ActiveValue::set(email.to_string()),
        name: ActiveValue::set(name),
        role: ActiveValue::set(role.as_str().to_string()),
        password: ActiveValue::set(format!("!oidc-no-local-password!{}", Uuid::new_v4())),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| ProvisionError::Db(loco_rs::model::ModelError::from(e)))?;
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_user_returns_existing_regardless_of_other_flags() {
        for first in [true, false] {
            for reg in [true, false] {
                for al in [true, false] {
                    assert_eq!(decide(true, first, reg, al), Decision::ReturnExisting);
                }
            }
        }
    }

    #[test]
    fn first_user_is_admin_even_with_toggle_off_and_not_allowlisted() {
        assert_eq!(decide(false, true, false, false), Decision::CreateAdmin);
        // Bootstrap wins over the toggle entirely.
        assert_eq!(decide(false, true, true, true), Decision::CreateAdmin);
        assert_eq!(decide(false, true, false, false).role(), Some(Role::Admin));
    }

    #[test]
    fn self_registration_enabled_creates_least_privilege() {
        let d = decide(false, false, true, false);
        assert_eq!(d, Decision::CreateLeastPrivilege);
        assert_eq!(d.role(), Some(Role::User));
    }

    #[test]
    fn toggle_off_denies_unless_allowlisted() {
        // Not allow-listed → denied, no account.
        assert_eq!(decide(false, false, false, false), Decision::Deny);
        assert_eq!(decide(false, false, false, false).role(), None);
        // Allow-listed → least privilege.
        assert_eq!(
            decide(false, false, false, true),
            Decision::CreateLeastPrivilege
        );
    }
}
