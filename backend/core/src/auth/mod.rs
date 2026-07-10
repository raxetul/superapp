//! Authentication & authorization plumbing (P4).
//!
//! - [`config`] — OIDC/Rauthy RP settings and the self-registration toggle.
//! - [`token`] — JWKS-based access-token validation (TR-04-001, TR-04-010).
//! - [`oidc`] — OIDC discovery + authorization-code flow (TR-04-001).
//! - [`refresh`] — Redis-backed refresh-token store with rotation (TR-04-003).
//! - [`provisioning`] — email-keyed user provisioning, admin bootstrap,
//!   self-registration toggle and allow-list gate (TR-04-004/011/012/013).
//! - [`extractor`] — the current-user Axum extractor + API-key auth
//!   (TR-04-002, TR-04-009).

pub mod config;
pub mod extractor;
pub mod oidc;
pub mod provisioning;
pub mod refresh;
pub mod service;
pub mod state;
pub mod token;
