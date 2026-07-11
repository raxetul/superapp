//! The real-time event envelope (TR-06-002).
//!
//! Events serialize as `{ type, data, timestamp, user_id }`. `user_id` targets
//! a single user, or is `null` for a broadcast to all subscribers.

use serde::{Deserialize, Serialize};

/// Well-known domain event types (TR-06-003).
pub mod types {
    pub const USER_CREATED: &str = "user.created";
    pub const MODULE_LOADED: &str = "module.loaded";
    pub const CONFIG_UPDATED: &str = "config.updated";
}

/// A real-time event delivered over SSE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Event type, e.g. `user.created`.
    #[serde(rename = "type")]
    pub type_: String,
    /// Arbitrary event payload.
    pub data: serde_json::Value,
    /// RFC 3339 timestamp.
    pub timestamp: String,
    /// Target user (identity key / email), or `None` for a broadcast.
    pub user_id: Option<String>,
}

impl EventEnvelope {
    /// A broadcast event (delivered to every subscriber).
    #[must_use]
    pub fn broadcast(type_: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            type_: type_.into(),
            data,
            timestamp: now_rfc3339(),
            user_id: None,
        }
    }

    /// An event targeted at a single user.
    #[must_use]
    pub fn targeted(
        type_: impl Into<String>,
        data: serde_json::Value,
        user_id: impl Into<String>,
    ) -> Self {
        Self {
            type_: type_.into(),
            data,
            timestamp: now_rfc3339(),
            user_id: Some(user_id.into()),
        }
    }

    /// Whether this event should be delivered to a subscriber authenticated as
    /// `subscriber` — true for broadcasts and for events targeted at them.
    #[must_use]
    pub fn is_for(&self, subscriber: &str) -> bool {
        match &self.user_id {
            None => true,
            Some(target) => target == subscriber,
        }
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn broadcast_is_for_everyone() {
        let e = EventEnvelope::broadcast(types::CONFIG_UPDATED, json!({"k":"v"}));
        assert!(e.user_id.is_none());
        assert!(e.is_for("alice@x"));
        assert!(e.is_for("bob@x"));
    }

    #[test]
    fn targeted_is_only_for_that_user() {
        let e = EventEnvelope::targeted(types::USER_CREATED, json!({}), "alice@x");
        assert!(e.is_for("alice@x"));
        assert!(!e.is_for("bob@x"));
    }

    #[test]
    fn serializes_with_type_key() {
        let v = serde_json::to_value(EventEnvelope::broadcast("module.loaded", json!({"id":1})))
            .unwrap();
        assert_eq!(v["type"], json!("module.loaded"));
        assert!(v["user_id"].is_null());
        assert!(v["timestamp"].is_string());
    }
}
