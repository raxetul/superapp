//! Initializer that installs the HTTP metrics middleware (TR-03-007).
//!
//! Wired via [`Hooks::initializers`](loco_rs::app::Hooks::initializers); its
//! [`after_routes`](loco_rs::app::Initializer::after_routes) layers
//! [`crate::metrics::track`] over the whole router so every request is counted.

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    app::{AppContext, Initializer},
    Result,
};

use crate::metrics::track;

/// Installs the request-counting middleware.
pub struct MetricsInitializer;

#[async_trait]
impl Initializer for MetricsInitializer {
    fn name(&self) -> String {
        "metrics".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router.layer(axum::middleware::from_fn(track)))
    }
}
