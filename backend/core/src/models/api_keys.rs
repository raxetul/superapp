//! Service-to-service API key model (TR-04-009).
//!
//! Keys have the form `sk_<prefix>_<secret>`. `prefix` is a non-secret lookup
//! handle; only the SHA-256 hash of the full plaintext is persisted. The
//! plaintext is returned exactly once, at creation. Revocation stamps
//! `revoked_at`; a revoked key never authenticates.

use chrono::Utc;
use loco_rs::prelude::*;
use rand::{distributions::Alphanumeric, Rng};
use sea_orm::{ActiveValue, DatabaseConnection};
use sha2::{Digest, Sha256};

pub use super::_entities::api_keys::{ActiveModel, Column, Entity, Model};

const KEY_SCHEME: &str = "sk";
const PREFIX_LEN: usize = 8;
const SECRET_LEN: usize = 32;

/// SHA-256 hex digest of a plaintext key. Deterministic, so lookups can match
/// a presented key against the stored hash.
#[must_use]
pub fn hash_key(plaintext: &str) -> String {
    let digest = Sha256::digest(plaintext.as_bytes());
    hex_lower(&digest)
}

/// Extract the non-secret `prefix` from a well-formed `sk_<prefix>_<secret>`
/// key. Returns `None` if the shape is wrong.
#[must_use]
pub fn parse_prefix(plaintext: &str) -> Option<String> {
    let mut parts = plaintext.splitn(3, '_');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(KEY_SCHEME), Some(prefix), Some(secret))
            if !prefix.is_empty() && !secret.is_empty() =>
        {
            Some(prefix.to_string())
        }
        _ => None,
    }
}

fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

impl Model {
    /// Whether this key is currently active (not revoked).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }

    /// Mint a new API key named `name`. Returns the persisted record and the
    /// **plaintext** key (shown once — never recoverable afterwards).
    ///
    /// # Errors
    /// On a DB query error.
    pub async fn create(db: &DatabaseConnection, name: &str) -> ModelResult<(Model, String)> {
        let prefix = random_token(PREFIX_LEN);
        let secret = random_token(SECRET_LEN);
        let plaintext = format!("{KEY_SCHEME}_{prefix}_{secret}");
        let model = ActiveModel {
            name: ActiveValue::set(name.to_string()),
            prefix: ActiveValue::set(prefix),
            key_hash: ActiveValue::set(hash_key(&plaintext)),
            revoked_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok((model, plaintext))
    }

    /// Find an active key by its lookup prefix.
    async fn find_active_by_prefix(db: &DatabaseConnection, prefix: &str) -> ModelResult<Model> {
        let key = Entity::find()
            .filter(Column::Prefix.eq(prefix))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        if key.is_active() {
            Ok(key)
        } else {
            Err(ModelError::EntityNotFound)
        }
    }

    /// Authenticate a presented plaintext key: parse its prefix, look up the
    /// active record, and constant-time-compare the stored hash. Any failure
    /// (malformed, unknown, revoked, hash mismatch) is [`ModelError::EntityNotFound`].
    ///
    /// # Errors
    /// [`ModelError::EntityNotFound`] when the key is invalid or revoked;
    /// other variants on a DB error.
    pub async fn authenticate(db: &DatabaseConnection, plaintext: &str) -> ModelResult<Model> {
        let prefix = parse_prefix(plaintext).ok_or(ModelError::EntityNotFound)?;
        let key = Self::find_active_by_prefix(db, &prefix).await?;
        if constant_time_eq(key.key_hash.as_bytes(), hash_key(plaintext).as_bytes()) {
            Ok(key)
        } else {
            Err(ModelError::EntityNotFound)
        }
    }

    /// Revoke the key with `prefix`, stamping `revoked_at`. Idempotent.
    ///
    /// # Errors
    /// [`ModelError::EntityNotFound`] if no such key; other variants on a DB error.
    pub async fn revoke(db: &DatabaseConnection, prefix: &str) -> ModelResult<Model> {
        let key = Entity::find()
            .filter(Column::Prefix.eq(prefix))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        if key.revoked_at.is_some() {
            return Ok(key);
        }
        let mut active = key.into_active_model();
        active.revoked_at = ActiveValue::set(Some(Utc::now().into()));
        Ok(active.update(db).await?)
    }
}

/// Length-independent constant-time byte comparison (both operands are equal
/// fixed length here — SHA-256 hex — so this only guards content timing).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic_and_sha256_hex() {
        let h = hash_key("sk_abc_def");
        assert_eq!(h, hash_key("sk_abc_def"));
        assert_eq!(h.len(), 64); // 32 bytes hex
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(h, hash_key("sk_abc_deg"));
    }

    #[test]
    fn parse_prefix_accepts_well_formed_and_rejects_junk() {
        assert_eq!(
            parse_prefix("sk_ABC12345_secretpart").as_deref(),
            Some("ABC12345")
        );
        assert_eq!(parse_prefix("nope"), None);
        assert_eq!(parse_prefix("sk__secret"), None);
        assert_eq!(parse_prefix("sk_prefix_"), None);
        assert_eq!(parse_prefix("bearer_x_y"), None);
    }

    #[test]
    fn constant_time_eq_matches_semantics() {
        assert!(constant_time_eq(b"abcd", b"abcd"));
        assert!(!constant_time_eq(b"abcd", b"abce"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
