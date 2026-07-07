# SuperApp — project rules

## API error format

HTTP **error** responses (non-2xx) in this project use **RFC 9457 Problem
Details**, served as `Content-Type: application/problem+json` with members
`type`, `title`, `status`, `detail`, `instance`, and extension members
(e.g. an `errors` array of `{pointer, detail}` for field-level validation
failures). **Successful** responses keep the house success envelope
(`success`, `data`, `message`, `pagination` — see `project.md`). RFC 9457
is for errors only. Implement both as shared, typed Rust models; controllers
must not emit ad-hoc error JSON.

## Phase documentation (always)

Every phase implemented via `/next-phase` MUST ship a phase-specific document
under `docs/phases/` (e.g. `docs/phases/p3-backend-core-bootstrap.md`) that
**includes diagrams** — embedded **Mermaid** (or DrawIO) covering the phase's
architecture and flows. The document shall map each requirement (by ID) to
its design and the tests proving it. Treat this document as a deliverable of
the phase: the phase is not complete until it exists.

## Push changes at the end of each phase

When a phase completes (all its requirements green, phase document written,
and the `PHASES.md` checkbox marked `- [x]`), commit the phase's changes with
Conventional Commits and push them to the remote before starting the next
phase. Do not leave a completed phase's work unpushed.
