---
description: Summarize the current phase — a phase-titles overview table plus a detail sub-table for the current phase only
---
Summarize where the SuperApp stands, read-only. **Do not modify any files.**

1. Read `PHASES.md`. Render **one overview table** listing **every phase, titles only** (no per-phase requirement detail), with a status marker:

   | # | Phase | Status |
   |---|-------|--------|
   | 1 | … | ✅ done |
   | 2 | … | ✅ done |
   | 3 | … | ▶ current |
   | 4 | … | ⬜ pending |
   | … | … | ⬜ pending |

   The **current** phase is the **first unchecked** `- [ ]` item; everything above it is `done`, everything below is `pending`.

2. Then, for the **current phase only**, read `REQUIREMENTS.md` (the Phase ↔ Requirement Mapping table + each mapped requirement) and render a **detail sub-table**:

   ### ▶ Current phase: `<Pn>` — <title>

   | Req ID | Priority | Summary | Covered? |
   |--------|----------|---------|----------|
   | TR-0n-001 | MUST | … | ☐ / ✅ |
   | … | … | … | … |

   - Include the cross-cutting requirements that apply to every phase (e.g. TR-00-001 TDD).
   - "Covered?" reflects whether the requirement's **Accept** criteria are met by merged code + passing tests, as far as you can tell from the repo; mark `☐` when unverified rather than guessing.

3. Close with a one-line **next action** (e.g. "run `/next-phase` to implement `<Pn>`").

Only the current phase gets the detail sub-table — all other phases appear as title rows in the overview table and nowhere else.
