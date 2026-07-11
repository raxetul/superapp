//! The auth composition root (P4).
//!
//! [`AuthState`] bundles the wired-up collaborators — token validator, OIDC
//! provider, refresh store, and the Cedar [`Enforcer`] — that the controllers
//! depend on. It is constructed once at startup ([`AuthState::build`]) and
//! layered into the router as an Axum `Extension`. Concrete adapters are wired
//! here; each side-effecting dependency degrades gracefully when its backing
//! service is unavailable, so the app still boots (e.g. in tests without a live
//! Rauthy) with those capabilities disabled.

use std::path::Path;
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::auth::config::{AuthSettings, ModulesSettings};
use crate::auth::oidc::{self, OidcProvider, RauthyOidcClient};
use crate::auth::refresh::{InMemoryRefreshStore, RefreshStore, RefreshTokens};
use crate::auth::token::TokenValidator;
use crate::authz::engine::PolicyEngine;
use crate::authz::entities::{CachedEntityProvider, DbEntityProvider, SystemClock};
use crate::authz::Enforcer;
use crate::events::EventHub;
use crate::modules::registry::{Gateway, ModuleRegistry};
use crate::modules::runtime::DockerRuntime;
use crate::modules::signing::{SelfSigner, TrustStore};

const ENTITY_CACHE_TTL_MILLIS: u64 = 5_000;

/// Fatal auth-wiring errors (only the policy set is mandatory — without it we
/// cannot make authorization decisions and must refuse to boot).
#[derive(Debug, thiserror::Error)]
pub enum AuthStateError {
    /// The Cedar policy set failed to load.
    #[error("failed to load authorization policies: {0}")]
    Policies(String),
}

/// The wired auth dependencies shared across requests.
#[derive(Clone)]
pub struct AuthState {
    /// Access-token validator (`None` when neither a static JWKS nor OIDC
    /// discovery is available).
    pub validator: Option<Arc<TokenValidator>>,
    /// OIDC RP for the authorization-code flow (`None` when not configured).
    pub oidc: Option<Arc<dyn OidcProvider>>,
    /// Refresh-token store (Redis in production, in-memory fallback).
    pub refresh: Arc<dyn RefreshTokens>,
    /// Cedar enforcement point.
    pub enforcer: Arc<Enforcer>,
    /// Short access-token lifetime (seconds).
    pub access_ttl_secs: u64,
    /// Whether self-registration is enabled (TR-04-011).
    pub self_registration_enabled: bool,
    /// Whether an OIDC provider is configured (surfaced by `/auth/capabilities`).
    pub oidc_configured: bool,
    /// Trusted module-signer public keys (self + external) (TR-05-002/009).
    pub trust: Arc<TrustStore>,
    /// Module lifecycle registry (P5).
    pub registry: Arc<ModuleRegistry>,
    /// Cedar-enforcing module gateway (TR-05-007).
    pub gateway: Arc<Gateway>,
    /// Real-time SSE event hub (P6).
    pub events: Arc<EventHub>,
}

impl AuthState {
    /// Build the auth state from settings, a DB handle, the Cedar policies
    /// directory, and the self-registration toggle.
    ///
    /// # Errors
    /// [`AuthStateError::Policies`] if the policy set cannot be loaded.
    pub async fn build(
        settings: &AuthSettings,
        modules_settings: &ModulesSettings,
        db: DatabaseConnection,
        policies_dir: &Path,
        self_registration_enabled: bool,
    ) -> Result<Self, AuthStateError> {
        // Cedar enforcement point (mandatory).
        let engine = PolicyEngine::load_from_dir(policies_dir)
            .map_err(|e| AuthStateError::Policies(e.to_string()))?;
        let provider = CachedEntityProvider::new(
            DbEntityProvider::new(db),
            Arc::new(SystemClock::default()),
            ENTITY_CACHE_TTL_MILLIS,
        );
        let enforcer = Arc::new(Enforcer::new(engine, Arc::new(provider)));

        // Module signing trust store: self key (bootstrapped on first startup)
        // plus any configured external signers (TR-05-002 / TR-05-009).
        let key_path = modules_settings.signing_key_path.clone().map_or_else(
            || std::env::temp_dir().join("superapp/self_signing.key"),
            std::path::PathBuf::from,
        );
        let self_signer = match SelfSigner::load_or_generate(&key_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "self-signing key persistence failed; using ephemeral key");
                SelfSigner::generate()
            }
        };
        let mut trust = self_signer.trust_store();
        for ext in &modules_settings.trusted_signers {
            if let Err(e) = trust.add_base64(&ext.signer, &ext.public_key) {
                tracing::warn!(signer = %ext.signer, error = %e, "ignoring malformed trusted signer key");
            }
        }
        let trust = Arc::new(trust);

        // Module runtime + gateway. Docker in production; the gateway enforces
        // Cedar before proxying (TR-05-007).
        let registry = Arc::new(ModuleRegistry::new(Arc::new(DockerRuntime::default())));
        let gateway = Arc::new(Gateway::new(registry.clone(), enforcer.clone()));

        // Access-token validator: static JWKS first, else OIDC discovery.
        let validator = build_validator(settings).await.map(Arc::new);

        // OIDC RP (network); optional.
        let oidc: Option<Arc<dyn OidcProvider>> = match &settings.oidc {
            Some(oidc_settings) => match RauthyOidcClient::discover(oidc_settings).await {
                Ok(client) => Some(Arc::new(client)),
                Err(e) => {
                    tracing::warn!(error = %e, "OIDC discovery failed; login endpoints disabled");
                    None
                }
            },
            None => None,
        };
        let oidc_configured = settings.oidc.is_some();

        // Refresh store: Redis, falling back to in-memory if unreachable.
        let refresh: Arc<dyn RefreshTokens> =
            match RefreshStore::connect(&settings.redis_url, settings.refresh_token_ttl_secs).await
            {
                Ok(store) => Arc::new(store),
                Err(e) => {
                    tracing::warn!(error = %e, "Redis unavailable; using in-memory refresh store");
                    Arc::new(InMemoryRefreshStore::default())
                }
            };

        Ok(Self {
            validator,
            oidc,
            refresh,
            enforcer,
            access_ttl_secs: settings.access_token_ttl_secs,
            self_registration_enabled,
            oidc_configured,
            trust,
            registry,
            gateway,
            events: Arc::new(EventHub::default()),
        })
    }
}

/// Build a token validator from the static JWKS settings, or via OIDC
/// discovery of the issuer's JWKS. Returns `None` if neither is available.
async fn build_validator(settings: &AuthSettings) -> Option<TokenValidator> {
    match settings.static_validator() {
        Ok(Some(v)) => return Some(v),
        Ok(None) => {}
        Err(e) => tracing::warn!(error = %e, "static JWKS invalid; ignoring"),
    }
    let oidc = settings.oidc.as_ref()?;
    match oidc::discover_jwks(&oidc.issuer_url).await {
        Ok(jwks) => {
            match TokenValidator::from_jwks_json(
                &jwks,
                oidc.issuer_url.clone(),
                oidc.expected_audience().to_string(),
            ) {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::warn!(error = %e, "discovered JWKS invalid");
                    None
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "JWKS discovery failed; token validation disabled");
            None
        }
    }
}
