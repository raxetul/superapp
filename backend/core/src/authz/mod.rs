//! Policy-based authorization (P4): Cedar enforcement point, policy set,
//! entity provider, and audit.
//!
//! - [`engine`] — loads the Cedar [`engine::PolicyEngine`] and evaluates
//!   `is_authorized` (TR-04-005, TR-04-006).
//! - [`entities`] — materializes principals/resources from the DB with a TTL
//!   cache (TR-04-007).
//! - [`audit`] — structured authorization audit logging (TR-04-008).
//!
//! [`Enforcer`] is the central enforcement point wiring these together: fetch
//! entities → evaluate → audit → decision.

pub mod audit;
pub mod engine;
pub mod entities;

use std::sync::Arc;

use cedar_policy::Context;

use self::audit::AuthzAuditEntry;
use self::engine::{AuthzDecision, AuthzError, PolicyEngine};
use self::entities::EntityProvider;

/// The central authorization enforcement point (TR-04-005). Holds the loaded
/// policy engine and an injected entity provider.
#[derive(Clone)]
pub struct Enforcer {
    engine: PolicyEngine,
    entities: Arc<dyn EntityProvider>,
}

impl Enforcer {
    /// Wire an engine to an entity provider.
    #[must_use]
    pub fn new(engine: PolicyEngine, entities: Arc<dyn EntityProvider>) -> Self {
        Self { engine, entities }
    }

    /// Enforce a decision for `principal`/`action`/`resource`, auditing the
    /// result. Callers map a denied decision to `403`.
    ///
    /// # Errors
    /// [`AuthzError`] on malformed UIDs or entity/DB failures.
    pub async fn enforce(
        &self,
        principal: &str,
        action: &str,
        resource: &str,
        context: Context,
    ) -> Result<AuthzDecision, AuthzError> {
        let entities = self.entities.entities().await?;
        let decision = self
            .engine
            .is_authorized(principal, action, resource, context, &entities)?;
        AuthzAuditEntry::new(principal, action, resource, &decision).log();
        Ok(decision)
    }
}
