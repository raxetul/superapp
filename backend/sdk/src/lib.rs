//! SuperApp backend module SDK (TR-09-001).
//!
//! Implements the backend module **service contract**: the fixed HTTP surface
//! ([`server::ModuleServer`]) every module container exposes to the core
//! gateway (lifecycle/readiness, declared routes, permissions, config,
//! health), plus the canonical [`manifest::Manifest`] type (TR-09-004),
//! config-schema validation (mirrors TR-05-006), a signing helper
//! (TR-05-002), and SDK version/compatibility (TR-09-005).
//!
//! A module may be written in **any** language that implements the contract
//! documented at `docs/module-contract.openapi.yaml`; this crate is the Rust
//! implementation of it.

pub mod config_schema;
pub mod manifest;
pub mod server;
pub mod signing;
pub mod version;

pub use manifest::{Endpoint, Manifest, Signature};
pub use server::ModuleServer;
pub use signing::ModuleSigner;
pub use version::SDK_VERSION;
