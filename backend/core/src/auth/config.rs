//! Auth configuration (P4).
//!
//! OIDC/Rauthy relying-party settings and token TTLs come from the loco
//! `settings.auth` block (env-sourced via the P2 config templating). The
//! self-registration toggle is a **startup environment variable**
//! (`SUPERAPP_BACKEND_SELF_REGISTRATION_ENABLED`, default `false`) read once at
//! boot, per TR-04-011.

use serde::Deserialize;

/// Environment variable gating self-onboarding (TR-04-011).
pub const SELF_REGISTRATION_ENV: &str = "SUPERAPP_BACKEND_SELF_REGISTRATION_ENABLED";

const DEFAULT_ACCESS_TTL_SECS: u64 = 900; // 15 minutes
const DEFAULT_REFRESH_TTL_SECS: u64 = 2_592_000; // 30 days
const DEFAULT_REDIS_URL: &str = "redis://localhost:6379";

/// OIDC relying-party settings for authenticating against Rauthy (TR-04-001).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OidcSettings {
    /// Rauthy issuer URL (used for OIDC discovery and `iss` validation).
    pub issuer_url: String,
    /// Confidential client id registered in Rauthy.
    pub client_id: String,
    /// Confidential client secret.
    pub client_secret: String,
    /// Redirect URL registered for the authorization-code callback.
    pub redirect_url: String,
    /// Expected token audience; when absent, `client_id` is used.
    #[serde(default)]
    pub audience: Option<String>,
}

impl OidcSettings {
    /// The audience an access token must carry (falls back to `client_id`).
    #[must_use]
    pub fn expected_audience(&self) -> &str {
        self.audience.as_deref().unwrap_or(&self.client_id)
    }
}

/// The `settings.auth` block. OIDC is optional so the app can boot in test/dev
/// without a live Rauthy; endpoints that need it fail clearly when it's absent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AuthSettings {
    /// OIDC RP settings; `None` when not configured.
    #[serde(default)]
    pub oidc: Option<OidcSettings>,
    /// Access-token lifetime in seconds (short-lived).
    #[serde(default = "default_access_ttl")]
    pub access_token_ttl_secs: u64,
    /// Refresh-token lifetime in seconds (long-lived, stored in Redis).
    #[serde(default = "default_refresh_ttl")]
    pub refresh_token_ttl_secs: u64,
    /// Redis connection URL for the refresh-token store.
    #[serde(default = "default_redis_url")]
    pub redis_url: String,
    /// Optional static token-validation settings. When present, the access
    /// token validator is built directly from this JWKS instead of via OIDC
    /// discovery — used where discovery is unavailable (e.g. tests) or to pin
    /// keys. Requires `issuer` and `audience` alongside.
    #[serde(default)]
    pub jwks_json: Option<String>,
    /// Expected issuer for static validation (see [`Self::jwks_json`]).
    #[serde(default)]
    pub issuer: Option<String>,
    /// Expected audience for static validation (see [`Self::jwks_json`]).
    #[serde(default)]
    pub audience: Option<String>,
}

fn default_access_ttl() -> u64 {
    DEFAULT_ACCESS_TTL_SECS
}
fn default_refresh_ttl() -> u64 {
    DEFAULT_REFRESH_TTL_SECS
}
fn default_redis_url() -> String {
    DEFAULT_REDIS_URL.to_string()
}

impl Default for AuthSettings {
    fn default() -> Self {
        AuthSettings {
            oidc: None,
            access_token_ttl_secs: DEFAULT_ACCESS_TTL_SECS,
            refresh_token_ttl_secs: DEFAULT_REFRESH_TTL_SECS,
            redis_url: DEFAULT_REDIS_URL.to_string(),
            jwks_json: None,
            issuer: None,
            audience: None,
        }
    }
}

impl AuthSettings {
    /// Build an access-token validator from the static JWKS settings, if all
    /// of `jwks_json`, `issuer`, and `audience` are present.
    ///
    /// # Errors
    /// When the JWKS JSON is malformed.
    pub fn static_validator(
        &self,
    ) -> Result<Option<crate::auth::token::TokenValidator>, crate::auth::token::TokenError> {
        match (&self.jwks_json, &self.issuer, &self.audience) {
            (Some(jwks), Some(iss), Some(aud)) => Ok(Some(
                crate::auth::token::TokenValidator::from_jwks_json(jwks, iss, aud)?,
            )),
            _ => Ok(None),
        }
    }
}

impl AuthSettings {
    /// Parse the `auth` block out of loco's free-form `settings` JSON value.
    /// A missing `settings` or `settings.auth` yields defaults (no OIDC).
    ///
    /// # Errors
    /// When `settings.auth` is present but malformed.
    pub fn from_settings(settings: Option<&serde_json::Value>) -> Result<Self, serde_json::Error> {
        match settings.and_then(|s| s.get("auth")) {
            Some(auth) => serde_json::from_value(auth.clone()),
            None => Ok(AuthSettings::default()),
        }
    }
}

/// Interpret a raw env value as the self-registration boolean. Truthy values
/// are `1`, `true`, `yes`, `on` (case-insensitive, trimmed); everything else —
/// including absence — is `false` (TR-04-011 default disabled).
#[must_use]
pub fn parse_self_registration(raw: Option<&str>) -> bool {
    matches!(
        raw.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// Read the self-registration toggle from the process environment (once, at
/// startup).
#[must_use]
pub fn self_registration_enabled() -> bool {
    parse_self_registration(std::env::var(SELF_REGISTRATION_ENV).ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn self_registration_defaults_to_false() {
        assert!(!parse_self_registration(None));
        assert!(!parse_self_registration(Some("")));
        assert!(!parse_self_registration(Some("false")));
        assert!(!parse_self_registration(Some("0")));
        assert!(!parse_self_registration(Some("nonsense")));
    }

    #[test]
    fn self_registration_truthy_values() {
        for v in ["1", "true", "TRUE", " yes ", "On"] {
            assert!(parse_self_registration(Some(v)), "expected {v:?} truthy");
        }
    }

    #[test]
    fn absent_auth_block_yields_defaults() {
        let s = AuthSettings::from_settings(None).unwrap();
        assert!(s.oidc.is_none());
        assert_eq!(s.access_token_ttl_secs, DEFAULT_ACCESS_TTL_SECS);
        assert_eq!(s.refresh_token_ttl_secs, DEFAULT_REFRESH_TTL_SECS);
        assert_eq!(s.redis_url, DEFAULT_REDIS_URL);
    }

    #[test]
    fn parses_full_auth_block() {
        let settings = json!({
            "auth": {
                "oidc": {
                    "issuer_url": "https://rauthy.example/auth/v1",
                    "client_id": "superapp",
                    "client_secret": "shh",
                    "redirect_url": "https://app.example/callback"
                },
                "access_token_ttl_secs": 600,
                "refresh_token_ttl_secs": 100,
                "redis_url": "redis://localhost:6379/1"
            }
        });
        let s = AuthSettings::from_settings(Some(&settings)).unwrap();
        let oidc = s.oidc.expect("oidc present");
        assert_eq!(oidc.client_id, "superapp");
        assert_eq!(oidc.expected_audience(), "superapp"); // falls back to client_id
        assert_eq!(s.access_token_ttl_secs, 600);
        assert_eq!(s.redis_url, "redis://localhost:6379/1");
    }

    #[test]
    fn audience_overrides_client_id_when_present() {
        let oidc = OidcSettings {
            issuer_url: "i".into(),
            client_id: "cid".into(),
            client_secret: "s".into(),
            redirect_url: "r".into(),
            audience: Some("aud".into()),
        };
        assert_eq!(oidc.expected_audience(), "aud");
    }
}
