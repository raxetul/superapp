//! Redis-backed refresh-token store with rotation (TR-04-003).
//!
//! The backend is a confidential OIDC client: it never hands a raw Rauthy
//! refresh token to end clients. Instead it stores the Rauthy refresh token in
//! Redis under an **opaque handle** (with a TTL) and gives the client the
//! handle. On refresh the handle is exchanged for a new access token and a new
//! refresh token, and the store **rotates**: the old handle is deleted and a
//! new one issued. Presenting a rotated/revoked handle therefore fails —
//! reuse detection falls out of rotation.

use async_trait::async_trait;
use rand::{distributions::Alphanumeric, Rng};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

const KEY_PREFIX: &str = "superapp:refresh:";
const HANDLE_LEN: usize = 40;

/// What we persist behind a refresh handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshRecord {
    /// The SuperApp user this session belongs to (email — the identity key).
    pub user_id: String,
    /// The upstream Rauthy refresh token, used to mint the next access token.
    pub refresh_token: String,
}

/// Why a refresh operation failed.
#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    /// The handle is unknown, expired, or already rotated (reuse) — reject.
    #[error("refresh handle is unknown, expired, or already used")]
    UnknownHandle,
    /// A stored record could not be decoded.
    #[error("stored refresh record is malformed: {0}")]
    Malformed(String),
    /// Underlying Redis error.
    #[error(transparent)]
    Redis(#[from] redis::RedisError),
}

/// The refresh-token store seam. Higher layers depend on this abstraction and
/// receive it by injection; production wires [`RefreshStore`] (Redis) and tests
/// inject an in-memory fake (see [`InMemoryRefreshStore`]).
#[async_trait]
pub trait RefreshTokens: Send + Sync {
    /// Store `record` under a fresh opaque handle and return the handle.
    async fn issue(&self, record: &RefreshRecord) -> Result<String, RefreshError>;
    /// Look up the record behind `handle`, if any.
    async fn get(&self, handle: &str) -> Result<Option<RefreshRecord>, RefreshError>;
    /// Consume `old_handle` and issue a new one carrying `new_record`.
    async fn rotate(
        &self,
        old_handle: &str,
        new_record: &RefreshRecord,
    ) -> Result<String, RefreshError>;
    /// Revoke `handle` (idempotent); returns whether one was removed.
    async fn revoke(&self, handle: &str) -> Result<bool, RefreshError>;
}

/// A refresh-token store backed by a pooled Redis connection.
#[derive(Clone)]
pub struct RefreshStore {
    conn: ConnectionManager,
    ttl_secs: u64,
}

fn key(handle: &str) -> String {
    format!("{KEY_PREFIX}{handle}")
}

fn new_handle() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(HANDLE_LEN)
        .map(char::from)
        .collect()
}

impl RefreshStore {
    /// Connect to Redis and build a store whose handles expire after
    /// `ttl_secs`.
    ///
    /// # Errors
    /// When the URL is invalid or Redis is unreachable.
    pub async fn connect(url: &str, ttl_secs: u64) -> Result<Self, RefreshError> {
        let client = redis::Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self { conn, ttl_secs })
    }

    async fn put(&self, handle: &str, record: &RefreshRecord) -> Result<(), RefreshError> {
        let payload =
            serde_json::to_string(record).map_err(|e| RefreshError::Malformed(e.to_string()))?;
        let mut conn = self.conn.clone();
        conn.set_ex::<_, _, ()>(key(handle), payload, self.ttl_secs)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl RefreshTokens for RefreshStore {
    async fn issue(&self, record: &RefreshRecord) -> Result<String, RefreshError> {
        let handle = new_handle();
        self.put(&handle, record).await?;
        Ok(handle)
    }

    async fn get(&self, handle: &str) -> Result<Option<RefreshRecord>, RefreshError> {
        let mut conn = self.conn.clone();
        let payload: Option<String> = conn.get(key(handle)).await?;
        match payload {
            None => Ok(None),
            Some(p) => serde_json::from_str(&p)
                .map(Some)
                .map_err(|e| RefreshError::Malformed(e.to_string())),
        }
    }

    async fn rotate(
        &self,
        old_handle: &str,
        new_record: &RefreshRecord,
    ) -> Result<String, RefreshError> {
        let mut conn = self.conn.clone();
        // Consume the old handle atomically. DEL returns the number removed;
        // 0 means it was absent → reuse/expiry/revocation.
        let removed: i64 = conn.del(key(old_handle)).await?;
        if removed == 0 {
            return Err(RefreshError::UnknownHandle);
        }
        let handle = new_handle();
        self.put(&handle, new_record).await?;
        Ok(handle)
    }

    async fn revoke(&self, handle: &str) -> Result<bool, RefreshError> {
        let mut conn = self.conn.clone();
        let removed: i64 = conn.del(key(handle)).await?;
        Ok(removed > 0)
    }
}

/// In-memory refresh store for tests and for booting without Redis. Satisfies
/// the same [`RefreshTokens`] seam so higher layers are exercised hermetically.
#[derive(Default)]
pub struct InMemoryRefreshStore {
    inner: std::sync::Mutex<std::collections::HashMap<String, RefreshRecord>>,
}

#[async_trait]
impl RefreshTokens for InMemoryRefreshStore {
    async fn issue(&self, record: &RefreshRecord) -> Result<String, RefreshError> {
        let handle = new_handle();
        self.inner
            .lock()
            .unwrap()
            .insert(handle.clone(), record.clone());
        Ok(handle)
    }

    async fn get(&self, handle: &str) -> Result<Option<RefreshRecord>, RefreshError> {
        Ok(self.inner.lock().unwrap().get(handle).cloned())
    }

    async fn rotate(
        &self,
        old_handle: &str,
        new_record: &RefreshRecord,
    ) -> Result<String, RefreshError> {
        let mut map = self.inner.lock().unwrap();
        if map.remove(old_handle).is_none() {
            return Err(RefreshError::UnknownHandle);
        }
        let handle = new_handle();
        map.insert(handle.clone(), new_record.clone());
        Ok(handle)
    }

    async fn revoke(&self, handle: &str) -> Result<bool, RefreshError> {
        Ok(self.inner.lock().unwrap().remove(handle).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests exercise the live Redis on localhost:6379 (present in this
    // environment). Each uses freshly-minted random handles, so parallel runs
    // never collide.
    const URL: &str = "redis://localhost:6379";

    async fn store() -> RefreshStore {
        RefreshStore::connect(URL, 60)
            .await
            .expect("redis reachable")
    }

    fn rec(token: &str) -> RefreshRecord {
        RefreshRecord {
            user_id: "alice@example.com".into(),
            refresh_token: token.into(),
        }
    }

    #[tokio::test]
    async fn issue_then_get_returns_record() {
        let s = store().await;
        let h = s.issue(&rec("rt-1")).await.unwrap();
        let got = s.get(&h).await.unwrap().expect("record present");
        assert_eq!(got.refresh_token, "rt-1");
        assert_eq!(got.user_id, "alice@example.com");
        s.revoke(&h).await.unwrap();
    }

    #[tokio::test]
    async fn rotate_issues_new_handle_and_invalidates_old() {
        let s = store().await;
        let h1 = s.issue(&rec("rt-1")).await.unwrap();
        let h2 = s.rotate(&h1, &rec("rt-2")).await.unwrap();
        assert_ne!(h1, h2);
        // Old handle is gone (rotation consumed it).
        assert!(s.get(&h1).await.unwrap().is_none());
        // New handle carries the rotated refresh token.
        assert_eq!(s.get(&h2).await.unwrap().unwrap().refresh_token, "rt-2");
        s.revoke(&h2).await.unwrap();
    }

    #[tokio::test]
    async fn reused_handle_is_rejected() {
        let s = store().await;
        let h1 = s.issue(&rec("rt-1")).await.unwrap();
        let _h2 = s.rotate(&h1, &rec("rt-2")).await.unwrap();
        // Second rotation of the same (already-consumed) handle must fail.
        let err = s.rotate(&h1, &rec("rt-3")).await.unwrap_err();
        assert!(matches!(err, RefreshError::UnknownHandle), "got {err:?}");
        s.revoke(&_h2).await.unwrap();
    }

    #[tokio::test]
    async fn revoke_removes_handle() {
        let s = store().await;
        let h = s.issue(&rec("rt-1")).await.unwrap();
        assert!(s.revoke(&h).await.unwrap());
        assert!(s.get(&h).await.unwrap().is_none());
        // Revoking again is a no-op.
        assert!(!s.revoke(&h).await.unwrap());
    }
}
