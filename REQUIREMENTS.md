# SuperApp — Requirements

Requirements catalog driving agentic development. Two groups: **Feature** (`FR-*`, user/product-facing capabilities) and **Technical** (`TR-*`, stack, infra, cross-cutting quality). Each requirement is independently implementable and verifiable — the **Accept** line is the done-signal an agent codes against.

**Development is test-driven (TDD).** The **Accept** criteria of every requirement are the test contract: write them as failing automated tests first (red), implement until green, then refactor. See [TR-00-001](#tr-00-001--test-driven-development-tdd).

**Conventions**
- **ID format:** `TR-PP-NNN` / `FR-PP-NNN` — `PP` = zero-padded number of the phase that delivers the requirement (`00` = cross-cutting, applies to all phases); `NNN` = zero-padded sequence **within that phase + group**, starting at `001`. IDs are stable and never reused; a requirement is renumbered only if it is re-assigned to a different phase.
- Priority: MUST · SHOULD · COULD.
- Each requirement is tagged with the phase(s) that deliver it; see the [Phase ↔ Requirement Mapping](#phase--requirement-mapping) at the bottom. The `PP` in the ID must match the `**Phase:**` tag.
- `PHASES.md` is regenerated from this catalog once requirements stabilize.

---

## Feature Requirements

### FR-07-001 — Login & logout (Rauthy)
**Phase:** P7 · **Priority:** MUST
The frontend shall provide login via Rauthy (supporting both SSO and username/password methods) and a logout that clears the session.
**Accept:** A user logs in via Rauthy (SSO or username/password) and reaches the app; logout clears tokens/session; no standalone registration is shown unless enabled (FR-07-004).

### FR-07-002 — Role-based UI adaptation
**Phase:** P7 · **Priority:** MUST
The interface shall adapt to the user's role (Admin vs User).
**Accept:** Admin sees management/config/module-admin sections; a regular user does not; gated by role.

### FR-07-003 — Admin user-management UI (allow-list & roles)
**Phase:** P7 · **Priority:** MUST
Admins shall be able to allow-list/invite users by **email** and manage user roles via the UI. (This is the only onboarding path while self-registration is disabled — the default.)
**Accept:** An admin adds an email to the allow-list and promotes/demotes a user through the UI; non-admins cannot reach these screens.

### FR-07-004 — Conditional self-registration UI
**Phase:** P7 · **Priority:** SHOULD
The frontend shall present a self-registration option only when the backend reports self-registration enabled (via a public auth-capabilities endpoint).
**Accept:** Registration UI is shown when enabled and hidden when disabled; a successful registration yields a least-privilege user.

### FR-08-001 — Login & logout (Rauthy)
**Phase:** P8 · **Priority:** MUST
The mobile app shall provide login via Rauthy (supporting both SSO and username/password methods) and a logout that clears the session.
**Accept:** A user logs in via Rauthy and reaches the app; logout clears the session/tokens.

### FR-08-002 — Role-based navigation
**Phase:** P8 · **Priority:** MUST
The mobile navigation shall adapt to the user's role (Admin vs User).
**Accept:** Admin sees admin screens/sections; a regular user does not.

### FR-08-003 — Admin management screens (allow-list & roles)
**Phase:** P8 · **Priority:** MUST
The mobile app shall provide admin screens to allow-list/invite users by **email** and manage user roles (parity with the web admin UI).
**Accept:** An admin allow-lists an email and changes a user's role on-device; non-admins cannot reach these screens.

### FR-08-004 — Conditional self-registration UI
**Phase:** P8 · **Priority:** SHOULD
The mobile app shall present a self-registration option only when the backend reports self-registration enabled.
**Accept:** The registration option appears when enabled and is hidden when disabled; a successful registration yields a least-privilege user.

---

## Technical Requirements

### TR-01-001 — Monorepo structure
**Phase:** P1 · **Priority:** MUST
The repo shall contain `backend/` (Rust), `frontend/` (React), and `mobile/` (React Native), each with a `modules/` area for plugins, plus shared docs at root.
**Accept:** Directory tree exists and matches `project.md`; each app builds independently.

### TR-01-002 — Rust backend
**Phase:** P1 · **Priority:** MUST
The entire backend shall be implemented in Rust.
**Accept:** No non-Rust backend services; `cargo build` produces all backend binaries.

### TR-01-003 — Data & infrastructure stack
**Phase:** P1 · **Priority:** MUST
The backend shall use PostgreSQL (primary store), Redis (cache + refresh-token store), Kafka (async messaging), and Prometheus + Grafana (metrics/observability).
**Accept:** Each dependency is declared in config and reachable from a healthcheck.

### TR-01-004 — Client stacks
**Phase:** P1 · **Priority:** MUST
Frontend shall use React + ShadCN UI; mobile shall use React Native + Tamagui UI.
**Accept:** Both apps scaffold and render a baseline screen with the chosen UI kit.

### TR-01-005 — Conventional Commits
**Phase:** P1 · **Priority:** SHOULD
All commits shall follow Conventional Commits.
**Accept:** Commit messages parse as `type(scope): subject`; optionally enforced by a commit hook.

### TR-01-006 — Documented engineering standards
**Phase:** P1 · **Priority:** MUST
Naming, package layout, API conventions, and error/response formats shall be documented as the single source of truth.
**Accept:** `project.md` (realigned to Rust) covers backend/frontend/mobile standards and API conventions.

### TR-02-001 — Internal infrastructure compose stack
**Phase:** P2 · **Priority:** MUST
The repo shall provide a Docker Compose stack that provisions the infrastructure (PostgreSQL, Redis, Kafka, Prometheus, Grafana) locally, with containers named using the `superapp-` prefix.
**Accept:** `docker compose up` starts all five services as `superapp-<service>`; each passes its healthcheck; `docker compose down` tears down cleanly; named volumes persist data across restarts.

### TR-02-002 — Environment configuration scaffolding
**Phase:** P2 · **Priority:** MUST
A committed `.env.example` shall enumerate every required variable (infrastructure connection strings, ports, secret placeholders) using the `SUPERAPP_BACKEND_` prefix, with safe defaults/docs; apps load all config from env and the real `.env` is gitignored.
**Accept:** Copying `.env.example`→`.env` and filling values yields a working boot; a missing required variable fails fast with a clear message; `.env` is gitignored (no secrets committed).

### TR-02-003 — App-to-infrastructure networking
**Phase:** P2 · **Priority:** MUST
The compose stack shall place app and infrastructure services on a shared `superapp` network so infrastructure is reachable by service name (no hardcoded host IPs).
**Accept:** From its container, the backend reaches `superapp-postgres`, `superapp-redis`, and `superapp-kafka` by service name on the `superapp` network.

### TR-02-004 — loco configuration profiles
**Phase:** P2 · **Priority:** SHOULD
The backend shall provide loco `config/{development,test,production}.yaml` profiles that source database/Redis/Kafka connection info from env, with no secrets committed.
**Accept:** Each profile loads the correct connection settings from env; `cargo loco start` connects to the infrastructure stack in development; the test profile targets an isolated test database.

### TR-02-005 — Dependency connectivity check
**Phase:** P2 · **Priority:** SHOULD
A documented command/script shall verify the app can reach every infrastructure dependency before startup.
**Accept:** The check reports reachability of PostgreSQL, Redis, and Kafka (and the Prometheus/Grafana endpoints) and exits non-zero with a clear message when any is unreachable.

### TR-02-006 — Developer task-runner entrypoints
**Phase:** P2 · **Priority:** COULD
A task runner (Makefile/justfile) shall wrap common local-dev operations for the SuperApp services (`up`, `down`, `logs`, `ps`) plus an `env:check` connectivity target.
**Accept:** Each target performs its documented action.

### TR-03-001 — loco application scaffold & boot
**Phase:** P3 · **Priority:** MUST
`backend/core` shall be a loco.rs application that boots via `cargo loco start` and serves a versioned API base at `/api/v1`.
**Accept:** `cargo build` produces the server binary; `cargo loco start` boots and serves; a baseline route under `/api/v1` returns 200; `cargo loco routes` lists the registered routes.

### TR-03-002 — SeaORM database integration & baseline migration
**Phase:** P3 · **Priority:** MUST
The backend shall connect to PostgreSQL through SeaORM with a pooled connection, using the connection settings wired in P2, and provide a baseline migration.
**Accept:** A baseline migration applies and rolls back cleanly via `cargo loco db migrate` / down; entities generate via `cargo loco db entities`; an integration test performs a round-trip query against an isolated test database.

### TR-03-003 — Standard success envelope & RFC 9457 error responses
**Phase:** P3 · **Priority:** MUST
Successful (2xx) HTTP responses shall use the house success envelope (`success`, `data`, `message`, `pagination`) from `project.md`; error (non-2xx) responses shall be **RFC 9457 Problem Details** documents served as `application/problem+json` (members `type`, `title`, `status`, `detail`, `instance`, plus extension members). Both shall be implemented as shared Rust types. The house envelope is not used for errors.
**Accept:** A success response serializes to the house envelope and an error response serializes to an RFC 9457 problem document with `Content-Type: application/problem+json` (schema tests for both); controllers return the typed success/problem types rather than ad-hoc JSON.

### TR-03-004 — Request validation
**Phase:** P3 · **Priority:** MUST
Inbound request payloads shall be validated, returning `422` on failure as an RFC 9457 problem document (TR-03-003) whose `errors` extension member carries the field-level failures.
**Accept:** An endpoint with a validated DTO returns `422` as `application/problem+json` with per-field entries in the `errors` extension member for invalid input, and `2xx` for valid input; tests cover both paths.

### TR-03-005 — Structured logging & request tracing
**Phase:** P3 · **Priority:** MUST
The backend shall emit structured logs with a per-request correlation/request ID propagated through the request lifecycle.
**Accept:** Logs are structured (JSON outside dev); each request log carries a request ID; the ID is returned in a response header.

### TR-03-006 — Health & readiness endpoints
**Phase:** P3 · **Priority:** MUST
The backend shall expose `/health` (liveness) and `/ready` (readiness — checks PostgreSQL/Redis/Kafka reachability).
**Accept:** `/health` returns 200 while the process is up; `/ready` returns 200 only when all dependencies are reachable and `503` with details when one is down; a test simulates an unreachable dependency.

### TR-03-007 — Prometheus metrics endpoint
**Phase:** P3 · **Priority:** SHOULD
The backend shall expose a Prometheus-compatible `/metrics` endpoint.
**Accept:** `/metrics` returns Prometheus text format including default HTTP request metrics; Prometheus (from helvetia-compose) can scrape it.

### TR-04-001 — OIDC SSO via Rauthy
**Phase:** P4 · **Priority:** MUST
The backend shall authenticate users via **Rauthy** (OIDC IdP) using the `openidconnect` RP crate — OIDC discovery, authorization-code flow, and JWKS-based token validation.
**Accept:** A user authenticates through Rauthy; the backend validates the token against Rauthy's JWKS; an invalid or expired token is rejected with `401`.

### TR-04-002 — Auth extractor & middleware
**Phase:** P4 · **Priority:** MUST
A custom Axum extractor/middleware shall validate the Rauthy access token and resolve the current application user; loco's built-in login/registration routes shall be disabled.
**Accept:** A protected route returns `401` without a valid token and `2xx` with one; loco's native auth endpoints are not exposed.

### TR-04-003 — Token lifecycle & rotation
**Phase:** P4 · **Priority:** MUST
The system shall use short-lived access tokens and long-lived refresh tokens with rotation on refresh; refresh tokens shall be stored in Redis.
**Accept:** Refresh issues a new access token and rotates the refresh token; a reused/revoked refresh token is rejected; refresh tokens persist in Redis with a TTL.

### TR-04-004 — Admin bootstrap & user provisioning
**Phase:** P4 · **Priority:** MUST
The first authenticated user shall become admin (bootstrap), independent of the self-registration toggle; users shall be identified by **email** (the OIDC `email` claim) and provisioning shall map a user to a record idempotently by email. Onboarding of all non-first users is governed by TR-04-011 / TR-04-013.
**Accept:** First login → admin role; provisioning is keyed by email and idempotent (repeat logins do not duplicate the user).

### TR-04-005 — Cedar policy enforcement point
**Phase:** P4 · **Priority:** MUST
Authorization decisions shall be made by **Cedar** (`cedar-policy`) via a central enforcement point evaluating `is_authorized(principal, action, resource, context)`.
**Accept:** Protected actions are authorized by Cedar; allow and deny paths are covered by tests; a denial returns `403`.

### TR-04-006 — Cedar schema & policy set
**Phase:** P4 · **Priority:** MUST
The system shall define a Cedar schema (principals, actions, resources incl. modules) and an initial policy set, stored so policies can be reloaded without recompiling.
**Accept:** The schema validates; policies validate against the schema in CI (`cedar validate`); editing a policy file changes the decision without a recompile.

### TR-04-007 — Cedar entity provider
**Phase:** P4 · **Priority:** MUST
The system shall materialize Cedar entities (principal/resource attributes, role memberships) from the database per authorization request, with caching.
**Accept:** Authorization requests resolve entities from current data; a role/attribute change is reflected in decisions within the cache TTL; covered by tests.

### TR-04-008 — Authorization audit logging
**Phase:** P4 · **Priority:** SHOULD
Each authorization decision shall be logged with principal, action, resource, decision, and the determining policy id(s).
**Accept:** Each protected request emits an audit entry containing the decision and matched policy id(s).

### TR-04-009 — Service-to-service API key authentication
**Phase:** P4 · **Priority:** SHOULD
Modules shall authenticate to the core via an `X-API-Key` header.
**Accept:** A valid API key authorizes module→core calls; a missing/invalid key returns `401`; keys are revocable.

### TR-04-010 — Username/password login via Rauthy
**Phase:** P4 · **Priority:** MUST
The product shall support username/password login as a Rauthy-provided method alongside SSO; the backend shall validate all Rauthy-issued tokens uniformly regardless of the login method used.
**Accept:** A user authenticates with username/password through Rauthy and obtains tokens the backend accepts identically to an SSO session.

### TR-04-011 — Startup self-registration toggle (default disabled)
**Phase:** P4 · **Priority:** MUST
A startup environment variable (`SUPERAPP_BACKEND_SELF_REGISTRATION_ENABLED`, default `false`) shall control self-onboarding, applying to **both** SSO and username/password-registration paths; the admin bootstrap (TR-04-004) is exempt.
**Accept:** Unset/false → self-onboarding is denied; true → a previously-unknown user onboarding via SSO or password registration is auto-provisioned; the value is read once at startup.

### TR-04-012 — Least-privilege role for self-onboarded users
**Phase:** P4 · **Priority:** MUST
When self-registration is enabled, auto-provisioned users shall receive the lowest-privilege role (never admin).
**Accept:** A self-onboarded user is created with the least-privilege role.

### TR-04-013 — Admin email allow-list when self-registration disabled
**Phase:** P4 · **Priority:** MUST
When self-registration is disabled, only users whose **email** an admin has pre-authorized shall be able to authenticate via any Rauthy method (incl. SSO); non-allow-listed identities shall be denied and not provisioned. Email (the OIDC `email` claim) is the identity key.
**Accept:** Toggle off + admin allow-lists an email → that user logs in (SSO or password) and is provisioned; a valid Rauthy user whose email is not allow-listed is rejected and no account is created.

### TR-05-001 — Module runtime & gateway routing
**Phase:** P5 · **Priority:** MUST
The core shall run modules as **out-of-process containers** and act as a **gateway**: starting/stopping a module's container and proxying its manifest-declared routes to it at runtime.
**Accept:** A sample module container starts, its declared routes become reachable through the core gateway, and it stops cleanly; a failed or unreachable module container is handled without crashing the core.

### TR-05-002 — Secure module verification
**Phase:** P5 · **Priority:** MUST
Modules shall carry an **array of signatures** (self-signed and/or external); the loader shall verify them before loading and accept the module only if at least one signature from a trusted signer validates. The signature shall cover the module's **immutable code/contract artifacts and exclude its variable/data/config portion**. Modules with no valid trusted signature shall be rejected.
**Accept:** A module with ≥1 valid trusted signature loads; a module whose signatures are all untrusted/invalid is rejected and audit-logged; changing the data/config part does **not** invalidate the signature while changing the code part **does**.

### TR-05-003 — Module registration endpoint
**Phase:** P5 · **Priority:** MUST
`POST /api/v1/modules/register` shall accept a manifest (`name`, `version`, `endpoints`, `permissions`, `config_schema`) and register the module.
**Accept:** Registration returns the module id and persists the manifest; a duplicate name+version is rejected; an invalid manifest returns `422`.

### TR-05-004 — Module lifecycle management
**Phase:** P5 · **Priority:** MUST
The core shall manage each module container's lifecycle — start, readiness, stop — containing failures.
**Accept:** A module container is started and reaches readiness before its routes are served; it stops cleanly on unload; a module that fails to start is rejected without affecting the core or other modules.

### TR-05-005 — Module health checks
**Phase:** P5 · **Priority:** MUST
`GET /api/v1/modules/{id}/health` shall report per-module status.
**Accept:** The endpoint returns healthy/unhealthy per module; an unhealthy module is reported without taking down the core.

### TR-05-006 — Module configuration with schema validation
**Phase:** P5 · **Priority:** MUST
`PUT /api/v1/modules/{id}/config` shall validate updates against the module's `config_schema` (JSON Schema); module env vars use the `SUPERAPP_MODULE_{NAME}_` prefix.
**Accept:** A valid config applies at runtime; an invalid config is rejected (`422`) against the schema.

### TR-05-007 — Module permission registration & gateway enforcement (Cedar)
**Phase:** P5 · **Priority:** MUST
A module's declared permissions shall register as Cedar actions/entities, and the core **gateway** shall enforce them before proxying to the module container.
**Accept:** After registration, the module's actions are authorizable via Cedar; a request to a module route without the required policy is denied at the gateway (`403`) and never reaches the module container.

### TR-05-008 — Module fault isolation
**Phase:** P5 · **Priority:** MUST
A failing or crashed module container shall not affect the core or other modules; the gateway shall contain and log the failure.
**Accept:** A module container that crashes or hangs yields a `502/503` for its routes only; the core and other module containers keep serving; the gateway recovers when the module is restarted.

### TR-05-009 — Self-signing key bootstrap
**Phase:** P5 · **Priority:** MUST
The backend shall generate a self-signing keypair on first startup, persist it securely, and use it to self-sign internally-built modules; the trust store shall hold this self key plus any configured external signer keys.
**Accept:** On first startup a keypair is generated and persisted; an internally-built module self-signed with it passes verification (TR-05-002); the corresponding public key is present in the trust store, and external signer keys can be added.

### TR-06-001 — SSE event stream endpoint
**Phase:** P6 · **Priority:** MUST
The backend shall expose `GET /api/v1/events/stream` over HTTP/2 Server-Sent Events, authenticated, streaming events to subscribed clients.
**Accept:** An authenticated client subscribes and receives events on a long-lived connection; an unauthenticated request returns `401`.

### TR-06-002 — Event envelope format
**Phase:** P6 · **Priority:** MUST
Events shall serialize as `{ type, data, timestamp, user_id }`, where `user_id` targets a single user or is null for broadcast.
**Accept:** Emitted events match the schema; a targeted event reaches only that user's stream; a broadcast reaches all subscribers.

### TR-06-003 — Event publishing API
**Phase:** P6 · **Priority:** MUST
Services shall publish domain events (e.g. `user.created`, `module.loaded`, `config.updated`) that are routed to the appropriate SSE subscribers.
**Accept:** Publishing a known event type delivers it to the correct subscribers; tests cover both targeted and broadcast delivery.

### TR-06-004 — Kafka topic conventions & producer
**Phase:** P6 · **Priority:** MUST
Asynchronous messages shall be produced to Kafka topics named `superapp.{service}.{action}` as JSON with a metadata envelope.
**Accept:** Producing emits to the correctly-named topic with the envelope; the message schema is validated in tests.

### TR-06-005 — Kafka consumers & consumer groups
**Phase:** P6 · **Priority:** MUST
Consumers shall subscribe using one consumer group per service for load balancing.
**Accept:** A consumer processes messages from its topic; two instances in the same group share partitions without double-processing.

### TR-06-006 — Dead-letter queue
**Phase:** P6 · **Priority:** MUST
Messages that fail processing shall be retried and then routed to a dead-letter queue.
**Accept:** A message that keeps failing is retried then routed to the DLQ; the DLQ entry preserves the original message plus failure metadata.

### TR-06-007 — Module access to messaging
**Phase:** P6 · **Priority:** SHOULD
Loaded modules shall be able to publish and consume via the core-provided messaging layer.
**Accept:** A loaded module publishes an event and consumes from a topic through the core-provided API.

### TR-06-008 — SSE reconnect & resume
**Phase:** P6 · **Priority:** SHOULD
SSE clients shall be able to reconnect and resume via `Last-Event-ID` without missing events within a bounded window.
**Accept:** After a dropped connection, a client reconnecting with `Last-Event-ID` receives the missed events buffered within the window.

### TR-07-001 — Frontend application scaffold
**Phase:** P7 · **Priority:** MUST
`frontend/core` shall be scaffolded with React + TypeScript + Vite + ShadCN UI + Tailwind (mobile-first), building and rendering a baseline screen.
**Accept:** `build` succeeds; the dev server renders a baseline screen using a ShadCN component; the layout is responsive across breakpoints.

### TR-07-002 — Typed API client & response envelope handling
**Phase:** P7 · **Priority:** MUST
The frontend shall provide a typed API client that consumes the standard response envelope and surfaces errors.
**Accept:** The client parses `{success,data,message,errors,pagination}`, exposes typed data, and surfaces error payloads to callers.

### TR-07-003 — OIDC auth/session integration (Rauthy)
**Phase:** P7 · **Priority:** MUST
The frontend shall integrate with Rauthy via OIDC Authorization Code + PKCE, storing tokens, injecting the `Authorization` header, and refreshing on expiry.
**Accept:** Login redirects to Rauthy and returns authenticated; the access token is attached to API calls; expiry triggers refresh or re-login.

### TR-07-004 — Role-based route protection
**Phase:** P7 · **Priority:** MUST
The frontend shall enforce role-based route protection (React Router v6); the UI reflects permissions while authorization remains server-side (Cedar).
**Accept:** Unauthenticated → redirect to login; a user hitting an admin route is blocked; an admin is allowed.

### TR-07-005 — Frontend module host
**Phase:** P7 · **Priority:** MUST
The frontend shall provide a module host that dynamically loads frontend modules (routes, components, permissions, `initialize`/`cleanup`).
**Accept:** A sample module's routes/components load at runtime; `initialize`/`cleanup` are invoked; routes are hidden when the user lacks the permission.

### TR-07-006 — SSE real-time client
**Phase:** P7 · **Priority:** MUST
The frontend shall provide an SSE client for `/api/v1/events/stream` with reconnect/resume.
**Accept:** The client receives events and updates the UI; it reconnects after a drop and resumes via `Last-Event-ID`.

### TR-07-007 — Environment configuration (`VITE_*`)
**Phase:** P7 · **Priority:** SHOULD
The frontend shall be configured via `VITE_*` environment variables (API base URL, OIDC settings).
**Accept:** The build reads `VITE_*` vars; a missing required variable fails with a clear error.

### TR-08-001 — Mobile application scaffold
**Phase:** P8 · **Priority:** MUST
`mobile/core` shall be scaffolded with React Native + Expo + TypeScript + Tamagui, building and rendering a baseline screen on iOS and Android.
**Accept:** The app builds and runs on an iOS simulator and an Android emulator; it renders a baseline screen using Tamagui.

### TR-08-002 — Typed API client & response envelope handling
**Phase:** P8 · **Priority:** MUST
The mobile app shall provide a typed API client that consumes the standard response envelope and surfaces errors.
**Accept:** The client parses `{success,data,message,errors,pagination}`, exposes typed data, and surfaces error payloads.

### TR-08-003 — OIDC auth via Rauthy (native)
**Phase:** P8 · **Priority:** MUST
The mobile app shall authenticate with Rauthy via OIDC Authorization Code + PKCE using the system browser with a deep-link redirect, storing tokens in platform secure storage (Keychain/Keystore) and refreshing on expiry.
**Accept:** Login opens the system browser and returns to the app authenticated via deep link; tokens persist in secure storage; expiry triggers refresh or re-login.

### TR-08-004 — Role-based navigation guards
**Phase:** P8 · **Priority:** MUST
The mobile app shall enforce role-based navigation guards (React Navigation); the UI reflects permissions while authorization remains server-side (Cedar).
**Accept:** Unauthenticated → auth stack; a user is blocked from admin screens; an admin is allowed.

### TR-08-005 — Mobile module host
**Phase:** P8 · **Priority:** MUST
The mobile app shall provide a module host that dynamically loads mobile modules (screens, components, permissions, `initialize`/`cleanup`).
**Accept:** A sample module's screens/components load at runtime; `initialize`/`cleanup` are invoked; screens are hidden when the user lacks the permission.

### TR-08-006 — Real-time client (foreground SSE)
**Phase:** P8 · **Priority:** MUST
The mobile app shall consume `/api/v1/events/stream` via SSE for live updates while foregrounded, with reconnect/resume on drop and on app-foreground.
**Accept:** While foregrounded the app receives events and updates the UI; it reconnects after a drop and resumes via `Last-Event-ID`.

### TR-08-007 — Environment configuration (`EXPO_PUBLIC_*`)
**Phase:** P8 · **Priority:** SHOULD
The mobile app shall be configured via `EXPO_PUBLIC_*` environment variables (API base URL, OIDC settings).
**Accept:** The build reads `EXPO_PUBLIC_*`; a missing required variable fails with a clear error.

### TR-08-008 — Background delivery via push notifications
**Phase:** P8 · **Priority:** SHOULD
The system shall deliver events to backgrounded/closed apps via push notifications (APNs/FCM, e.g. Expo notifications), since SSE cannot run reliably in the background; this requires a backend push-send capability.
**Accept:** With the app backgrounded, a server-side event produces a push notification delivered to the device; tapping it deep-links to the relevant screen.

### TR-09-001 — Backend module SDK & service contract
**Phase:** P9 · **Priority:** MUST
The SDK shall define the backend module **service contract** (the HTTP/gRPC interface a module container exposes: lifecycle/readiness, declared routes, permissions, config, health) and provide a Rust SDK to implement it; modules may be written in any language that fulfills the contract.
**Accept:** A backend module built with the SDK exposes the contract, runs as a container, and is registered and proxied by the core gateway; the contract is documented (e.g. OpenAPI/proto).

### TR-09-002 — Frontend module SDK package
**Phase:** P9 · **Priority:** MUST
A TypeScript SDK package shall expose the frontend module interface (routes, components, permissions, `initialize`/`cleanup`) and types.
**Accept:** A frontend module imports the SDK types, builds, and loads in the web module host (TR-07-005).

### TR-09-003 — Mobile module SDK package
**Phase:** P9 · **Priority:** MUST
A TypeScript SDK package shall expose the mobile module interface (screens, components, permissions, `initialize`/`cleanup`) and types.
**Accept:** A mobile module imports the SDK types, builds, and loads in the mobile module host (TR-08-005).

### TR-09-004 — Canonical module manifest schema
**Phase:** P9 · **Priority:** MUST
A single shared manifest schema (`name`, `version`, `endpoints`, `permissions`, `config_schema`, `signatures[]`) shall be used by all platforms and `/modules/register`.
**Accept:** A manifest validates against the one shared schema; backend registration (TR-05-003) and the SDK reference the same schema; an invalid manifest is rejected consistently.

### TR-09-005 — SDK versioning & compatibility contract
**Phase:** P9 · **Priority:** MUST
The SDK shall expose a version and define compatibility rules so the core can reject incompatible modules.
**Accept:** A module built against an incompatible SDK version is rejected at load/registration with a clear error; compatible versions load.

### TR-09-006 — Module scaffolding & signed packaging
**Phase:** P9 · **Priority:** SHOULD
The SDK shall provide a generator that scaffolds a cross-platform module skeleton (backend+frontend+mobile) wired to the SDK, plus a packaging step that produces a **signed** artifact (signature array per TR-05-002, excluding the data part).
**Accept:** Running the generator produces a buildable skeleton in all three trees matching `project.md`; the packaging step emits an artifact carrying a valid signature the loader accepts.

### TR-09-007 — Reference module (end-to-end, cross-platform)
**Phase:** P9 · **Priority:** MUST
A reference module spanning backend + frontend + mobile shall demonstrate registration, a route + web screen + mobile screen, a Cedar-governed permission, schema-validated config, lifecycle, and health.
**Accept:** It registers and loads in the backend (P5), renders in the web host (P7) and mobile host (P8); a Cedar policy gates its action (`403` without / `200` with); its health reports status.

### TR-09-008 — Module-author documentation
**Phase:** P9 · **Priority:** SHOULD
The SDK shall ship documentation covering the contracts, manifest, lifecycle, permissions, config, signing, and distribution.
**Accept:** Docs exist and the reference module can be built and registered by following them.

### TR-09-009 — Module distribution via private OCI registry
**Phase:** P9 · **Priority:** MUST
Modules shall be packaged as **OCI/Docker images** and pushed to a **self-hosted private artifact repository**; the core shall resolve modules from that registry by name+version (no public package registries).
**Accept:** Building a module produces an OCI image; pushing it to the private registry succeeds; the core resolves/pulls a module image from the private registry by name+version.

### TR-10-001 — CI pipeline (GitHub Actions)
**Phase:** P10 · **Priority:** MUST
A GitHub Actions pipeline shall lint, test, and build on every push/PR and fail on red.
**Accept:** The workflow runs `cargo fmt --check` + `cargo clippy` + `cargo test` (backend) and lint + test (frontend/mobile) on each push/PR; a failing job fails the check.

### TR-10-002 — Coverage gates
**Phase:** P10 · **Priority:** MUST
CI shall enforce a minimum **80% coverage on critical paths** (the gate TR-00-001 defers to P10).
**Accept:** Coverage is measured (`cargo llvm-cov` / frontend coverage); the GitHub Actions job fails when critical-path coverage is below 80%.

### TR-10-003 — Multi-stage backend image
**Phase:** P10 · **Priority:** MUST
The Rust/loco backend shall ship a multi-stage Dockerfile producing a minimal runtime image.
**Accept:** `docker build` yields a runnable image that starts the loco server; the final layer contains no build toolchain.

### TR-10-004 — Multi-stage frontend image
**Phase:** P10 · **Priority:** MUST
The React frontend shall ship a multi-stage Dockerfile that builds and serves the app.
**Accept:** `docker build` yields an image that serves the built frontend.

### TR-10-005 — Docker Compose orchestration
**Phase:** P10 · **Priority:** MUST
A Docker Compose deployment shall orchestrate the SuperApp services (backend gateway, frontend) and registered module containers, attaching to the helvetia-compose network for infrastructure (TR-02-003) without duplicating infra services.
**Accept:** `compose up` starts backend + frontend and any registered module containers attached to the helvetia-compose network; healthchecks pass; the gateway routes to module containers; no PostgreSQL/Redis/Kafka/Prometheus/Grafana services are redefined here.

### TR-10-006 — Image publication to private registry
**Phase:** P10 · **Priority:** MUST
CI shall publish versioned app (and module) images to the self-hosted private registry (TR-09-009).
**Accept:** GitHub Actions pushes versioned images to the private registry; a pull yields a runnable image.

### TR-10-007 — Environment-specific deployment config
**Phase:** P10 · **Priority:** SHOULD
Deployment shall provide dev/prod configurations selected via env, with no secrets committed.
**Accept:** Dev and prod select the correct settings via env; no secrets are present in the repo.

### TR-10-008 — Conventional Commits enforcement
**Phase:** P10 · **Priority:** SHOULD
Conventional Commits shall be enforced via a commit hook and/or CI check (realizes TR-01-005).
**Accept:** A non-conforming commit message is rejected by the hook or the CI check.

### TR-10-009 — Mobile build/release pipeline
**Phase:** P10 · **Priority:** SHOULD
A mobile build/release pipeline (Expo/EAS) shall produce distributable iOS and Android artifacts.
**Accept:** A build job produces iOS and Android distributable artifacts.

### TR-00-001 — Test-driven development (TDD)
**Phase:** all · **Priority:** MUST
Development shall follow TDD across backend (Rust), frontend, and mobile: for each requirement, encode its **Accept** criteria as automated tests first (red), implement until they pass (green), then refactor. New behavior is not merged without an accompanying test written as part of the same change.
**Accept:** Every merged feature ships with tests that map to its requirement's Accept criteria; CI fails on red; coverage gates (defined in Phase 10) are enforced.

---

## Phase ↔ Requirement Mapping

| Phase | Requirements |
|---|---|
| P1 — Project definition & standards | TR-01-001, TR-01-002, TR-01-003, TR-01-004, TR-01-005, TR-01-006 |
| _Cross-cutting (all phases)_ | TR-00-001 (TDD) |
| P2 — Infrastructure & local env | TR-02-001, TR-02-002, TR-02-003, TR-02-004, TR-02-005, TR-02-006 |
| P3 — Backend core bootstrap | TR-03-001, TR-03-002, TR-03-003, TR-03-004, TR-03-005, TR-03-006, TR-03-007 |
| P4 — Auth, SSO & policy-based AuthZ | TR-04-001, TR-04-002, TR-04-003, TR-04-004, TR-04-005, TR-04-006, TR-04-007, TR-04-008, TR-04-009, TR-04-010, TR-04-011, TR-04-012, TR-04-013 |
| P5 — Dynamic module loading | TR-05-001, TR-05-002, TR-05-003, TR-05-004, TR-05-005, TR-05-006, TR-05-007, TR-05-008, TR-05-009 |
| P6 — Real-time & messaging | TR-06-001, TR-06-002, TR-06-003, TR-06-004, TR-06-005, TR-06-006, TR-06-007, TR-06-008 |
| P7 — Frontend core | FR-07-001, FR-07-002, FR-07-003, FR-07-004, TR-07-001, TR-07-002, TR-07-003, TR-07-004, TR-07-005, TR-07-006, TR-07-007 |
| P8 — Mobile core | FR-08-001, FR-08-002, FR-08-003, FR-08-004, TR-08-001, TR-08-002, TR-08-003, TR-08-004, TR-08-005, TR-08-006, TR-08-007, TR-08-008 |
| P9 — Module SDK & reference module | TR-09-001, TR-09-002, TR-09-003, TR-09-004, TR-09-005, TR-09-006, TR-09-007, TR-09-008, TR-09-009 |
| P10 — Testing, Docker & deployment | TR-10-001, TR-10-002, TR-10-003, TR-10-004, TR-10-005, TR-10-006, TR-10-007, TR-10-008, TR-10-009 |
