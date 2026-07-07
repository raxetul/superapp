---
description: Add or refine requirements in REQUIREMENTS.md; offers the phase list to pick from if none is given
argument-hint: "[phase number] [topic/notes]"
---
Add or refine requirements in `REQUIREMENTS.md`. Notes/topic (if any): $ARGUMENTS

**Step 0 — pick the phase.**
- If $ARGUMENTS names a phase number, use it.
- Otherwise, read `PHASES.md` and the Phase ↔ Requirement Mapping table in `REQUIREMENTS.md`, then present the phases as a selectable list — each with its completion state and how many requirements are already mapped (or `TBD`) — using the AskUserQuestion tool. Wait for me to select exactly one before continuing.

**Then add the requirements:**
- Two groups only: **Feature** (`FR-PP-NNN`, user/product-facing) and **Technical** (`TR-PP-NNN`, stack/infra/quality). Place each new requirement in the right group section.
- **ID format** `TR-PP-NNN` / `FR-PP-NNN`: `PP` = the requirement's phase number, zero-padded (`00` = cross-cutting/all-phases); `NNN` = the next free sequence **within that phase + group**, zero-padded, starting at `001`. Never renumber existing IDs.
- Each requirement needs: a single **shall** statement, a `**Phase:**` tag (must match `PP`), a `**Priority:**` (MUST/SHOULD/COULD), and an **Accept** line written as concrete, testable done-criteria (this is the TDD test contract per TR-00-001).
- Keep statements atomic and independently verifiable — split if a requirement bundles two behaviors.
- Update the **Phase ↔ Requirement Mapping** table at the bottom with the new IDs.

Before writing, propose the draft requirements and let me confirm. Then apply.
