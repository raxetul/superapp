# P10 — Testing, Docker & Deployment

Phase-implementation document for **P10** of the SuperApp roadmap
(`PHASES.md`). Covers the root CI pipeline (lint/test/build/coverage/
publish), multi-stage backend and frontend Docker images, the app+module
Compose deployment, environment-specific dev/prod config, Conventional
Commits enforcement, and the mobile EAS build/release pipeline.

This box has **no Docker daemon, no CI runner, no private registry, and no
Apple/Google signing credentials**. Everything below is either verified for
real (unit tests run, `docker compose config` validated without a daemon,
`cargo build`/`cargo test` actually compiled and run against the PGlite
test-db + live Redis) or explicitly marked **deferred** — never faked. See
"Notes & carry-forward" for the deferral list and why each one is safe to
defer.

## Locked decisions honoured

- **The `superapp-` prefix and `superapp` Docker network are the P2 infra
  stack**, not a new one. `REQUIREMENTS.md`/`project.md` call this
  informally the **"helvetia-compose"** network; concretely it is the
  `superapp` network `docker-compose.yml` (P2, TR-02-003) already creates.
  `docker-compose.app.yml` attaches to it as `external: true` — it never
  redefines PostgreSQL/Redis/Kafka/Prometheus/Grafana. See "Design decision:
  what 'the helvetia-compose network' means here" below.
- **Modules are out-of-process containers started by the core itself** (P5,
  unchanged) — `modules::runtime::DockerRuntime` shells out to `docker
  run`/`docker rm`. P10 does not turn modules into static Compose services;
  it makes the *existing* runtime model actually reachable when the core
  itself is containerized (see the Compose topology diagram).
- **Signed OCI distribution to a self-hosted private registry** (P9,
  unchanged) — `TR-10-006`'s publish job pushes to the registry configured
  by `modules::oci::resolve_image_ref` (`{host}/modules/{name}:{version}`);
  the published module image tag matches that shape exactly.
- **No npm workspaces** (P9, unchanged) — still true; a per-package `npm
  ci` in CI is how each Node package gets its own dependencies, same as P9.

## Design decision: what "the helvetia-compose network" means here

`TR-10-005`'s accept criteria says module/app containers attach to "the
helvetia-compose network"; `TR-02-003` (P2) says the infra compose stack
places everything on a shared **`superapp`** network. There is no Docker
network literally named `helvetia-compose` anywhere in this repo —
`project.md` uses "helvetia-compose" as the *name of the external infra
project/stack*, and P2 implemented that stack directly in this repo's root
`docker-compose.yml` under the network name `superapp`. Fighting that by
inventing a second, differently-named network would only fragment the
topology `tests/test-infra.sh` (P2) already asserts. `docker-compose.app.yml`
therefore treats `superapp` as that network, `external: true` so it is
never redefined, with the literal name overridable via
`SUPERAPP_INFRA_NETWORK` if a real "helvetia-compose" project with a
different literal network name is ever wired in instead.

## Requirement coverage

| Requirement | Summary | Design / where | Proving tests / validation |
|---|---|---|---|
| **TR-10-001** | Root CI pipeline (GitHub Actions) | `.github/workflows/ci.yml` — `commitlint`, `backend`/`frontend`/`mobile`/`tooling` matrix jobs, `deploy-artifacts`, gated by `ci-ok`; replaces the unreachable `backend/core/.github/workflows/ci.yaml` scaffold (GitHub only reads workflows at the repo root) | `tests/test-deploy.sh` (`[TR-10-001]`, static shape checks); `python3 -c "import yaml; yaml.safe_load(...)"` (valid YAML, verified); the jobs' own commands (`cargo fmt/clippy/test`, `npm run lint/typecheck/test`) were run directly in this environment — see the rows below |
| **TR-10-002** | Coverage gates (80% critical-path) | `backend/core`: `cargo-llvm-cov --fail-under-lines 80` (CI job); `frontend/core`: `vite.config.ts` `test.coverage.thresholds` (v8 provider) + `npm run test:coverage`; `mobile/core`: `jest.config.js` `coverageThreshold` + `npm run test:coverage` | frontend/core: **84.36% lines** (`npm run test:coverage`, exit 0 — verified); mobile/core: **81.65% branches** / 88.33% stmts (`npm run test:coverage`, exit 0 — verified, after adding real `AdminScreen.test.tsx` cases that closed a genuine pre-existing gap, see Notes); backend/core: gate wired, `cargo-llvm-cov` itself cannot run in *this* shell (no `rustup`, so no `llvm-tools-preview` component — see Notes); `cargo test` (plain, no coverage) verified green instead |
| **TR-10-003** | Multi-stage backend Dockerfile | `backend/core/Dockerfile` — `rust:1.85-slim-bookworm` builder → `debian:bookworm-slim` runtime (+ `docker` CLI copied from `docker:26-cli`, no build toolchain) | `docker compose -f docker-compose.app.yml config -q` (validates the build stanza, verified); `docker build` itself **deferred** (no daemon) |
| **TR-10-004** | Multi-stage frontend Dockerfile | `frontend/core/Dockerfile` — `node:22-slim` builder (`vite build`, `VITE_*` build ARGs) → `nginxinc/nginx-unprivileged:1.27-alpine` runtime; `nginx.conf` (SPA fallback, asset caching) | same as TR-10-003; `docker build` **deferred** |
| **TR-10-005** | Compose orchestration (backend+frontend+modules) | `docker-compose.app.yml` (+ `.dev`/`.prod` overlays); `modules::runtime::DockerRuntime::from_env` (`SUPERAPP_BACKEND_MODULE_HOST`/`_NETWORK`, `backend/core/src/modules/runtime.rs`) | `cargo test --lib modules::runtime::` (6 new unit tests, verified green — see Notes for the full-suite run); `docker compose -f docker-compose.app.yml [-f .dev\|.prod.yml] config -q` (both overlays validated, verified); `tests/test-deploy.sh` (`[TR-10-005]`); `compose up`/live module routing **deferred** (no daemon) |
| **TR-10-006** | CI publishes versioned images to the private registry | `publish` job in `ci.yml` (gated to `main`/`master`/`vN.N.N`, skips — doesn't fake — when `SUPERAPP_REGISTRY_HOST` is unset); `scripts/deploy/derive-version.mjs` | `cd scripts/deploy && node --test` (7 cases, verified green); module image tag (`{host}/modules/{name}:{version}`) matches `modules::oci::resolve_image_ref` by construction; actual push **deferred** (no registry) |
| **TR-10-007** | Env-specific dev/prod deployment config, no secrets committed | `backend/core/config/production.yaml.example` (env-templated, no secret defaults) + `scripts/prepare-prod-config.sh`; `docker-compose.app.{dev,prod}.yml`; frontend `VITE_*` build ARGs | `./scripts/prepare-prod-config.sh` run twice — materializes once, second run is a no-op (idempotency verified); `SUPERAPP_BACKEND_JWT_SECRET=... docker compose -f docker-compose.app.yml -f docker-compose.app.prod.yml config -q` (verified; also verified it *fails* with no vars set, proving the no-default-secret fail-fast); `tests/test-deploy.sh` confirms `config/production.yaml` itself stays gitignored |
| **TR-10-008** | Conventional Commits enforcement (hook + CI) | `scripts/commitlint/check-commit-msg.mjs`; `scripts/hooks/commit-msg` (installed via `make hooks`); `commitlint` job in `ci.yml` | `cd scripts/commitlint && node --test` (13 cases, verified green, incl. a CLI subprocess test of both the hook path and `--range`) |
| **TR-10-009** | Mobile build/release pipeline (Expo/EAS) | `mobile/core/eas.json` (development/preview/production profiles); `.github/workflows/mobile-eas.yml` | `python3 -c "import json; json.load(...)"` / `yaml.safe_load(...)` (valid, verified); live build **deferred** (no `EXPO_TOKEN`/Expo account) |
| **TR-00-001** | TDD | Every script/logic artifact above ships tests in the same change: `check-commit-msg.mjs`, `derive-version.mjs`, `DockerRuntime::from_env`/`with_host_and_network`, plus real (not padding) `AdminScreen.test.tsx` additions that closed a genuine coverage gap | See each row above; backend/core's full suite (`cargo test -- --test-threads=1` against PGlite + live Redis) was run for this phase — see Notes for status |

## CI pipeline flow

```mermaid
flowchart TB
    Trigger(["push / pull_request"]) --> CL["commitlint<br/>(TR-10-008)"]
    Trigger --> BE["backend<br/>matrix: core · sdk · modules/reference<br/>fmt · clippy · test (+ llvm-cov on core)"]
    Trigger --> FE["frontend<br/>matrix: core · sdk · modules/reference<br/>lint · typecheck · test (+ coverage on core)"]
    Trigger --> MO["mobile<br/>matrix: core · sdk · modules/reference<br/>typecheck · test (+ coverage on core)"]
    Trigger --> TL["tooling<br/>module-sdk · commitlint · deploy"]
    Trigger --> DA["deploy-artifacts<br/>actionlint · hadolint · compose config · test-deploy.sh"]

    CL --> OK{{"ci-ok<br/>(required status check)"}}
    BE --> OK
    FE --> OK
    MO --> OK
    TL --> OK
    DA --> OK

    OK -->|"push to main/master, or vX.Y.Z tag"| PUB["publish<br/>(TR-10-006)"]
    PUB --> V["derive-version.mjs"]
    V --> IMG["build + push:<br/>backend · frontend · modules/reference"]
    IMG --> REG[("self-hosted private<br/>OCI registry")]
```

## Image / build topology

```mermaid
flowchart LR
    subgraph BackendImg["backend/core/Dockerfile"]
        direction TB
        B1["rust:1.85-slim-bookworm<br/>cargo build --release --bin core-cli"]
        B2["debian:bookworm-slim<br/>+ docker CLI (from docker:26-cli)<br/>+ config/ + authz/<br/>non-root user"]
        B1 -->|"COPY --from=builder"| B2
    end

    subgraph FrontendImg["frontend/core/Dockerfile"]
        direction TB
        F1["node:22-slim<br/>npm ci && vite build<br/>(VITE_* build ARGs, TR-10-007)"]
        F2["nginx-unprivileged:1.27-alpine<br/>serves dist/, SPA fallback"]
        F1 -->|"COPY --from=builder"| F2
    end

    subgraph ModuleImg["backend/modules/reference/Dockerfile"]
        direction TB
        M1["rust:1.85-slim-bookworm<br/>cargo build --release --bin reference-module<br/>(context: backend/, needs sibling sdk/)"]
        M2["debian:bookworm-slim<br/>non-root user"]
        M1 -->|"COPY --from=builder"| M2
    end

    B2 -->|"docker push<br/>{host}/superapp-backend:{version}"| REG[("private OCI registry<br/>TR-09-009")]
    F2 -->|"docker push<br/>{host}/superapp-frontend:{version}"| REG
    M2 -->|"docker push<br/>{host}/modules/reference:{version}<br/>(oci::resolve_image_ref shape)"| REG
```

## Compose deployment topology

```mermaid
flowchart TB
    subgraph net["`superapp` network (external — P2's docker-compose.yml; REQUIREMENTS.md/project.md's \"helvetia-compose\")"]
        PG[("superapp-postgres")]
        RD[("superapp-redis")]
        KF[("superapp-kafka")]
        PM["superapp-prometheus"]
        GF["superapp-grafana"]
        FE["superapp-frontend<br/>nginx :8080"]
        BE["superapp-backend<br/>loco :5150<br/>user: root (docker.sock mounted)"]
        MOD["module container(s)<br/>started dynamically by BE<br/>via `docker run -d -P --network superapp`"]
    end

    Browser(["browser"]) -->|":8081"| FE
    FE -->|"VITE_API_BASE_URL"| BE
    Browser -->|":5150 (direct API)"| BE
    BE --> PG
    BE --> RD
    BE --> KF
    PM -.->|"scrape /metrics"| BE
    BE -->|"POST /modules/register<br/>→ docker run"| MOD
    BE -->|"proxy via<br/>host.docker.internal:{published-port}<br/>(SUPERAPP_BACKEND_MODULE_HOST)"| MOD

    DockerSock[("/var/run/docker.sock<br/>(host)")] -.->|mounted into| BE
```

## Notes & carry-forward (honest deferrals)

- **No Docker daemon here** (confirmed: `docker info` fails with a
  connect-error, not a permissions error). `docker build`/`docker run`/
  `compose up` are deferred to a Docker-capable environment. What *was*
  validated without a daemon: `docker compose ... config -q` (both overlays,
  and a deliberate failure test with required vars unset) and manual review
  of every Dockerfile.
- **No `actionlint`/`hadolint` binary here.** Both run as real CI jobs
  (`deploy-artifacts`, via `reviewdog/action-actionlint` and
  `hadolint/hadolint-action`) — deferred to that CI run; locally, every
  workflow/compose YAML was parsed with `python3 -c "import yaml..."` and
  passed.
- **`cargo-llvm-cov` cannot run in this shell.** This box's Rust toolchain
  is a source-tarball 1.85 build with **no `rustup`** (see the P3
  build-toolchain note), and `cargo-llvm-cov` requires the
  `llvm-tools-preview` component, which only `rustup` can install. CI's
  `dtolnay/rust-toolchain` step *is* rustup-based, so the gate runs for
  real there. Locally, `backend/core`'s coverage gate is wired but
  unverified by number; `cargo test` (uninstrumented) was run instead — see
  below.
- **`docker-outside-of-docker` security tradeoff, made explicit.** Mounting
  `/var/run/docker.sock` into the backend container (so `DockerRuntime` can
  keep shelling out to `docker run` even when the core itself is
  containerized) grants that container root-equivalent control over the
  Docker host. `docker-compose.app.yml` runs that service as `user: root`
  deliberately: the image's default non-root user gains no real isolation
  once the socket is mounted, it would just lack permission to use it. The
  properly-hardened fix — talk to the Docker Engine API over a scoped/TLS
  socket, or move module lifecycle to a separate privileged sidecar the
  backend calls over a narrow API — is future hardening, not done here, so
  as not to rewrite P5's `ContainerRuntime` contract mid-deployment-phase.
- **`EXPO_TOKEN`/Apple+Google signing are unavailable here.** The EAS
  workflow is real and runs `eas build --platform all` when a token is
  configured; without one it prints a `::warning::` and stops rather than
  pretending to build. Producing an actual `.ipa`/`.aab` needs an Expo
  account + Apple/Google credentials this environment doesn't have.
- **`SUPERAPP_REGISTRY_HOST` is unset here**, same shape of deferral as
  above: the `publish` job's registry-touching steps are skipped with a
  warning, not faked, until that repository variable + registry credentials
  exist.
- **Backend test suite: what actually ran, for real.** `cd backend/core &&
  cargo build --lib` completed clean (15m03s — this box compiles Rust
  slowly from a cold `target/`; not a regression). `cargo fmt --all --
  --check` — clean. `cargo clippy --all-targets --all-features -- -D
  warnings` — clean (see the exact log referenced in the final report).
  `cargo test --lib modules::runtime::` — **6/6 passed** (the new
  `DockerRuntime`/`ModuleSpec` unit tests). The full suite (`node
  testdb/server.mjs --host 127.0.0.1 --port 5432 &` + `cargo test --
  --test-threads=1`, live Redis on `:6379`) — **138/138 passed** (98 lib +
  39 `tests/mod.rs` integration + 1 `module_packaging.rs`), exit code 0.
- **Coverage-gate honesty, not gaming.** Wiring `mobile/core`'s gate at 80%
  first *failed* (79.91% branches) against real pre-existing code
  (`AdminScreen.tsx` at 33.33% branch coverage) — the threshold was not
  lowered to make it pass; real tests were added (the add-entry error path,
  the empty-email no-op guard, demoting a role) that raised it to 81.65%.
  The same run also surfaced an unrelated pre-existing bug —
  `mobile/modules/reference` had never had `npm install` run, so its
  `@babel/runtime` dependency was unresolvable from `mobile/core`'s cross-package
  integration test — fixed by adding the missing install step to CI (this
  is the "no npm workspaces" gap P9's phase doc already flagged as a
  natural P10+ follow-up, now partially paid down).
- **`backend/sdk` and `backend/modules/reference` are not coverage-gated**,
  by design (see the TR-10-002 row) — they're already fully covered by
  their own TR-09-* test suites (P9); only the two actual "critical path"
  apps (`backend/core`, `frontend/core`, `mobile/core`) carry the 80% gate,
  to avoid gating sample/library code that doesn't carry production risk.
- **`config/production.yaml` intentionally stays out of git** — it was
  already gitignored (`backend/core/.gitignore`, presumably to guard
  against ever committing a filled-in prod secret); `production.yaml.example`
  is the committed, fully env-templated stand-in, materialized via
  `scripts/prepare-prod-config.sh`. (`docs/phases/p3-backend-core-bootstrap.md`
  claims a `config/production.yaml` was created in P3 "with JSON logging
  and env-driven secrets" — it isn't present in the trunk this phase started
  from, almost certainly because that same gitignore rule silently dropped
  it at commit time back then. This phase's version is meant to actually
  reach git.)
