//! TR-09-007 (backend): the reference module, proven end-to-end against
//! **real** module code — registers via `/modules/register`, loads through
//! `ModuleRegistry`, and its route is Cedar-gated at the `Gateway` before
//! proxying, with health reporting status throughout.
//!
//! No Docker daemon is available in this environment, so the container-
//! runtime seam (the project's established DI pattern — see P5) is swapped
//! for one that runs the actual `reference_module::router()` in-process; the
//! module's own code, manifest, config schema, and HTTP contract are all
//! real, unmodified reference-module code, not a test double of it.
//!
//! TR-09-004 bonus: the reference module's manifest is built with the
//! *frontend-independent* `superapp-module-sdk` crate's own `Manifest` type
//! and converted to the core's `Manifest` via a JSON round-trip below — since
//! both types serialize to the identical canonical shape, this is itself a
//! proof the two independently-declared types never drifted apart.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue};
use serde_json::json;
use serial_test::serial;

use superapp_core::app::App;
use superapp_core::authz::engine::{AuthzError, PolicyEngine};
use superapp_core::authz::entities::{parse_entities, EntityProvider};
use superapp_core::authz::Enforcer;
use superapp_core::models::_entities::users;
use superapp_core::modules::manifest::Manifest as CoreManifest;
use superapp_core::modules::registry::{Gateway, GatewayOutcome, ModuleHealth, ModuleRegistry};
use superapp_core::modules::runtime::{ContainerRuntime, ModuleSpec, RunningHandle, RuntimeError};

use crate::support;

/// Convert the SDK's independently-declared `Manifest` to the core's, via a
/// JSON round-trip (see module docs — this is itself a TR-09-004 proof). Left
/// generic over `Serialize` so this test crate needs no direct dependency on
/// `superapp-module-sdk` (only on `reference-module`, which re-exports it).
fn to_core_manifest(m: &impl serde::Serialize) -> CoreManifest {
    let json = serde_json::to_string(m).expect("SDK manifest serializes");
    serde_json::from_str(&json).expect("core Manifest parses the SDK manifest's JSON verbatim")
}

/// Runs the **real** reference-module router in-process (no Docker) — the
/// only thing swapped is the container-runtime seam.
#[derive(Default)]
struct RealReferenceModuleRuntime {
    tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

#[async_trait]
impl ContainerRuntime for RealReferenceModuleRuntime {
    async fn start(&self, spec: &ModuleSpec) -> Result<RunningHandle, RuntimeError> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| RuntimeError::Start(spec.name.clone(), e.to_string()))?;
        let addr = listener
            .local_addr()
            .map_err(|e| RuntimeError::Start(spec.name.clone(), e.to_string()))?;
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, reference_module::router()).await;
        });
        self.tasks.lock().unwrap().push(task);
        Ok(RunningHandle {
            id: spec.name.clone(),
            address: format!("http://{addr}"),
        })
    }

    async fn stop(&self, _handle: &RunningHandle) -> Result<(), RuntimeError> {
        if let Some(task) = self.tasks.lock().unwrap().pop() {
            task.abort();
        }
        Ok(())
    }
}

async fn seed_user(db: &sea_orm::DatabaseConnection, email: &str, role: &str) {
    users::ActiveModel {
        email: ActiveValue::set(email.to_string()),
        name: ActiveValue::set(email.split('@').next().unwrap().to_string()),
        password: ActiveValue::set("!oidc!".to_string()),
        role: ActiveValue::set(role.to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

#[tokio::test]
#[serial]
async fn reference_module_registers_via_the_real_manifest() {
    request::<App, _, _>(|request, ctx| async move {
        seed_user(&ctx.db, "boss@example.com", "admin").await;

        let mut manifest = to_core_manifest(&reference_module::manifest());
        support::sign_manifest(&mut manifest);

        let res = request
            .post("/api/v1/modules/register")
            .add_header("authorization", &support::bearer("boss@example.com"))
            .text(&serde_json::to_string(&manifest).unwrap())
            .await;
        assert_eq!(res.status_code(), 200, "body: {}", res.text());
        let body: serde_json::Value = res.json();
        assert_eq!(body["data"]["name"], json!(reference_module::NAME));
    })
    .await;
}

struct StaticEntities(serde_json::Value);
#[async_trait]
impl EntityProvider for StaticEntities {
    async fn entities_json(&self) -> Result<serde_json::Value, AuthzError> {
        Ok(self.0.clone())
    }
    async fn entities(&self) -> Result<cedar_policy::Entities, AuthzError> {
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
async fn reference_module_loads_and_its_route_is_cedar_gated_end_to_end() {
    let rt = Arc::new(RealReferenceModuleRuntime::default());
    let reg = Arc::new(ModuleRegistry::new(rt));
    reg.load(
        to_core_manifest(&reference_module::manifest()),
        &ModuleSpec::new(
            reference_module::NAME,
            reference_module::VERSION,
            "registry.superapp.internal/modules/reference:1.0.0",
        ),
    )
    .await
    .expect("the real reference-module router starts and becomes ready");
    assert!(reg.is_loaded(reference_module::NAME));

    let gw = Gateway::new(reg.clone(), enforcer());

    // No policy grants `reference:read` to a plain user → denied before the
    // module ever sees the request (TR-05-007 / TR-09-007).
    let denied = gw
        .handle("User::\"bob@x\"", reference_module::NAME, "GET", "/items")
        .await;
    assert_eq!(denied, GatewayOutcome::Forbidden);

    // Admin is allowed (admin-full-access) → proxied through to the real
    // reference-module code.
    let allowed = gw
        .handle("User::\"boss@x\"", reference_module::NAME, "GET", "/items")
        .await;
    match allowed {
        GatewayOutcome::Proxied { status, body } => {
            assert_eq!(status, 200);
            assert!(body.contains("hello from the reference module"));
        }
        other => panic!("expected proxied 200, got {other:?}"),
    }

    // Health reports status (TR-05-005 / TR-09-007).
    assert_eq!(
        reg.health(reference_module::NAME).await,
        ModuleHealth::Healthy
    );
}
