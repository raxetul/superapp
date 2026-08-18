# syntax=docker/dockerfile:1
#
# Multi-stage build for the reference backend module (TR-09-007), the
# pattern every backend module image follows (TR-10-006 publishes these
# alongside the app images). `reference-module` path-depends on
# `../../sdk` (backend/sdk), so the build context is `backend/`, not this
# directory:
#
#   docker build -f backend/modules/reference/Dockerfile -t superapp-module-reference:1.0.0 backend

FROM rust:1.85-slim-bookworm AS builder
WORKDIR /build

COPY sdk ./sdk
COPY modules/reference ./modules/reference
WORKDIR /build/modules/reference
RUN cargo build --release --bin reference-module --locked

# --- Runtime: minimal image, no build toolchain ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /app --shell /usr/sbin/nologin superapp

WORKDIR /app
COPY --from=builder /build/modules/reference/target/release/reference-module ./reference-module

USER superapp
# Matches src/main.rs's PORT default (8080) and the SDK ModuleServer contract
# (/health, /ready, /sdk, /manifest, /config) the core's DockerRuntime polls.
ENV PORT=8080
EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=5 \
  CMD curl -fsS "http://localhost:${PORT}/health" || exit 1

ENTRYPOINT ["/app/reference-module"]
