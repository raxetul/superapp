//! Authorization audit logging (TR-04-008).
//!
//! Every authorization decision emits a structured audit record carrying the
//! principal, action, resource, the decision, and the ids of the policies that
//! determined it.

use serde::Serialize;

use super::engine::AuthzDecision;

/// A single authorization audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthzAuditEntry {
    /// Principal UID (e.g. `User::"alice@example.com"`).
    pub principal: String,
    /// Action UID (e.g. `Action::"admin.access"`).
    pub action: String,
    /// Resource UID (e.g. `AdminPanel::"main"`).
    pub resource: String,
    /// `"Allow"` or `"Deny"`.
    pub decision: &'static str,
    /// Ids of the determining policies.
    pub policies: Vec<String>,
}

impl AuthzAuditEntry {
    /// Build an audit entry from a decision.
    #[must_use]
    pub fn new(principal: &str, action: &str, resource: &str, decision: &AuthzDecision) -> Self {
        Self {
            principal: principal.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            decision: if decision.allowed { "Allow" } else { "Deny" },
            policies: decision.determining_policies.clone(),
        }
    }

    /// Emit the entry to the structured log (info for allow, warn for deny).
    pub fn log(&self) {
        if self.decision == "Allow" {
            tracing::info!(
                target: "authz.audit",
                principal = %self.principal,
                action = %self.action,
                resource = %self.resource,
                decision = self.decision,
                policies = ?self.policies,
                "authorization decision",
            );
        } else {
            tracing::warn!(
                target: "authz.audit",
                principal = %self.principal,
                action = %self.action,
                resource = %self.resource,
                decision = self.decision,
                policies = ?self.policies,
                "authorization decision",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_captures_decision_and_policies() {
        let allow = AuthzDecision {
            allowed: true,
            determining_policies: vec!["admin-full-access".into()],
        };
        let e = AuthzAuditEntry::new(
            "User::\"a@b.com\"",
            "Action::\"admin.access\"",
            "AdminPanel::\"main\"",
            &allow,
        );
        assert_eq!(e.decision, "Allow");
        assert_eq!(e.policies, vec!["admin-full-access".to_string()]);

        let deny = AuthzDecision {
            allowed: false,
            determining_policies: vec![],
        };
        let e = AuthzAuditEntry::new(
            "User::\"x\"",
            "Action::\"admin.access\"",
            "AdminPanel::\"main\"",
            &deny,
        );
        assert_eq!(e.decision, "Deny");
        assert!(e.policies.is_empty());
    }
}
