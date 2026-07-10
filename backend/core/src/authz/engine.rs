//! Cedar policy enforcement point (TR-04-005, TR-04-006).
//!
//! [`PolicyEngine`] loads a Cedar [`PolicySet`] from `.cedar` files on disk (so
//! editing a policy changes decisions without recompiling — TR-04-006) and
//! evaluates `is_authorized(principal, action, resource, context)`. A `Deny`
//! maps to `403` at the HTTP boundary. The determining policy ids are returned
//! so the caller can audit them (TR-04-008).

use std::path::Path;
use std::str::FromStr;

use cedar_policy::{
    Authorizer, Context, Decision, Entities, EntityUid, PolicySet, Request, Schema,
};

/// Authorization errors distinct from a plain `Deny`.
#[derive(Debug, thiserror::Error)]
pub enum AuthzError {
    /// A `.cedar` policy file (or the concatenation) failed to parse.
    #[error("failed to parse Cedar policies: {0}")]
    Policy(String),
    /// The policies directory could not be read.
    #[error("failed to read policies from `{0}`: {1}")]
    Io(String, String),
    /// A principal/action/resource UID or request was malformed.
    #[error("malformed authorization request: {0}")]
    BadRequest(String),
}

/// The result of an authorization check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthzDecision {
    /// Whether the action is permitted.
    pub allowed: bool,
    /// Ids of the policies that determined the decision (for audit).
    pub determining_policies: Vec<String>,
}

impl AuthzDecision {
    /// Whether the decision denied the action.
    #[must_use]
    pub fn is_denied(&self) -> bool {
        !self.allowed
    }
}

/// A loaded, evaluatable Cedar policy set. (`Authorizer` is stateless and not
/// `Clone`, so it is constructed per evaluation rather than stored.)
#[derive(Clone)]
pub struct PolicyEngine {
    policies: PolicySet,
}

impl PolicyEngine {
    /// Build an engine from a policy-set source string.
    ///
    /// # Errors
    /// [`AuthzError::Policy`] if the source does not parse.
    pub fn from_policies_str(src: &str) -> Result<Self, AuthzError> {
        let policies = PolicySet::from_str(src).map_err(|e| AuthzError::Policy(e.to_string()))?;
        Ok(Self { policies })
    }

    /// Load and concatenate every `*.cedar` file in `dir` (sorted for stable
    /// policy ordering) into one policy set.
    ///
    /// # Errors
    /// [`AuthzError::Io`] if the directory cannot be read, [`AuthzError::Policy`]
    /// if any file fails to parse.
    pub fn load_from_dir(dir: &Path) -> Result<Self, AuthzError> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| AuthzError::Io(dir.display().to_string(), e.to_string()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "cedar"))
            .collect();
        entries.sort();

        let mut src = String::new();
        for path in entries {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| AuthzError::Io(path.display().to_string(), e.to_string()))?;
            src.push_str(&text);
            src.push('\n');
        }
        Self::from_policies_str(&src)
    }

    /// Evaluate an authorization request. Any malformed UID/context yields a
    /// safe `Deny` decision (fail closed) rather than an error, except that a
    /// parse failure is reported so callers can distinguish a bug from a policy
    /// denial when they care.
    ///
    /// # Errors
    /// [`AuthzError::BadRequest`] when a UID or request is malformed.
    pub fn is_authorized(
        &self,
        principal: &str,
        action: &str,
        resource: &str,
        context: Context,
        entities: &Entities,
    ) -> Result<AuthzDecision, AuthzError> {
        let principal = EntityUid::from_str(principal)
            .map_err(|e| AuthzError::BadRequest(format!("principal: {e}")))?;
        let action = EntityUid::from_str(action)
            .map_err(|e| AuthzError::BadRequest(format!("action: {e}")))?;
        let resource = EntityUid::from_str(resource)
            .map_err(|e| AuthzError::BadRequest(format!("resource: {e}")))?;

        let request = Request::new(principal, action, resource, context, None)
            .map_err(|e| AuthzError::BadRequest(e.to_string()))?;

        let response = Authorizer::new().is_authorized(&request, &self.policies, entities);

        Ok(AuthzDecision {
            allowed: response.decision() == Decision::Allow,
            // Prefer the human-meaningful `@id("…")` annotation for the audit
            // trail (TR-04-008); fall back to Cedar's auto-assigned id.
            determining_policies: response
                .diagnostics()
                .reason()
                .map(|pid| {
                    self.policies
                        .policy(pid)
                        .and_then(|p| p.annotation("id"))
                        .map_or_else(|| pid.to_string(), ToString::to_string)
                })
                .collect(),
        })
    }
}

/// Validate a policy set against a Cedar schema (used by the CI/validation
/// test for TR-04-006). Returns the human-readable validation errors, if any.
///
/// # Errors
/// [`AuthzError::Policy`] if the schema or policies fail to parse.
pub fn validate_policies(schema_src: &str, policies_src: &str) -> Result<Vec<String>, AuthzError> {
    use cedar_policy::{ValidationMode, Validator};

    let (schema, _warnings) = Schema::from_cedarschema_str(schema_src)
        .map_err(|e| AuthzError::Policy(format!("schema: {e}")))?;
    let policies =
        PolicySet::from_str(policies_src).map_err(|e| AuthzError::Policy(e.to_string()))?;
    let validator = Validator::new(schema);
    let result = validator.validate(&policies, ValidationMode::default());
    if result.validation_passed() {
        Ok(Vec::new())
    } else {
        Ok(result
            .validation_errors()
            .map(ToString::to_string)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cedar_policy::Entities;
    use serde_json::json;

    fn manifest_path(rel: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
    }

    fn engine() -> PolicyEngine {
        PolicyEngine::load_from_dir(&manifest_path("authz/policies"))
            .expect("policies load from disk")
    }

    /// Entities: an admin user, a regular user, and the admin panel resource.
    fn entities() -> Entities {
        let v = json!([
            {"uid":{"type":"Role","id":"admin"},"attrs":{},"parents":[]},
            {"uid":{"type":"Role","id":"user"},"attrs":{},"parents":[]},
            {"uid":{"type":"AdminPanel","id":"main"},"attrs":{},"parents":[]},
            {"uid":{"type":"User","id":"boss@corp.example"},
             "attrs":{"email":"boss@corp.example","role":"admin"},
             "parents":[{"type":"Role","id":"admin"}]},
            {"uid":{"type":"User","id":"bob@corp.example"},
             "attrs":{"email":"bob@corp.example","role":"user"},
             "parents":[{"type":"Role","id":"user"}]}
        ]);
        Entities::from_json_value(v, None).unwrap()
    }

    #[test]
    fn admin_is_allowed_admin_panel_access() {
        let d = engine()
            .is_authorized(
                "User::\"boss@corp.example\"",
                "Action::\"admin.access\"",
                "AdminPanel::\"main\"",
                Context::empty(),
                &entities(),
            )
            .unwrap();
        assert!(d.allowed, "admin should be allowed");
        assert!(
            d.determining_policies
                .iter()
                .any(|p| p == "admin-full-access"),
            "expected admin policy to determine; got {:?}",
            d.determining_policies
        );
    }

    #[test]
    fn regular_user_is_denied_admin_panel_access() {
        let d = engine()
            .is_authorized(
                "User::\"bob@corp.example\"",
                "Action::\"admin.access\"",
                "AdminPanel::\"main\"",
                Context::empty(),
                &entities(),
            )
            .unwrap();
        assert!(d.is_denied(), "non-admin must be denied admin access");
    }

    #[test]
    fn user_may_read_own_profile_but_not_another() {
        let e = engine();
        let ents = entities();
        let own = e
            .is_authorized(
                "User::\"bob@corp.example\"",
                "Action::\"profile.read\"",
                "User::\"bob@corp.example\"",
                Context::empty(),
                &ents,
            )
            .unwrap();
        assert!(own.allowed, "reading own profile is allowed");

        let other = e
            .is_authorized(
                "User::\"bob@corp.example\"",
                "Action::\"profile.read\"",
                "User::\"boss@corp.example\"",
                Context::empty(),
                &ents,
            )
            .unwrap();
        assert!(other.is_denied(), "reading another's profile is denied");
    }

    #[test]
    fn policies_validate_against_schema() {
        let schema = std::fs::read_to_string(manifest_path("authz/schema.cedarschema")).unwrap();
        let policies = std::fs::read_to_string(manifest_path("authz/policies/core.cedar")).unwrap();
        let errors = validate_policies(&schema, &policies).expect("validation runs");
        assert!(errors.is_empty(), "policies must validate: {errors:?}");
    }
}
