//! Auth initializer (P4 composition root).
//!
//! Builds the [`AuthState`] from configuration once at startup and layers it —
//! plus the token validator — into the router as Axum `Extension`s, so
//! controllers and the [`CurrentUser`](crate::auth::extractor::CurrentUser)
//! extractor receive their collaborators by injection.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Extension;
use axum::Router as AxumRouter;
use loco_rs::prelude::*;

use crate::auth::config::{self_registration_enabled, AuthSettings, ModulesSettings};
use crate::auth::state::AuthState;

/// Default location of the Cedar policy files (relative to the working dir).
const DEFAULT_POLICIES_DIR: &str = "authz/policies";

pub struct AuthInitializer;

#[async_trait]
impl Initializer for AuthInitializer {
    fn name(&self) -> String {
        "auth".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let settings = AuthSettings::from_settings(ctx.config.settings.as_ref())
            .map_err(|e| Error::Message(format!("invalid settings.auth: {e}")))?;
        let modules_settings = ModulesSettings::from_settings(ctx.config.settings.as_ref())
            .map_err(|e| Error::Message(format!("invalid settings.modules: {e}")))?;

        let policies_dir = PathBuf::from(DEFAULT_POLICIES_DIR);
        let state = AuthState::build(
            &settings,
            &modules_settings,
            ctx.db.clone(),
            &policies_dir,
            self_registration_enabled(),
        )
        .await
        .map_err(|e| Error::Message(format!("auth state: {e}")))?;

        let state = Arc::new(state);
        let mut router = router.layer(Extension(state.clone()));
        if let Some(validator) = state.validator.clone() {
            router = router.layer(Extension(validator));
        }
        Ok(router)
    }
}
