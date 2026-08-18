//! The container-runtime seam (TR-05-001 / TR-05-004).
//!
//! Modules run out-of-process. The core depends on the [`ContainerRuntime`]
//! abstraction and receives an implementation by injection:
//! - production wires [`DockerRuntime`] (shells out to the `docker` CLI to run
//!   the module's OCI image and map a port);
//! - tests wire [`InProcessRuntime`], which starts a **real** local HTTP server
//!   per module so the gateway, lifecycle, health, and fault-isolation logic
//!   are exercised end-to-end without Docker.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

/// What to run for a module.
#[derive(Debug, Clone)]
pub struct ModuleSpec {
    pub name: String,
    pub version: String,
    /// OCI image reference (used by the Docker runtime).
    pub image: String,
    /// Environment for the container (module config uses the
    /// `SUPERAPP_MODULE_{NAME}_` prefix).
    pub env: HashMap<String, String>,
}

impl ModuleSpec {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        image: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            image: image.into(),
            env: HashMap::new(),
        }
    }
}

/// A handle to a started module.
#[derive(Debug, Clone)]
pub struct RunningHandle {
    /// Runtime id (container id, or the module name for the fake).
    pub id: String,
    /// Base URL the gateway proxies to, e.g. `http://127.0.0.1:47821`.
    pub address: String,
}

/// Runtime failures.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("failed to start module `{0}`: {1}")]
    Start(String, String),
    #[error("failed to stop module `{0}`: {1}")]
    Stop(String, String),
}

/// Abstraction over a container runtime.
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Start a module and return a handle exposing its address.
    async fn start(&self, spec: &ModuleSpec) -> Result<RunningHandle, RuntimeError>;
    /// Stop a running module.
    async fn stop(&self, handle: &RunningHandle) -> Result<(), RuntimeError>;
}

// ---------------------------------------------------------------------------
// Production adapter: Docker CLI. Compile-verified here; exercised where a
// Docker daemon is available (deployment / P10).
// ---------------------------------------------------------------------------

/// Env var overriding the host a module's published port is reachable on
/// (TR-10-005). Needed when the core itself runs inside a container on a
/// bridge network: there, `127.0.0.1` (the default below) is the core's own
/// network namespace, not the Docker host's, so a sibling module container's
/// `docker run -P` published port is unreachable at that address. Deployment
/// compose sets this to the Docker host's address as seen from the backend
/// container; bare-metal/dev (core running directly on the Docker host)
/// leaves it unset and keeps the loopback default.
pub const MODULE_HOST_ENV: &str = "SUPERAPP_BACKEND_MODULE_HOST";

/// Env var naming the Docker network started module containers should join
/// (TR-10-005 — "module containers attached to the helvetia-compose
/// network"). Unset (the default) leaves `docker run` on Docker's default
/// bridge, matching prior behavior.
pub const MODULE_NETWORK_ENV: &str = "SUPERAPP_BACKEND_MODULE_NETWORK";

/// Runs modules as containers via the `docker` CLI.
pub struct DockerRuntime {
    /// Host the mapped port is reachable on.
    host: String,
    /// Docker network started module containers join, if any.
    network: Option<String>,
}

impl DockerRuntime {
    /// Build from [`MODULE_HOST_ENV`]/[`MODULE_NETWORK_ENV`], falling back to
    /// the loopback host and Docker's default bridge network.
    #[must_use]
    pub fn from_env() -> Self {
        Self::with_host_and_network(
            std::env::var(MODULE_HOST_ENV).ok(),
            std::env::var(MODULE_NETWORK_ENV).ok(),
        )
    }

    fn with_host_and_network(host: Option<String>, network: Option<String>) -> Self {
        Self {
            host: host
                .filter(|h| !h.trim().is_empty())
                .unwrap_or_else(|| "127.0.0.1".to_string()),
            network: network.filter(|n| !n.trim().is_empty()),
        }
    }

    #[cfg(test)]
    fn with_host(host: Option<String>) -> Self {
        Self::with_host_and_network(host, None)
    }
}

impl Default for DockerRuntime {
    fn default() -> Self {
        Self::with_host_and_network(None, None)
    }
}

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    async fn start(&self, spec: &ModuleSpec) -> Result<RunningHandle, RuntimeError> {
        use std::process::Command;
        let mut cmd = Command::new("docker");
        cmd.args(["run", "-d", "-P"]);
        if let Some(network) = &self.network {
            cmd.args(["--network", network]);
        }
        for (k, v) in &spec.env {
            cmd.args(["-e", &format!("{k}={v}")]);
        }
        cmd.arg(&spec.image);
        let out = cmd
            .output()
            .map_err(|e| RuntimeError::Start(spec.name.clone(), e.to_string()))?;
        if !out.status.success() {
            return Err(RuntimeError::Start(
                spec.name.clone(),
                String::from_utf8_lossy(&out.stderr).to_string(),
            ));
        }
        let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
        // Resolve the published port for the module's service port (8080).
        let port_out = Command::new("docker")
            .args(["port", &id, "8080/tcp"])
            .output()
            .map_err(|e| RuntimeError::Start(spec.name.clone(), e.to_string()))?;
        let mapping = String::from_utf8_lossy(&port_out.stdout);
        let port = mapping
            .lines()
            .next()
            .and_then(|l| l.rsplit(':').next())
            .unwrap_or("8080")
            .trim();
        Ok(RunningHandle {
            id,
            address: format!("http://{}:{port}", self.host),
        })
    }

    async fn stop(&self, handle: &RunningHandle) -> Result<(), RuntimeError> {
        let out = std::process::Command::new("docker")
            .args(["rm", "-f", &handle.id])
            .output()
            .map_err(|e| RuntimeError::Stop(handle.id.clone(), e.to_string()))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(RuntimeError::Stop(
                handle.id.clone(),
                String::from_utf8_lossy(&out.stderr).to_string(),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Test adapter: in-process HTTP servers.
// ---------------------------------------------------------------------------

/// Behaviour a fake module server should exhibit (selected via the
/// `SUPERAPP_MODULE_BEHAVIOR` env key on the spec).
const BEHAVIOR_ENV: &str = "SUPERAPP_MODULE_BEHAVIOR";

/// SDK version the fake module server reports on `GET /sdk` (TR-09-005). When
/// unset, `/sdk` 404s — modeling a pre-SDK module that reports no version at
/// all (the core treats that as compatible; see `modules::compat`).
const SDK_VERSION_ENV: &str = "SUPERAPP_MODULE_SDK_VERSION";

struct FakeServer {
    task: tokio::task::JoinHandle<()>,
    hits: Arc<AtomicU64>,
}

/// Starts a real local HTTP server per module. Health is `GET /health`; every
/// other path increments a per-module hit counter and echoes. Used to test the
/// gateway/lifecycle/fault paths without Docker.
#[derive(Default)]
pub struct InProcessRuntime {
    servers: Mutex<HashMap<String, FakeServer>>,
}

impl InProcessRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of non-health requests the module server received.
    #[must_use]
    pub fn hits(&self, name: &str) -> u64 {
        self.servers
            .lock()
            .unwrap()
            .get(name)
            .map_or(0, |s| s.hits.load(Ordering::SeqCst))
    }
}

#[async_trait]
impl ContainerRuntime for InProcessRuntime {
    async fn start(&self, spec: &ModuleSpec) -> Result<RunningHandle, RuntimeError> {
        use axum::extract::State;
        use axum::http::StatusCode;
        use axum::response::IntoResponse;
        use axum::routing::any;
        use axum::Router;

        let unhealthy = spec.env.get(BEHAVIOR_ENV).map(String::as_str) == Some("unhealthy");
        let sdk_version = spec.env.get(SDK_VERSION_ENV).cloned();
        let hits = Arc::new(AtomicU64::new(0));
        let name = spec.name.clone();

        let hits_for_app = hits.clone();
        let module_name = name.clone();
        let app = Router::new()
            .route(
                "/health",
                any(move || async move {
                    if unhealthy {
                        StatusCode::SERVICE_UNAVAILABLE
                    } else {
                        StatusCode::OK
                    }
                }),
            )
            .route(
                "/sdk",
                any(move || {
                    let sdk_version = sdk_version.clone();
                    async move {
                        match sdk_version {
                            Some(v) => {
                                axum::Json(serde_json::json!({ "sdkVersion": v })).into_response()
                            }
                            None => StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                }),
            )
            .fallback(any(
                |State((hits, module)): State<(Arc<AtomicU64>, String)>,
                 req: axum::extract::Request| async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    let path = req.uri().path().to_string();
                    axum::Json(serde_json::json!({ "module": module, "path": path }))
                },
            ))
            .with_state((hits_for_app, module_name));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| RuntimeError::Start(name.clone(), e.to_string()))?;
        let addr = listener
            .local_addr()
            .map_err(|e| RuntimeError::Start(name.clone(), e.to_string()))?;
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        self.servers
            .lock()
            .unwrap()
            .insert(name.clone(), FakeServer { task, hits });

        Ok(RunningHandle {
            id: name,
            address: format!("http://{addr}"),
        })
    }

    async fn stop(&self, handle: &RunningHandle) -> Result<(), RuntimeError> {
        if let Some(server) = self.servers.lock().unwrap().remove(&handle.id) {
            server.task.abort();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_host_is_loopback() {
        assert_eq!(DockerRuntime::with_host(None).host, "127.0.0.1");
    }

    #[test]
    fn blank_env_value_falls_back_to_loopback() {
        assert_eq!(DockerRuntime::with_host(Some("   ".to_string())).host, "127.0.0.1");
    }

    #[test]
    fn configured_host_is_used_verbatim() {
        assert_eq!(
            DockerRuntime::with_host(Some("gateway.internal".to_string())).host,
            "gateway.internal"
        );
    }

    #[test]
    fn no_network_by_default() {
        assert_eq!(DockerRuntime::with_host_and_network(None, None).network, None);
    }

    #[test]
    fn blank_network_is_treated_as_unset() {
        assert_eq!(
            DockerRuntime::with_host_and_network(None, Some("  ".to_string())).network,
            None
        );
    }

    #[test]
    fn configured_network_is_used_verbatim() {
        assert_eq!(
            DockerRuntime::with_host_and_network(None, Some("superapp".to_string())).network,
            Some("superapp".to_string())
        );
    }
}
