//! Admin email allow-list model (TR-04-013).
//!
//! When self-registration is disabled (default), only emails present here may
//! authenticate. Email is normalized to lowercase so the allow-list check is
//! case-insensitive (OIDC `email` claims vary in case).

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, DatabaseConnection};

pub use super::_entities::allowlisted_emails::{ActiveModel, Column, Entity, Model};

/// Normalize an email for allow-list storage/comparison.
#[must_use]
pub fn normalize(email: &str) -> String {
    email.trim().to_lowercase()
}

impl Model {
    /// Whether `email` is on the allow-list (case-insensitive).
    ///
    /// # Errors
    /// On a DB query error.
    pub async fn is_allowlisted(db: &DatabaseConnection, email: &str) -> ModelResult<bool> {
        let found = Entity::find()
            .filter(Column::Email.eq(normalize(email)))
            .one(db)
            .await?;
        Ok(found.is_some())
    }

    /// Add `email` to the allow-list, idempotently. Returns the existing row if
    /// the email is already present.
    ///
    /// # Errors
    /// On a DB query error.
    pub async fn add(
        db: &DatabaseConnection,
        email: &str,
        added_by: Option<&str>,
    ) -> ModelResult<Model> {
        let email = normalize(email);
        if let Some(existing) = Entity::find()
            .filter(Column::Email.eq(email.clone()))
            .one(db)
            .await?
        {
            return Ok(existing);
        }
        let model = ActiveModel {
            email: ActiveValue::set(email),
            added_by: ActiveValue::set(added_by.map(str::to_string)),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(model)
    }

    /// Remove `email` from the allow-list. Returns the number of rows removed
    /// (0 if it was not present).
    ///
    /// # Errors
    /// On a DB query error.
    pub async fn remove(db: &DatabaseConnection, email: &str) -> ModelResult<u64> {
        let res = Entity::delete_many()
            .filter(Column::Email.eq(normalize(email)))
            .exec(db)
            .await?;
        Ok(res.rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lowercases_and_trims() {
        assert_eq!(normalize("  Alice@Example.COM "), "alice@example.com");
    }
}
