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

/// Runs modules as containers via the `docker` CLI.
pub struct DockerRuntime {
    /// Host the mapped port is reachable on.
    host: String,
}

impl Default for DockerRuntime {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
        }
    }
}

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    async fn start(&self, spec: &ModuleSpec) -> Result<RunningHandle, RuntimeError> {
        use std::process::Command;
        let mut cmd = Command::new("docker");
        cmd.args(["run", "-d", "-P"]);
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
        use axum::routing::any;
        use axum::Router;

        let unhealthy = spec.env.get(BEHAVIOR_ENV).map(String::as_str) == Some("unhealthy");
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
