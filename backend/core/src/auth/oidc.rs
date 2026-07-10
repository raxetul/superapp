//! OIDC relying-party client for Rauthy (TR-04-001): discovery, the
//! authorization-code flow (with PKCE), and refresh.
//!
//! Per the project DI rule, callers depend on the [`OidcProvider`] trait and
//! receive an implementation by injection. Production wires
//! [`RauthyOidcClient`] (real network via `openidconnect`); tests inject
//! [`FakeOidcProvider`]. The network client is exercised end-to-end only
//! against a live Rauthy; the flow *orchestration* in controllers is tested
//! against the fake.

use async_trait::async_trait;
use openidconnect::core::{CoreClient, CoreProviderMetadata, CoreResponseType, CoreTokenResponse};
use openidconnect::reqwest::async_http_client;
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope,
};

use crate::auth::config::OidcSettings;

/// Tokens returned by an OIDC token endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcTokens {
    /// The access token (a JWT validated against the JWKS elsewhere).
    pub access_token: String,
    /// The refresh token, when the server issued one.
    pub refresh_token: Option<String>,
    /// Access-token lifetime in seconds, when advertised.
    pub expires_in_secs: Option<u64>,
}

/// A prepared authorization redirect plus the transient values the caller must
/// stash (in a cookie/session) to complete the callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthRedirect {
    /// The Rauthy authorization URL to redirect the browser to.
    pub url: String,
    /// CSRF state to compare on callback.
    pub csrf_state: String,
    /// PKCE verifier to send on code exchange.
    pub pkce_verifier: String,
    /// Nonce to compare against the ID token.
    pub nonce: String,
}

/// OIDC failures. All map to `401`/`502` at the HTTP boundary depending on
/// cause; the controller decides.
#[derive(Debug, thiserror::Error)]
pub enum OidcError {
    /// OIDC is not configured (no `settings.auth.oidc`).
    #[error("OIDC is not configured")]
    NotConfigured,
    /// Discovery or client construction failed.
    #[error("OIDC configuration/discovery error: {0}")]
    Config(String),
    /// The authorization-code or refresh exchange failed.
    #[error("OIDC token exchange failed: {0}")]
    Exchange(String),
}

/// The OIDC RP seam. Injected into the auth controllers.
#[async_trait]
pub trait OidcProvider: Send + Sync {
    /// Build an authorization-code redirect (with fresh PKCE + CSRF + nonce).
    fn authorize_url(&self) -> Result<AuthRedirect, OidcError>;
    /// Exchange an authorization `code` (+ its PKCE verifier) for tokens.
    async fn exchange_code(&self, code: &str, pkce_verifier: &str)
        -> Result<OidcTokens, OidcError>;
    /// Exchange a refresh token for a fresh set of tokens.
    async fn refresh(&self, refresh_token: &str) -> Result<OidcTokens, OidcError>;
}

/// Fetch the issuer's JWKS JSON via OIDC discovery (`/.well-known/
/// openid-configuration` → `jwks_uri`). Used at startup to build the access-
/// token validator when a static JWKS is not configured.
///
/// # Errors
/// [`OidcError::Config`] if discovery or the JWKS fetch fails.
pub async fn discover_jwks(issuer_url: &str) -> Result<String, OidcError> {
    let disc_url = format!(
        "{}/.well-known/openid-configuration",
        issuer_url.trim_end_matches('/')
    );
    let client = reqwest::Client::new();
    let meta: serde_json::Value = client
        .get(&disc_url)
        .send()
        .await
        .map_err(|e| OidcError::Config(e.to_string()))?
        .json()
        .await
        .map_err(|e| OidcError::Config(e.to_string()))?;
    let jwks_uri = meta
        .get("jwks_uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| OidcError::Config("discovery document has no jwks_uri".into()))?;
    client
        .get(jwks_uri)
        .send()
        .await
        .map_err(|e| OidcError::Config(e.to_string()))?
        .text()
        .await
        .map_err(|e| OidcError::Config(e.to_string()))
}

/// Real Rauthy RP built from discovered provider metadata.
pub struct RauthyOidcClient {
    client: CoreClient,
    scopes: Vec<String>,
}

impl RauthyOidcClient {
    /// Discover Rauthy's OIDC metadata and construct the RP client.
    ///
    /// # Errors
    /// [`OidcError::Config`] if the issuer URL is invalid or discovery fails.
    pub async fn discover(settings: &OidcSettings) -> Result<Self, OidcError> {
        let issuer = IssuerUrl::new(settings.issuer_url.clone())
            .map_err(|e| OidcError::Config(e.to_string()))?;
        let metadata = CoreProviderMetadata::discover_async(issuer, async_http_client)
            .await
            .map_err(|e| OidcError::Config(e.to_string()))?;
        let redirect = RedirectUrl::new(settings.redirect_url.clone())
            .map_err(|e| OidcError::Config(e.to_string()))?;
        let client = CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(settings.client_id.clone()),
            Some(ClientSecret::new(settings.client_secret.clone())),
        )
        .set_redirect_uri(redirect);
        Ok(Self {
            client,
            scopes: vec!["openid".into(), "email".into(), "profile".into()],
        })
    }
}

#[async_trait]
impl OidcProvider for RauthyOidcClient {
    fn authorize_url(&self) -> Result<AuthRedirect, OidcError> {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let mut req = self.client.authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );
        for s in &self.scopes {
            req = req.add_scope(Scope::new(s.clone()));
        }
        let (url, csrf, nonce) = req.set_pkce_challenge(challenge).url();
        Ok(AuthRedirect {
            url: url.to_string(),
            csrf_state: csrf.secret().clone(),
            pkce_verifier: verifier.secret().clone(),
            nonce: nonce.secret().clone(),
        })
    }

    async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<OidcTokens, OidcError> {
        let resp = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| OidcError::Exchange(e.to_string()))?;
        Ok(to_tokens(&resp))
    }

    async fn refresh(&self, refresh_token: &str) -> Result<OidcTokens, OidcError> {
        let resp = self
            .client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| OidcError::Exchange(e.to_string()))?;
        Ok(to_tokens(&resp))
    }
}

/// Map an `openidconnect` token response into our transport-agnostic
/// [`OidcTokens`]. (`OAuth2TokenResponse` brings the accessor methods into
/// scope for the concrete `CoreTokenResponse`.)
fn to_tokens(resp: &CoreTokenResponse) -> OidcTokens {
    OidcTokens {
        access_token: resp.access_token().secret().clone(),
        refresh_token: resp.refresh_token().map(|t| t.secret().clone()),
        expires_in_secs: resp.expires_in().map(|d| d.as_secs()),
    }
}

/// In-memory OIDC provider for controller tests. Deterministic; no network.
pub struct FakeOidcProvider {
    /// Tokens returned by `exchange_code`, keyed by authorization code.
    pub codes: std::collections::HashMap<String, OidcTokens>,
    /// Tokens returned by `refresh`, keyed by presented refresh token.
    pub refreshes: std::collections::HashMap<String, OidcTokens>,
}

impl FakeOidcProvider {
    /// A fake that exchanges `code` → the given tokens.
    #[must_use]
    pub fn with_code(code: &str, tokens: OidcTokens) -> Self {
        let mut codes = std::collections::HashMap::new();
        codes.insert(code.to_string(), tokens);
        Self {
            codes,
            refreshes: std::collections::HashMap::new(),
        }
    }

    /// Register a refresh mapping.
    #[must_use]
    pub fn and_refresh(mut self, refresh_token: &str, tokens: OidcTokens) -> Self {
        self.refreshes.insert(refresh_token.to_string(), tokens);
        self
    }
}

#[async_trait]
impl OidcProvider for FakeOidcProvider {
    fn authorize_url(&self) -> Result<AuthRedirect, OidcError> {
        Ok(AuthRedirect {
            url: "https://rauthy.test/authorize?fake=1".into(),
            csrf_state: "csrf-fake".into(),
            pkce_verifier: "verifier-fake".into(),
            nonce: "nonce-fake".into(),
        })
    }

    async fn exchange_code(
        &self,
        code: &str,
        _pkce_verifier: &str,
    ) -> Result<OidcTokens, OidcError> {
        self.codes
            .get(code)
            .cloned()
            .ok_or_else(|| OidcError::Exchange(format!("unknown code {code}")))
    }

    async fn refresh(&self, refresh_token: &str) -> Result<OidcTokens, OidcError> {
        self.refreshes
            .get(refresh_token)
            .cloned()
            .ok_or_else(|| OidcError::Exchange("unknown refresh token".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_provider_exchanges_code_and_refresh() {
        let provider = FakeOidcProvider::with_code(
            "auth-code-1",
            OidcTokens {
                access_token: "at-1".into(),
                refresh_token: Some("rt-1".into()),
                expires_in_secs: Some(900),
            },
        )
        .and_refresh(
            "rt-1",
            OidcTokens {
                access_token: "at-2".into(),
                refresh_token: Some("rt-2".into()),
                expires_in_secs: Some(900),
            },
        );

        let redirect = provider.authorize_url().unwrap();
        assert!(redirect.url.starts_with("https://rauthy.test/authorize"));

        let t = provider.exchange_code("auth-code-1", "v").await.unwrap();
        assert_eq!(t.access_token, "at-1");
        assert_eq!(t.refresh_token.as_deref(), Some("rt-1"));

        let t2 = provider.refresh("rt-1").await.unwrap();
        assert_eq!(t2.access_token, "at-2");

        assert!(provider.exchange_code("nope", "v").await.is_err());
    }
}
