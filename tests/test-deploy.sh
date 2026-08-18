#!/usr/bin/env bash
# P10 acceptance tests — CI/Docker/deploy artifacts (TR-10-*).
# Static contract checks: this box has no Docker daemon, no CI runner, no
# registry, no Apple/Google signing — anything needing those is a validated
# *shape* check here (docker compose config; grep for required steps), not a
# live run. See docs/phases/p10-testing-docker-deploy.md for the deferrals.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
fail=0
pass(){ echo "  ok  : $1"; }
err(){ echo "  FAIL: $1"; fail=1; }

echo "[TR-10-001] root CI pipeline covers backend/frontend/mobile/tooling"
CI=".github/workflows/ci.yml"
[ -f "$CI" ] || err "$CI missing"
for j in "commitlint:" "backend:" "frontend:" "mobile:" "tooling:"; do
  grep -q "^  $j" "$CI" 2>/dev/null && pass "job $j present" || err "job $j missing from $CI"
done
grep -q "cargo fmt" "$CI" && pass "cargo fmt wired" || err "cargo fmt not wired"
grep -q "cargo clippy" "$CI" && pass "cargo clippy wired" || err "cargo clippy not wired"
grep -q "cargo test" "$CI" && pass "cargo test wired" || err "cargo test not wired"
[ -f "backend/core/.github/workflows/ci.yaml" ] && err "stale unreachable backend/core/.github workflow still present (should be consolidated into $CI)" || pass "no stale nested workflow"

echo "[TR-10-002] coverage gates wired at 80%"
grep -q "llvm-cov" "$CI" 2>/dev/null && pass "backend coverage (cargo-llvm-cov) wired" || err "backend coverage not wired"
grep -qE "fail-under-lines[= ]80|fail_under_lines[: ]80" "$CI" 2>/dev/null && pass "backend 80% threshold wired" || err "backend 80% threshold not found"
grep -q "test:coverage" "$CI" 2>/dev/null && pass "frontend/mobile coverage script wired" || err "frontend/mobile coverage script not wired"

echo "[TR-10-003] backend multi-stage Dockerfile"
BD="backend/core/Dockerfile"
[ -f "$BD" ] || err "$BD missing"
[ "$(grep -c '^FROM ' "$BD" 2>/dev/null)" -ge 2 ] && pass "backend Dockerfile is multi-stage" || err "backend Dockerfile is not multi-stage"
grep -qE "^FROM .*(debian|distroless|alpine)" "$BD" 2>/dev/null && pass "minimal runtime base" || err "runtime base not minimal"

echo "[TR-10-004] frontend multi-stage Dockerfile"
FD="frontend/core/Dockerfile"
[ -f "$FD" ] || err "$FD missing"
[ "$(grep -c '^FROM ' "$FD" 2>/dev/null)" -ge 2 ] && pass "frontend Dockerfile is multi-stage" || err "frontend Dockerfile is not multi-stage"
grep -qi "nginx" "$FD" 2>/dev/null && pass "frontend served by nginx" || err "no static server stage found"

echo "[TR-10-005] compose deployment: infra network, no infra redefinition, healthchecks"
APP="docker-compose.app.yml"
[ -f "$APP" ] || err "$APP missing"
grep -qE "external: *true" "$APP" 2>/dev/null && pass "infra network is external (not redefined)" || err "infra network not marked external"
for svc in postgres redis kafka prometheus grafana; do
  grep -qE "^\s*${svc}:\s*$" "$APP" 2>/dev/null && err "infra service '$svc' redefined in $APP" || pass "no '$svc' redefinition in $APP"
done
grep -q "healthcheck:" "$APP" 2>/dev/null && pass "healthchecks present" || err "no healthchecks in $APP"
if command -v docker >/dev/null 2>&1; then
  SUPERAPP_BACKEND_JWT_SECRET=test docker compose -f "$APP" -f docker-compose.app.dev.yml config -q 2>/dev/null \
    && pass "compose (dev overlay) validates" || err "compose (dev overlay) invalid"
  SUPERAPP_BACKEND_JWT_SECRET=test VITE_API_BASE_URL=https://x.invalid \
    docker compose -f "$APP" -f docker-compose.app.prod.yml config -q 2>/dev/null \
    && pass "compose (prod overlay) validates" || err "compose (prod overlay) invalid"
fi

echo "[TR-10-006] CI publishes versioned images to the private registry"
grep -qi "registry" "$CI" 2>/dev/null && pass "registry publish step present" || err "no registry publish step in $CI"
grep -qE "docker/login-action|docker login" "$CI" 2>/dev/null && pass "registry login step present" || err "no registry login step"

echo "[TR-10-007] env-specific dev/prod config, no secrets committed"
[ -f "docker-compose.app.dev.yml" ] || err "docker-compose.app.dev.yml missing"
[ -f "docker-compose.app.prod.yml" ] || err "docker-compose.app.prod.yml missing"
[ -f "backend/core/config/production.yaml.example" ] || err "backend/core/config/production.yaml.example missing"
[ -f "backend/core/config/production.yaml" ] && err "backend/core/config/production.yaml is committed (should be gitignored, template-only)"
grep -qxF '**/config/production.yaml' backend/core/.gitignore 2>/dev/null && pass "production.yaml stays gitignored" || err "production.yaml ignore rule missing"
[ -x "scripts/prepare-prod-config.sh" ] && pass "prepare-prod-config.sh present+executable" || err "prepare-prod-config.sh missing/not executable"

echo "[TR-10-008] Conventional Commits enforced via hook + CI"
[ -x "scripts/hooks/commit-msg" ] && pass "commit-msg hook present" || err "commit-msg hook missing"
grep -q "^hooks:" Makefile 2>/dev/null && pass "make hooks target present" || err "make hooks target missing"
grep -q "^  commitlint:" "$CI" 2>/dev/null && pass "commitlint CI job present" || err "commitlint CI job missing"

echo "[TR-10-009] mobile build/release pipeline (Expo/EAS)"
[ -f "mobile/core/eas.json" ] || err "mobile/core/eas.json missing"
[ -f ".github/workflows/mobile-eas.yml" ] || err ".github/workflows/mobile-eas.yml missing"

echo
[ "$fail" = 0 ] && { echo "ALL P10 DEPLOY TESTS PASSED"; exit 0; } || { echo "SOME TESTS FAILED"; exit 1; }
