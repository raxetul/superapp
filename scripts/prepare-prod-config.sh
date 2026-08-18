#!/usr/bin/env bash
# TR-10-007 — materialize backend/core/config/production.yaml from its
# committed template (gitignored, never overwritten if already present so a
# deployer's real values survive re-runs). Run before building/starting the
# production backend image.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/backend/core/config/production.yaml.example"
DEST="$ROOT/backend/core/config/production.yaml"

if [ ! -f "$SRC" ]; then
  echo "FAIL: missing template $SRC" >&2
  exit 1
fi

if [ -f "$DEST" ]; then
  echo "ok  : $DEST already present — leaving it untouched"
  exit 0
fi

cp "$SRC" "$DEST"
echo "ok  : materialized $DEST from production.yaml.example"
