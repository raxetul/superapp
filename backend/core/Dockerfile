# syntax=docker/dockerfile:1
#
# Multi-stage build for the SuperApp backend (loco.rs, TR-10-003).
# Build context is `backend/core` (self-contained: `migration` is nested
# here; `backend/modules/reference` is only a [dev-dependencies] path dep,
# not needed to build the release binary — see docs/phases/p9-module-sdk.md).
#
#   docker build -f backend/core/Dockerfile -t superapp-backend:dev backend/core
#
# rustc 1.85 pinned to match backend/core/Cargo.toml's `rust-version` (the
# dev toolchain here is a source-tarball 1.85 build with no rustup — see the
# P3 build-toolchain notes); using the same version in CI avoids a resolver
# picking a dependency version the pinned local toolchain can't build.

FROM rust:1.85-slim-bookworm AS builder
WORKDIR /build

# pkg-config/libssl-dev: only needed transitively by some crates' build
# scripts at compile time. Runtime TLS is rustls (reqwest/sea-orm both pin
# `rustls-tls` / `runtime-tokio-rustls`, no OpenSSL), so none of this ships
# in the runtime stage below.
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release --bin core-cli --locked

# --- Runtime: minimal image, no build toolchain (TR-10-003 accept) ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /app --shell /usr/sbin/nologin superapp

WORKDIR /app
COPY --from=builder /build/target/release/core-cli ./core-cli
# loco resolves `config/{LOCO_ENV}.yaml` and the auth initializer resolves
# `authz/policies` both relative to the process CWD (no CARGO_MANIFEST_DIR
# baked in outside #[cfg(test)]) — both must sit next to the binary here.
COPY --from=builder /build/config ./config
COPY --from=builder /build/authz ./authz

USER superapp
ENV LOCO_ENV=production
EXPOSE 5150

HEALTHCHECK --interval=15s --timeout=5s --start-period=20s --retries=5 \
  CMD curl -fsS "http://localhost:${SUPERAPP_BACKEND_PORT:-5150}/health" || exit 1

ENTRYPOINT ["/app/core-cli"]
CMD ["start"]
