//! Coarse principal roles (P4).
//!
//! The system has exactly two roles: [`Role::Admin`] and the least-privilege
//! [`Role::User`]. Roles are stored as a string on `users.role` and surfaced to
//! Cedar as principal group membership (TR-04-004, TR-04-012).

use serde::{Deserialize, Serialize};
use std::fmt;

/// A user's coarse authorization role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Full administrative privileges (first user / bootstrap).
    Admin,
    /// The least-privilege default role.
    User,
}

impl Role {
    /// The lowest-privilege role, assigned to self-onboarded users
    /// (TR-04-012).
    pub const LEAST_PRIVILEGE: Role = Role::User;

    /// The stored/wire string form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::User => "user",
        }
    }

    /// Parse a stored role string. Unknown values fall back to the
    /// least-privilege role — the DB column can never widen privilege by
    /// accident.
    #[must_use]
    pub fn from_stored(s: &str) -> Role {
        match s {
            "admin" => Role::Admin,
            _ => Role::User,
        }
    }

    /// Whether this role is the administrator role.
    #[must_use]
    pub fn is_admin(self) -> bool {
        matches!(self, Role::Admin)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_stored_string() {
        assert_eq!(Role::from_stored(Role::Admin.as_str()), Role::Admin);
        assert_eq!(Role::from_stored(Role::User.as_str()), Role::User);
    }

    #[test]
    fn unknown_role_falls_back_to_least_privilege() {
        assert_eq!(Role::from_stored("superuser"), Role::User);
        assert_eq!(Role::from_stored(""), Role::User);
        assert_eq!(Role::LEAST_PRIVILEGE, Role::User);
        assert!(!Role::LEAST_PRIVILEGE.is_admin());
    }

    #[test]
    fn admin_is_admin() {
        assert!(Role::Admin.is_admin());
    }
}
