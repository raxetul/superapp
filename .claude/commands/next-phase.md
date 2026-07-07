---
description: Implement the current pending phase from PHASES.md, test-first, driven by its requirements
argument-hint: "[optional phase number, e.g. 3]"
---
You are implementing a phase of the SuperApp, **gated on full requirements coverage across the entire roadmap**.

**Gate — evaluate this FIRST, before reading anything else or touching code:**
0. Read the **Phase ↔ Requirement Mapping** table in `REQUIREMENTS.md`. If **any** phase row is still `TBD` (no requirements mapped), **STOP — do not implement anything.** Report every phase that is still unmapped and instruct the user to run `/next-phase-requirements` until every phase has requirements. Implementing *any* phase is forbidden while even one phase remains unspecified — this is a hard rule, not a warning.

**Only when every phase has requirements (no TBD remains):**

1. Read `PHASES.md`. The target phase is $ARGUMENTS if given, otherwise the **first unchecked** `- [ ]` item (the current phase).
2. Read `REQUIREMENTS.md` and collect every requirement mapped to that phase (see the Phase ↔ Requirement Mapping table), plus the cross-cutting requirements (e.g. TR-00-001 TDD).
3. Honor the locked project decisions: backend in **Rust** on the **loco.rs** framework (Axum + **SeaORM** ORM), SSO via **Rauthy + `openidconnect`**, authorization via **Cedar (`cedar-policy`)**, stack PostgreSQL/Redis/Kafka/Prometheus.
4. Follow **TDD** (TR-00-001): for each requirement, write failing tests from its **Accept** criteria first, implement until green, then refactor. Do not add behavior without a test in the same change.
5. When all of the phase's requirements pass their tests, mark the phase `- [x]` in `PHASES.md`.

Report which requirements you covered (by ID) and the tests proving each.
