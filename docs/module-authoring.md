# Authoring a SuperApp module

TR-09-008. Everything a module author needs to build, sign, register, and
distribute a module — enough to build and register the reference module
(`backend/modules/reference`, `frontend/modules/reference`,
`mobile/modules/reference`) by following this document alone.

## 1. Architecture (locked)

Modules run **out-of-process**, as containers, behind the core's **gateway**.
The core starts/health-checks/proxies/stops each module and enforces Cedar
authorization at the gateway edge, *before* a request ever reaches the
module. Distribution is via **signed OCI images** pushed to a **self-hosted
private registry** — never a public package registry. See
[`docs/phases/p5-dynamic-modules.md`](phases/p5-dynamic-modules.md) for the
runtime/gateway design this builds on.

## 2. The service contract

Every module container exposes the fixed HTTP surface documented in
[`docs/module-contract.openapi.yaml`](module-contract.openapi.yaml):
`/health`, `/ready`, `/sdk`, `/manifest`, `/config` (GET/PUT). Your own
business routes (the manifest's declared `endpoints`) sit alongside these.
Any language may implement this contract; the Rust SDK (`backend/sdk`) is
one implementation of it.

```rust
use superapp_module_sdk::{Manifest, ModuleServer};

let manifest = Manifest::new("reference", "1.0.0")
    .endpoint("GET", "/items", Some("reference:read"))
    .permission("reference:read")
    .config_schema(serde_json::json!({
        "type": "object",
        "required": ["greeting"],
        "properties": { "greeting": { "type": "string" } }
    }));

let router = ModuleServer::new(manifest)
    .initial_config(serde_json::json!({ "greeting": "hello" }))
    .merge(my_business_routes)
    .build();
// axum::serve(listener, router).await
```

## 3. The manifest (TR-09-004)

One canonical shape, shared by every platform and by
`POST /api/v1/modules/register`:
[`schemas/module-manifest.schema.json`](../schemas/module-manifest.schema.json).

| Field | Meaning |
|---|---|
| `name`, `version` | identity |
| `endpoints[]` | `{method, path, permission?}` — routes the gateway proxies; `permission` is the Cedar action required to call it |
| `permissions[]` | permissions this module declares/consumes |
| `config_schema` | JSON Schema validated against `PUT /modules/{id}/config` |
| `signatures[]` | detached ed25519 signatures over the **code artifact** (below) |

Every SDK (`backend/sdk`, `frontend/sdk`, `mobile/sdk`) declares its own copy
of this shape and is tested against the same schema file — see
`docs/phases/p9-module-sdk.md`'s requirement table for the exact tests. They
must never diverge from `backend/core`'s `Manifest`.

## 4. Permissions (Cedar) and lifecycle

An endpoint's `permission` is enforced by the gateway's `Enforcer` as
`Action::"<permission>"` on `Resource::Module::"<name>"` — a denied request
never reaches your module. Grant it to a role via a Cedar policy dropped into
`backend/core/authz/policies/` (reloadable, no recompile).

Lifecycle: the core starts your container, polls `GET /health` until ready
(or times out and stops it), then proxies your `endpoints[]`. It queries
`GET /sdk` once at load and rejects an incompatible SDK major version before
your module is ever marked ready (TR-09-005) — see §6.

## 5. Config

Config is validated against `config_schema` both by the core (at
`PUT /modules/{id}/config`) and, if you use the SDK's `ModuleServer`, by your
own `/config` endpoint (same validator logic, `config_schema` module in each
SDK) — so a bad config is rejected consistently everywhere (TR-09-004).

## 6. SDK versioning (TR-09-005)

Each SDK exposes `SDK_VERSION` (e.g. `"1.0.0"`) and a major-version
compatibility rule. The **backend** core checks it live at `ModuleRegistry::load`
(`GET /sdk`, rejecting a mismatched major with a clear `LoadError::IncompatibleSdk`).
The **frontend/mobile** hosts check a module's declared `sdkVersion` field at
`ModuleRegistry.register()`, same rule, before `initialize` ever runs. A
module that declares no version is always treated as compatible (pre-SDK).

## 7. Signing (TR-05-002) and packaging (TR-09-006)

Signatures cover the **code artifact** only — `name`, `version`, `endpoints`,
`permissions`, `config_schema`, with recursively sorted object keys,
serialized compact — never `signatures` or runtime config. This means
changing your config never invalidates your signature, but changing a route
does.

```bash
# Generate a signing keypair (Rust module authors: superapp_module_sdk::signing::ModuleSigner works too)
node scripts/module-sdk/package-module.mjs generate-key --out signer.pem
# → prints the public key (base64) to hand to the core operator for
#   `modules.trusted_signers` in config/*.yaml

# Sign a manifest
node scripts/module-sdk/package-module.mjs sign \
  --manifest manifest.json --key signer.pem --signer my-module-ci \
  --out signed-manifest.json
```

The core accepts a module iff **at least one** signature comes from a
trusted signer and validates over the code artifact
(`backend/core/src/modules/signing.rs::verify`).

## 8. Scaffolding a new module (TR-09-006, SHOULD)

```bash
node scripts/module-sdk/scaffold-module.mjs my-module --permission my-module:read
```

Produces a buildable skeleton in all three trees —
`backend/modules/my-module`, `frontend/modules/my-module`,
`mobile/modules/my-module` — wired to the SDKs and matching the reference
module's structure. Refuses to overwrite an existing module directory.

## 9. Distribution (TR-09-009)

Build your module's Docker image and push it to the **self-hosted private
registry** (never a public registry):

```bash
docker build -t registry.superapp.internal/modules/my-module:1.0.0 .
docker push registry.superapp.internal/modules/my-module:1.0.0
```

The core resolves an image reference from a module's `name`+`version`
against the configured `modules.registry_host` setting
(`backend/core/src/modules/oci.rs::resolve_image_ref`) — never a public
registry shorthand. **Building and pushing the image is a CI/deployment
concern this repo's environment cannot exercise** (no Docker daemon here);
see `docs/phases/p9-module-sdk.md` for what's proven vs. deferred.

## 10. Register it

```bash
curl -X POST http://localhost:3000/api/v1/modules/register \
  -H "Authorization: Bearer <admin token>" \
  --data @signed-manifest.json
```

`422` on structural/config-schema errors, `403` on an untrusted/invalid
signature, `409` on a duplicate `name`+`version`, `200` with the module's id
on success.
