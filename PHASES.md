# SuperApp — Agentic Development Phases

Ordered roadmap for agentic development. Checklist order = execution order.
`- [x]` = completed, `- [ ]` = pending; the first pending item is the current phase.
This file drives the bottom tmux phase strip (see `~/.claude/scripts/agentic-phases.sh`).

## Phases

- [x] Project definition & engineering standards (`project.md`)
- [x] Infrastructure & local env (helvetia-compose wiring, `.env` scaffolding)
- [x] Backend core bootstrap (loco.rs scaffold, config, SeaORM/DB, logging, response/validator)
- [x] Authentication, SSO & policy-based AuthZ (Rauthy IdP broker over OIDC via `openidconnect` RP, JWT access/refresh rotation, Cedar `cedar-policy` rule-based authorization, admin bootstrap, middleware)
- [x] Dynamic module loading system (plugin loader, `/modules/register`, health checks)
- [x] Real-time & messaging (HTTP2 SSE event stream, Kafka topics & consumers)
- [x] Frontend core (React + ShadCN, auth flows, role-based routing, module host)
- [x] Mobile core (React Native + Tamagui, auth, navigation, module host)
- [x] Module SDK & reference module (cross-platform contracts, sample end-to-end module)
- [ ] Testing, Docker & deployment (coverage gates, multi-stage builds, compose orchestration)
