//! Module lifecycle registry + gateway (TR-05-001/004/005/007/008).
//!
//! [`ModuleRegistry`] starts a module via the injected [`ContainerRuntime`],
//! waits for readiness (polling `/health`) before serving its routes, proxies
//! manifest-declared routes to it, reports per-module health, and stops it
//! cleanly. A crashed/unreachable module yields `502/503` for **its** routes
//! only — the core and other modules keep serving (fault isolation).
//!
//! [`Gateway`] layers Cedar authorization on top: it resolves the manifest
//! endpoint, enforces the endpoint's required permission, and only then
//! proxies — a denied request is stopped at the gateway and never reaches the
//! module container.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use cedar_policy::Context;

use crate::authz::Enforcer;
use crate::modules::manifest::Manifest;
use crate::modules::runtime::{ContainerRuntime, ModuleSpec, RunningHandle, RuntimeError};

/// A currently-loaded module.
#[derive(Clone)]
pub struct LoadedModule {
    pub manifest: Manifest,
    pub handle: RunningHandle,
}

/// Per-module health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleHealth {
    Healthy,
    Unhealthy,
    /// Loaded but unreachable (crashed/hung).
    Unreachable,
    /// Not loaded.
    Unknown,
}

/// Result of proxying to a module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyOutcome {
    /// The module answered.
    Response { status: u16, body: String },
    /// The module is not loaded.
    NotFound,
    /// The module is loaded but unreachable (fault isolated → 502/503).
    Unavailable(String),
}

/// Failure to load a module.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("module `{0}` did not become ready in time")]
    NotReady(String),
}

const READINESS_TRIES: u32 = 30;
const READINESS_INTERVAL: Duration = Duration::from_millis(100);

/// Manages loaded modules and proxying.
pub struct ModuleRegistry {
    runtime: Arc<dyn ContainerRuntime>,
    http: reqwest::Client,
    loaded: Mutex<HashMap<String, LoadedModule>>,
}

impl ModuleRegistry {
    #[must_use]
    pub fn new(runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self {
            runtime,
            http: reqwest::Client::new(),
            loaded: Mutex::new(HashMap::new()),
        }
    }

    /// Start `spec`'s module, wait for readiness, then register it as ready.
    /// If it fails to start or never becomes ready it is stopped and the error
    /// returned — other modules are untouched.
    ///
    /// # Errors
    /// [`LoadError`] on start failure or readiness timeout.
    pub async fn load(&self, manifest: Manifest, spec: &ModuleSpec) -> Result<(), LoadError> {
        let handle = self.runtime.start(spec).await?;
        if !self.await_ready(&handle.address).await {
            let _ = self.runtime.stop(&handle).await;
            return Err(LoadError::NotReady(spec.name.clone()));
        }
        self.loaded
            .lock()
            .unwrap()
            .insert(manifest.name.clone(), LoadedModule { manifest, handle });
        Ok(())
    }

    /// Stop and forget a module (idempotent).
    ///
    /// # Errors
    /// [`RuntimeError`] if the runtime fails to stop it.
    pub async fn unload(&self, name: &str) -> Result<(), RuntimeError> {
        let module = self.loaded.lock().unwrap().remove(name);
        if let Some(module) = module {
            self.runtime.stop(&module.handle).await?;
        }
        Ok(())
    }

    #[must_use]
    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded.lock().unwrap().contains_key(name)
    }

    #[must_use]
    pub fn manifest(&self, name: &str) -> Option<Manifest> {
        self.loaded
            .lock()
            .unwrap()
            .get(name)
            .map(|m| m.manifest.clone())
    }

    fn address(&self, name: &str) -> Option<String> {
        self.loaded
            .lock()
            .unwrap()
            .get(name)
            .map(|m| m.handle.address.clone())
    }

    async fn await_ready(&self, address: &str) -> bool {
        for _ in 0..READINESS_TRIES {
            if self.probe_health(address).await == ModuleHealth::Healthy {
                return true;
            }
            tokio::time::sleep(READINESS_INTERVAL).await;
        }
        false
    }

    async fn probe_health(&self, address: &str) -> ModuleHealth {
        match self
            .http
            .get(format!("{address}/health"))
            .timeout(Duration::from_millis(500))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => ModuleHealth::Healthy,
            Ok(_) => ModuleHealth::Unhealthy,
            Err(_) => ModuleHealth::Unreachable,
        }
    }

    /// Report a module's current health (TR-05-005).
    pub async fn health(&self, name: &str) -> ModuleHealth {
        match self.address(name) {
            Some(addr) => self.probe_health(&addr).await,
            None => ModuleHealth::Unknown,
        }
    }

    /// Proxy a `GET` to a module route. A connection error is contained as
    /// [`ProxyOutcome::Unavailable`] (fault isolation, TR-05-008).
    pub async fn proxy_get(&self, name: &str, path: &str) -> ProxyOutcome {
        let Some(addr) = self.address(name) else {
            return ProxyOutcome::NotFound;
        };
        match self
            .http
            .get(format!("{addr}{path}"))
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                ProxyOutcome::Response { status, body }
            }
            Err(e) => ProxyOutcome::Unavailable(e.to_string()),
        }
    }
}

/// The gateway: Cedar-authorize, then proxy (TR-05-007).
pub struct Gateway {
    registry: Arc<ModuleRegistry>,
    enforcer: Arc<Enforcer>,
}

/// Gateway decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayOutcome {
    Proxied {
        status: u16,
        body: String,
    },
    /// Denied by Cedar before proxying — the module never saw the request.
    Forbidden,
    /// No such module/route.
    NotFound,
    /// Module unreachable.
    Unavailable(String),
}

impl Gateway {
    #[must_use]
    pub fn new(registry: Arc<ModuleRegistry>, enforcer: Arc<Enforcer>) -> Self {
        Self { registry, enforcer }
    }

    /// Handle a gateway request for `module`'s `method path` on behalf of
    /// `principal` (a Cedar `User::"…"` UID). Enforces the endpoint's declared
    /// permission before proxying.
    pub async fn handle(
        &self,
        principal: &str,
        module: &str,
        method: &str,
        path: &str,
    ) -> GatewayOutcome {
        let Some(manifest) = self.registry.manifest(module) else {
            return GatewayOutcome::NotFound;
        };
        let Some(endpoint) = manifest
            .endpoints
            .iter()
            .find(|e| e.method.eq_ignore_ascii_case(method) && e.path == path)
        else {
            return GatewayOutcome::NotFound;
        };

        // Enforce the endpoint's required permission BEFORE proxying.
        if let Some(permission) = &endpoint.permission {
            let action = format!("Action::\"{permission}\"");
            let resource = format!("Module::\"{module}\"");
            let allowed = self
                .enforcer
                .enforce(principal, &action, &resource, Context::empty())
                .await
                .map(|d| d.allowed)
                .unwrap_or(false); // fail closed
            if !allowed {
                return GatewayOutcome::Forbidden; // never reaches the module
            }
        }

        match self.registry.proxy_get(module, path).await {
            ProxyOutcome::Response { status, body } => GatewayOutcome::Proxied { status, body },
            ProxyOutcome::NotFound => GatewayOutcome::NotFound,
            ProxyOutcome::Unavailable(e) => GatewayOutcome::Unavailable(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authz::engine::PolicyEngine;
    use crate::authz::entities::{parse_entities, EntityProvider};
    use crate::modules::manifest::{Endpoint, Manifest};
    use crate::modules::runtime::InProcessRuntime;
    use async_trait::async_trait;
    use serde_json::json;

    fn manifest(name: &str, permission: Option<&str>) -> Manifest {
        Manifest {
            name: name.into(),
            version: "1.0.0".into(),
            endpoints: vec![Endpoint {
                method: "GET".into(),
                path: "/items".into(),
                permission: permission.map(Into::into),
            }],
            permissions: permission.map(|p| vec![p.to_string()]).unwrap_or_default(),
            config_schema: json!({"type":"object"}),
            signatures: vec![],
        }
    }

    fn spec(name: &str, behavior: Option<&str>) -> ModuleSpec {
        let mut s = ModuleSpec::new(name, "1.0.0", "example.local/mod:1.0.0");
        if let Some(b) = behavior {
            s.env.insert("SUPERAPP_MODULE_BEHAVIOR".into(), b.into());
        }
        s
    }

    #[tokio::test]
    async fn load_reaches_ready_and_proxies_then_unloads() {
        let rt = Arc::new(InProcessRuntime::new());
        let reg = ModuleRegistry::new(rt.clone());
        reg.load(manifest("billing", None), &spec("billing", None))
            .await
            .expect("module loads and becomes ready");
        assert!(reg.is_loaded("billing"));
        assert_eq!(reg.health("billing").await, ModuleHealth::Healthy);

        // Route is reachable through the gateway proxy.
        match reg.proxy_get("billing", "/items").await {
            ProxyOutcome::Response { status, body } => {
                assert_eq!(status, 200);
                assert!(body.contains("/items"));
            }
            other => panic!("expected proxied response, got {other:?}"),
        }
        assert_eq!(rt.hits("billing"), 1);

        reg.unload("billing").await.unwrap();
        assert!(!reg.is_loaded("billing"));
    }

    #[tokio::test]
    async fn crashed_module_is_fault_isolated_from_others() {
        let rt = Arc::new(InProcessRuntime::new());
        let reg = ModuleRegistry::new(rt.clone());
        reg.load(manifest("a", None), &spec("a", None))
            .await
            .unwrap();
        reg.load(manifest("b", None), &spec("b", None))
            .await
            .unwrap();

        // Crash module "a" by stopping its server out from under the registry.
        let addr_a = reg.address("a").unwrap();
        rt.stop(&RunningHandle {
            id: "a".into(),
            address: addr_a,
        })
        .await
        .unwrap();

        // "a"'s routes now yield Unavailable; "b" keeps serving.
        assert!(matches!(
            reg.proxy_get("a", "/items").await,
            ProxyOutcome::Unavailable(_)
        ));
        assert!(matches!(
            reg.proxy_get("b", "/items").await,
            ProxyOutcome::Response { status: 200, .. }
        ));
    }

    #[tokio::test]
    async fn unhealthy_module_fails_readiness() {
        let rt = Arc::new(InProcessRuntime::new());
        let reg = ModuleRegistry::new(rt);
        let err = reg
            .load(manifest("sick", None), &spec("sick", Some("unhealthy")))
            .await;
        assert!(matches!(err, Err(LoadError::NotReady(_))));
        assert!(!reg.is_loaded("sick"));
    }

    // --- Gateway Cedar enforcement (TR-05-007) ---

    struct StaticEntities(serde_json::Value);
    #[async_trait]
    impl EntityProvider for StaticEntities {
        async fn entities_json(
            &self,
        ) -> Result<serde_json::Value, crate::authz::engine::AuthzError> {
            Ok(self.0.clone())
        }
        async fn entities(
            &self,
        ) -> Result<cedar_policy::Entities, crate::authz::engine::AuthzError> {
            parse_entities(self.0.clone())
        }
    }

    fn enforcer() -> Arc<Enforcer> {
        let engine = PolicyEngine::load_from_dir(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/authz/policies"
        )))
        .unwrap();
        let entities = StaticEntities(json!([
            {"uid":{"type":"Role","id":"admin"},"attrs":{},"parents":[]},
            {"uid":{"type":"Role","id":"user"},"attrs":{},"parents":[]},
            {"uid":{"type":"User","id":"boss@x"},"attrs":{"email":"boss@x","role":"admin"},"parents":[{"type":"Role","id":"admin"}]},
            {"uid":{"type":"User","id":"bob@x"},"attrs":{"email":"bob@x","role":"user"},"parents":[{"type":"Role","id":"user"}]}
        ]));
        Arc::new(Enforcer::new(engine, Arc::new(entities)))
    }

    #[tokio::test]
    async fn gateway_denies_unpermitted_request_before_proxying() {
        let rt = Arc::new(InProcessRuntime::new());
        let reg = Arc::new(ModuleRegistry::new(rt.clone()));
        reg.load(
            manifest("billing", Some("billing:read")),
            &spec("billing", None),
        )
        .await
        .unwrap();
        let gw = Gateway::new(reg.clone(), enforcer());

        // Regular user lacks the permission → Forbidden, and the module never
        // received the request.
        let denied = gw
            .handle("User::\"bob@x\"", "billing", "GET", "/items")
            .await;
        assert_eq!(denied, GatewayOutcome::Forbidden);
        assert_eq!(rt.hits("billing"), 0, "module must not be reached on deny");

        // Admin is permitted (admin-full-access) → proxied through.
        let allowed = gw
            .handle("User::\"boss@x\"", "billing", "GET", "/items")
            .await;
        assert!(matches!(
            allowed,
            GatewayOutcome::Proxied { status: 200, .. }
        ));
        assert_eq!(rt.hits("billing"), 1);
    }
}
