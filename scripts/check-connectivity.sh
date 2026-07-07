#!/usr/bin/env bash
# TR-02-005 — verify the app can reach every infrastructure dependency.
# Reads host:port from env (see .env.example) with sensible defaults; exits
# non-zero with a clear message if any dependency is unreachable.
set -u

check() {
  local name="$1" host="$2" port="$3"
  if timeout 2 bash -c "exec 3<>/dev/tcp/$host/$port" 2>/dev/null; then
    echo "ok  : $name reachable at $host:$port"; return 0
  else
    echo "FAIL: $name unreachable at $host:$port" >&2; return 1
  fi
}

rc=0
check "PostgreSQL" "${SUPERAPP_BACKEND_DB_HOST:-localhost}"      "${SUPERAPP_BACKEND_DB_PORT:-5432}"      || rc=1
check "Redis"      "${SUPERAPP_BACKEND_REDIS_HOST:-localhost}"   "${SUPERAPP_BACKEND_REDIS_PORT:-6379}"   || rc=1
check "Kafka"      "${SUPERAPP_BACKEND_KAFKA_HOST:-localhost}"   "${SUPERAPP_BACKEND_KAFKA_PORT:-9092}"   || rc=1
check "Prometheus" "${SUPERAPP_BACKEND_PROM_HOST:-localhost}"    "${SUPERAPP_BACKEND_PROM_PORT:-9090}"    || rc=1
check "Grafana"    "${SUPERAPP_BACKEND_GRAFANA_HOST:-localhost}" "${SUPERAPP_BACKEND_GRAFANA_PORT:-3001}" || rc=1

if [ "$rc" -eq 0 ]; then
  echo "all dependencies reachable"
else
  echo "one or more dependencies unreachable" >&2
fi
exit "$rc"
