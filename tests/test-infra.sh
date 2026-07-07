#!/usr/bin/env bash
# P2 acceptance tests — infra/local-env artifacts (TR-02-001/002/003/005/006).
# Runtime "services come up healthy" checks (TR-02-001/003 live) require the Docker
# daemon and are out of scope here; this validates the static contract + script behavior.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
fail=0
pass(){ echo "  ok  : $1"; }
err(){ echo "  FAIL: $1"; fail=1; }

echo "[TR-02-001] internal compose stack, superapp- prefix"
[ -f docker-compose.yml ] || err "docker-compose.yml missing"
for svc in postgres redis kafka prometheus grafana; do
  grep -q "container_name: superapp-$svc" docker-compose.yml 2>/dev/null \
    && pass "superapp-$svc" || err "container superapp-$svc missing"
done
if command -v docker >/dev/null 2>&1; then
  docker compose config -q 2>/dev/null && pass "compose file validates" || err "compose config invalid"
fi

echo "[TR-02-003] shared superapp network + named volumes"
grep -qE "^[[:space:]]+superapp:" docker-compose.yml 2>/dev/null && pass "superapp network" || err "superapp network missing"
grep -qE "^volumes:" docker-compose.yml 2>/dev/null && pass "named volumes block" || err "named volumes missing"

echo "[TR-02-002] .env scaffolding"
[ -f .env.example ] || err ".env.example missing"
for k in SUPERAPP_BACKEND_DATABASE_URL SUPERAPP_BACKEND_REDIS_URL SUPERAPP_BACKEND_KAFKA_BROKERS; do
  grep -q "^$k=" .env.example 2>/dev/null && pass "$k present" || err "$k missing in .env.example"
done
grep -qxF '.env' .gitignore 2>/dev/null && pass ".env gitignored" || err ".env not gitignored"

echo "[TR-02-005] connectivity check exits non-zero when deps unreachable"
[ -x scripts/check-connectivity.sh ] || err "scripts/check-connectivity.sh missing/not executable"
if [ -x scripts/check-connectivity.sh ]; then
  SUPERAPP_BACKEND_DB_PORT=1 SUPERAPP_BACKEND_REDIS_PORT=1 SUPERAPP_BACKEND_KAFKA_PORT=1 \
  SUPERAPP_BACKEND_PROM_PORT=1 SUPERAPP_BACKEND_GRAFANA_PORT=1 \
    scripts/check-connectivity.sh >/dev/null 2>&1 \
    && err "check returned 0 with all deps down (should be non-zero)" \
    || pass "non-zero on unreachable deps"
fi

echo "[TR-02-006] Makefile targets"
for t in up down logs ps env-check; do
  grep -qE "^$t:" Makefile 2>/dev/null && pass "make $t" || err "make $t target missing"
done

echo "[TR-02-004] loco config profiles source DB from env; test profile isolated"
DEV=backend/core/config/development.yaml; TST=backend/core/config/test.yaml
grep -q 'get_env(name="SUPERAPP_BACKEND_DATABASE_URL"' "$DEV" 2>/dev/null \
  && pass "dev sources SUPERAPP_BACKEND_DATABASE_URL" || err "dev config not env-sourced"
grep -q 'superapp_test' "$TST" 2>/dev/null \
  && pass "test profile targets isolated DB (superapp_test)" || err "test profile not isolated"

echo
[ "$fail" = 0 ] && { echo "ALL P2 INFRA TESTS PASSED"; exit 0; } || { echo "SOME TESTS FAILED"; exit 1; }
