//! Dependency readiness probing for the `/ready` endpoint (TR-03-006).
//!
//! Liveness (`/health`) merely reflects that the process is up. Readiness
//! aggregates reachability of the backend's runtime dependencies —
//! PostgreSQL (always), plus Redis and Kafka when their addresses are
//! configured — and is only "ready" when every probed dependency is up.

use std::time::Duration;

use loco_rs::app::AppContext;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

/// Default per-dependency probe timeout.
pub const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// Reachability state of a single dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DepState {
    /// Dependency is reachable.
    Up,
    /// Dependency is unreachable.
    Down,
}

/// Reachability report for one dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DepStatus {
    /// Logical dependency name, e.g. `postgres`, `redis`, `kafka`.
    pub name: String,
    /// Whether it is up or down.
    pub state: DepState,
    /// Failure detail when down (omitted when up).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl DepStatus {
    /// A reachable dependency.
    #[must_use]
    pub fn up(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: DepState::Up,
            detail: None,
        }
    }

    /// An unreachable dependency, with a failure detail.
    #[must_use]
    pub fn down(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: DepState::Down,
            detail: Some(detail.into()),
        }
    }

    /// Whether this dependency is reachable.
    #[must_use]
    pub fn is_up(&self) -> bool {
        self.state == DepState::Up
    }
}

/// `true` iff every dependency in `statuses` is up (an empty slice is ready).
#[must_use]
pub fn all_up(statuses: &[DepStatus]) -> bool {
    statuses.iter().all(DepStatus::is_up)
}

/// Probe a TCP endpoint (`host:port`) for reachability within `timeout`.
pub async fn probe_tcp(name: &str, addr: &str, timeout: Duration) -> DepStatus {
    match tokio::time::timeout(timeout, TcpStream::connect(addr)).await {
        Ok(Ok(_stream)) => DepStatus::up(name),
        Ok(Err(err)) => DepStatus::down(name, err.to_string()),
        Err(_elapsed) => DepStatus::down(name, format!("timed out after {}ms", timeout.as_millis())),
    }
}

/// Probe the primary PostgreSQL connection via a pool ping.
pub async fn probe_db(ctx: &AppContext) -> DepStatus {
    match ctx.db.ping().await {
        Ok(()) => DepStatus::up("postgres"),
        Err(err) => DepStatus::down("postgres", err.to_string()),
    }
}

/// Readiness dependency addresses, sourced from loco `settings.readiness`.
///
/// Addresses are optional: an unset dependency is not probed. Dev/prod configs
/// set them to the real infrastructure endpoints; the test profile omits them
/// so `/ready` is deterministic (DB-only). The Redis/Kafka probe logic itself
/// is covered by unit tests that hit real reachable/closed ports.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReadinessSettings {
    /// `host:port` of Redis, if it should be probed.
    pub redis: Option<String>,
    /// `host:port` of a Kafka broker, if it should be probed.
    pub kafka: Option<String>,
}

/// Read `settings.readiness` from the loco config (defaults to all-unset).
#[must_use]
pub fn readiness_settings(ctx: &AppContext) -> ReadinessSettings {
    ctx.config
        .settings
        .as_ref()
        .and_then(|s| s.get("readiness"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// Run every configured readiness probe and return their statuses.
pub async fn collect_readiness(ctx: &AppContext) -> Vec<DepStatus> {
    let settings = readiness_settings(ctx);
    let mut statuses = vec![probe_db(ctx).await];
    if let Some(addr) = settings.redis.as_deref() {
        statuses.push(probe_tcp("redis", addr, DEFAULT_PROBE_TIMEOUT).await);
    }
    if let Some(addr) = settings.kafka.as_deref() {
        statuses.push(probe_tcp("kafka", addr, DEFAULT_PROBE_TIMEOUT).await);
    }
    statuses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_tcp_reports_up_for_a_listening_socket() {
        // Bind an ephemeral listener and probe it → up.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let status = probe_tcp("redis", &addr, DEFAULT_PROBE_TIMEOUT).await;
        assert_eq!(status.state, DepState::Up);
        assert!(status.is_up());
    }

    #[tokio::test]
    async fn probe_tcp_reports_down_for_a_closed_port() {
        // Bind then immediately drop to obtain a very-likely-closed port.
        let addr = {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            l.local_addr().unwrap().to_string()
        };
        let status = probe_tcp("kafka", &addr, Duration::from_millis(200)).await;
        assert_eq!(status.state, DepState::Down);
        assert!(status.detail.is_some());
    }

    #[test]
    fn all_up_is_true_only_when_every_dependency_is_up() {
        assert!(all_up(&[DepStatus::up("postgres"), DepStatus::up("redis")]));
        assert!(!all_up(&[
            DepStatus::up("postgres"),
            DepStatus::down("kafka", "refused"),
        ]));
        // An empty set (nothing to check) is considered ready.
        assert!(all_up(&[]));
    }
}
