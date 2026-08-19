# syntax=docker/dockerfile:1
#
# Multi-stage build for the SuperApp frontend (TR-10-004).
# Build context is `frontend/core`:
#
#   docker build -f frontend/core/Dockerfile -t superapp-frontend:dev frontend/core
#
# Vite bakes `VITE_*` values into the built bundle at build time (there's no
# server to inject them at container start for a static SPA), so dev/prod
# selection (TR-10-007) happens via `--build-arg` at image-build time — the
# ARGs below default to frontend/core/.env.example's values. Vite's
# `loadEnv()` gives `process.env`-sourced `VITE_*` vars priority over `.env`
# file values, so exporting them as ENV before `npm run build` is sufficient;
# no code change to `src/config/env.ts` (TR-07-007) was needed.

FROM node:22-slim AS builder
WORKDIR /build

COPY package.json package-lock.json ./
RUN npm ci

COPY . .

ARG VITE_API_BASE_URL=http://localhost:5150
ARG VITE_OIDC_ISSUER=http://localhost:8080/auth/v1
ARG VITE_OIDC_CLIENT_ID=superapp-web
ARG VITE_OIDC_REDIRECT_URI=http://localhost:5173/auth/callback
ARG VITE_OIDC_SCOPE="openid profile email"
ARG VITE_OIDC_POST_LOGOUT_REDIRECT_URI=http://localhost:5173/
ENV VITE_API_BASE_URL=${VITE_API_BASE_URL} \
    VITE_OIDC_ISSUER=${VITE_OIDC_ISSUER} \
    VITE_OIDC_CLIENT_ID=${VITE_OIDC_CLIENT_ID} \
    VITE_OIDC_REDIRECT_URI=${VITE_OIDC_REDIRECT_URI} \
    VITE_OIDC_SCOPE=${VITE_OIDC_SCOPE} \
    VITE_OIDC_POST_LOGOUT_REDIRECT_URI=${VITE_OIDC_POST_LOGOUT_REDIRECT_URI}

RUN npm run build

# --- Runtime: static file server, no Node/build toolchain (TR-10-004) ---
FROM nginxinc/nginx-unprivileged:1.27-alpine AS runtime

COPY --from=builder /build/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=5 \
  CMD wget -qO- http://localhost:8080/ >/dev/null || exit 1
