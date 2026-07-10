//! Cedar entity provider (TR-04-007).
//!
//! Materializes Cedar [`Entities`] (principals with their `role`/email
//! attributes and `Role` group membership, plus fixed resources) from the
//! database. Because materialization hits the DB on every authorization
//! request, results are cached for a short TTL; a role/attribute change becomes
//! visible once the TTL lapses.
//!
//! Per the DI rule the cache takes its [`Clock`] by injection, so the TTL
//! behaviour is tested deterministically with a fake clock and a fake inner
//! provider — no DB, no wall-clock.

use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;
use cedar_policy::Entities;
use loco_rs::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde_json::{json, Value};

use crate::models::users;

/// A monotonic clock, injected so caches can be tested without wall-time.
pub trait Clock: Send + Sync {
    /// Milliseconds elapsed since some fixed, process-stable epoch.
    fn now_millis(&self) -> u64;
}

/// Production clock backed by a monotonic [`Instant`] captured at construction.
pub struct SystemClock {
    base: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self {
            base: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn now_millis(&self) -> u64 {
        self.base.elapsed().as_millis() as u64
    }
}

/// Build the fixed (non-user) entities: the two role groups and the admin
/// panel resource.
fn fixed_entities() -> Vec<Value> {
    vec![
        json!({"uid": {"type": "Role", "id": "admin"}, "attrs": {}, "parents": []}),
        json!({"uid": {"type": "Role", "id": "user"}, "attrs": {}, "parents": []}),
        json!({"uid": {"type": "AdminPanel", "id": "main"}, "attrs": {}, "parents": []}),
    ]
}

/// The Cedar entity JSON for a single user (principal). Pure — unit-testable.
#[must_use]
pub fn user_entity_json(user: &users::Model) -> Value {
    let role = user.role().as_str();
    json!({
        "uid": {"type": "User", "id": user.email},
        "attrs": {"email": user.email, "role": role},
        "parents": [{"type": "Role", "id": role}],
    })
}

/// Assemble the full entity JSON array from a set of users.
#[must_use]
pub fn entities_json(users: &[users::Model]) -> Value {
    let mut all = fixed_entities();
    all.extend(users.iter().map(user_entity_json));
    Value::Array(all)
}

/// Parse an entity JSON array into Cedar [`Entities`].
///
/// # Errors
/// [`super::engine::AuthzError::BadRequest`] if the JSON is not valid entities.
pub fn parse_entities(value: Value) -> Result<Entities, super::engine::AuthzError> {
    Entities::from_json_value(value, None)
        .map_err(|e| super::engine::AuthzError::BadRequest(format!("entities: {e}")))
}

/// The entity-provider seam. Injected into the enforcement layer.
#[async_trait]
pub trait EntityProvider: Send + Sync {
    /// The current entity JSON (before Cedar parsing), possibly cached.
    async fn entities_json(&self) -> Result<Value, super::engine::AuthzError>;

    /// The parsed Cedar entities.
    async fn entities(&self) -> Result<Entities, super::engine::AuthzError> {
        parse_entities(self.entities_json().await?)
    }
}

/// Materializes entities from the database (all users + fixed resources).
pub struct DbEntityProvider {
    db: DatabaseConnection,
}

impl DbEntityProvider {
    /// Wrap a database connection.
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl EntityProvider for DbEntityProvider {
    async fn entities_json(&self) -> Result<Value, super::engine::AuthzError> {
        let users = users::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| super::engine::AuthzError::BadRequest(format!("db: {e}")))?;
        Ok(entities_json(&users))
    }
}

/// Wraps an inner [`EntityProvider`] with a TTL cache over the (expensive) DB
/// materialization.
pub struct CachedEntityProvider<P: EntityProvider> {
    inner: P,
    clock: std::sync::Arc<dyn Clock>,
    ttl_millis: u64,
    cached: Mutex<Option<(u64, Value)>>, // (stamped_at_millis, value)
}

impl<P: EntityProvider> CachedEntityProvider<P> {
    /// Wrap `inner`, caching materializations for `ttl_millis`.
    #[must_use]
    pub fn new(inner: P, clock: std::sync::Arc<dyn Clock>, ttl_millis: u64) -> Self {
        Self {
            inner,
            clock,
            ttl_millis,
            cached: Mutex::new(None),
        }
    }
}

#[async_trait]
impl<P: EntityProvider> EntityProvider for CachedEntityProvider<P> {
    async fn entities_json(&self) -> Result<Value, super::engine::AuthzError> {
        let now = self.clock.now_millis();
        // Fast path: fresh cache.
        if let Some((stamped, value)) = self.cached.lock().unwrap().as_ref() {
            if now.saturating_sub(*stamped) < self.ttl_millis {
                return Ok(value.clone());
            }
        }
        // Refresh from the inner provider.
        let fresh = self.inner.entities_json().await?;
        *self.cached.lock().unwrap() = Some((now, fresh.clone()));
        Ok(fresh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    /// Manually-advanced clock for deterministic TTL tests.
    #[derive(Default)]
    struct FakeClock(AtomicU64);
    impl FakeClock {
        fn advance(&self, ms: u64) {
            self.0.fetch_add(ms, Ordering::SeqCst);
        }
    }
    impl Clock for FakeClock {
        fn now_millis(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    /// Inner provider whose output can be swapped to simulate a DB change.
    struct FakeProvider {
        value: Mutex<Value>,
        calls: AtomicU64,
    }
    #[async_trait]
    impl EntityProvider for FakeProvider {
        async fn entities_json(&self) -> Result<Value, super::super::engine::AuthzError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.value.lock().unwrap().clone())
        }
    }

    #[test]
    fn user_entity_json_carries_role_and_group_membership() {
        // Build a user model with the admin role via serde (avoids DB).
        let model: users::Model = serde_json::from_value(json!({
            "created_at": "2023-11-12T12:34:56.789Z",
            "updated_at": "2023-11-12T12:34:56.789Z",
            "id": 1,
            "pid": "11111111-1111-1111-1111-111111111111",
            "email": "boss@corp.example",
            "password": "x",
            "api_key": "k",
            "name": "Boss",
            "role": "admin"
        }))
        .unwrap();
        let v = user_entity_json(&model);
        assert_eq!(v["uid"]["id"], json!("boss@corp.example"));
        assert_eq!(v["attrs"]["role"], json!("admin"));
        assert_eq!(v["parents"][0], json!({"type":"Role","id":"admin"}));
    }

    #[tokio::test]
    async fn cache_serves_stale_within_ttl_then_refreshes() {
        let clock = Arc::new(FakeClock::default());
        let inner = FakeProvider {
            value: Mutex::new(json!(["v1"])),
            calls: AtomicU64::new(0),
        };
        // borrow the inner's fields after moving? Keep a handle via Arc instead.
        let inner = Arc::new(inner);
        let provider = CachedEntityProvider::new(ArcProvider(inner.clone()), clock.clone(), 1000);

        // First call materializes and caches.
        assert_eq!(provider.entities_json().await.unwrap(), json!(["v1"]));
        assert_eq!(inner.calls.load(Ordering::SeqCst), 1);

        // Underlying data changes, but within TTL the cache serves the old value.
        *inner.value.lock().unwrap() = json!(["v2"]);
        clock.advance(500);
        assert_eq!(provider.entities_json().await.unwrap(), json!(["v1"]));
        assert_eq!(
            inner.calls.load(Ordering::SeqCst),
            1,
            "no refresh within TTL"
        );

        // Past the TTL, the change is picked up.
        clock.advance(600); // total 1100 > 1000
        assert_eq!(provider.entities_json().await.unwrap(), json!(["v2"]));
        assert_eq!(inner.calls.load(Ordering::SeqCst), 2, "refreshed after TTL");
    }

    /// Adapts an `Arc<P>` into an `EntityProvider` so the test can retain a
    /// handle to the inner provider's counters.
    struct ArcProvider(Arc<FakeProvider>);
    #[async_trait]
    impl EntityProvider for ArcProvider {
        async fn entities_json(&self) -> Result<Value, super::super::engine::AuthzError> {
            self.0.entities_json().await
        }
    }
}
