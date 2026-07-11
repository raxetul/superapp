//! Dynamic module loading (P5).
//!
//! Modules run as **out-of-process containers**; the core acts as a
//! **gateway** that verifies, starts, health-checks, proxies, and stops them,
//! enforcing Cedar authorization at the gateway edge.
//!
//! - [`manifest`] — the module manifest + the signed code artifact (TR-05-003/002).
//! - [`signing`] — ed25519 signature verification + self-signing key bootstrap
//!   and trust store (TR-05-002/009).
//! - [`config_schema`] — JSON-Schema validation of module config (TR-05-006).
//! - [`runtime`] — the `ContainerRuntime` seam + fake/real adapters
//!   (TR-05-001/004).
//! - [`registry`] — lifecycle management + gateway proxying + fault isolation
//!   + Cedar enforcement (TR-05-001/004/005/007/008).

pub mod config_schema;
pub mod manifest;
pub mod registry;
pub mod runtime;
pub mod signing;
