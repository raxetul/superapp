//! Process-wide Prometheus metrics (TR-03-007).
//!
//! A tiny dependency-free registry: [`track`] is an Axum middleware that counts
//! every HTTP request by method and status, and [`Metrics::render`] emits the
//! Prometheus text exposition format served at `/metrics`. Kept hand-rolled to
//! avoid pulling metrics-exporter crates that break the repo's rustc 1.85 MSRV.

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        LazyLock, Mutex,
    },
    time::Instant,
};

use axum::{extract::Request, middleware::Next, response::Response};

/// Prometheus text exposition content type (version 0.0.4).
pub const PROMETHEUS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Global metrics registry, shared by the [`track`] middleware and the
/// `/metrics` handler.
pub static METRICS: LazyLock<Metrics> = LazyLock::new(Metrics::new);

/// In-memory metric store.
pub struct Metrics {
    start: Instant,
    requests_total: AtomicU64,
    /// `(method, status)` → count.
    by_key: Mutex<BTreeMap<(String, u16), u64>>,
}

impl Metrics {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            requests_total: AtomicU64::new(0),
            by_key: Mutex::new(BTreeMap::new()),
        }
    }

    /// Record one completed HTTP request.
    pub fn record(&self, method: &str, status: u16) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        let mut by_key = self.by_key.lock().expect("metrics mutex poisoned");
        *by_key.entry((method.to_string(), status)).or_insert(0) += 1;
    }

    /// Render the current metrics in Prometheus text exposition format.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("# HELP http_requests_total Total number of HTTP requests processed.\n");
        out.push_str("# TYPE http_requests_total counter\n");
        {
            let by_key = self.by_key.lock().expect("metrics mutex poisoned");
            for ((method, status), count) in by_key.iter() {
                out.push_str(&format!(
                    "http_requests_total{{method=\"{method}\",status=\"{status}\"}} {count}\n"
                ));
            }
        }
        out.push_str("# HELP process_uptime_seconds Seconds since process start.\n");
        out.push_str("# TYPE process_uptime_seconds gauge\n");
        out.push_str(&format!(
            "process_uptime_seconds {:.3}\n",
            self.start.elapsed().as_secs_f64()
        ));
        out
    }
}

/// Axum middleware that records every request into [`METRICS`] after it runs.
pub async fn track(request: Request, next: Next) -> Response {
    let method = request.method().as_str().to_string();
    let response = next.run(request).await;
    METRICS.record(&method, response.status().as_u16());
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_emits_prometheus_counter_and_gauge() {
        let m = Metrics::new();
        m.record("GET", 200);
        m.record("GET", 200);
        m.record("POST", 422);
        let text = m.render();

        assert!(text.contains("# TYPE http_requests_total counter"));
        assert!(text.contains("http_requests_total{method=\"GET\",status=\"200\"} 2"));
        assert!(text.contains("http_requests_total{method=\"POST\",status=\"422\"} 1"));
        assert!(text.contains("# TYPE process_uptime_seconds gauge"));
        assert!(text.contains("process_uptime_seconds "));
    }
}
