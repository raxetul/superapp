---
description: Regenerate PHASES.md from the requirements catalog so phases and requirement IDs stay aligned
---
Regenerate `PHASES.md` from `REQUIREMENTS.md`.

1. Read the **Phase ↔ Requirement Mapping** table in `REQUIREMENTS.md`.
2. Ensure every phase line in `PHASES.md` reflects the requirements now mapped to it (titles/scope consistent with the catalog), and append the mapped requirement IDs in parentheses so each phase line shows its coverage.
3. Preserve the existing checklist format and completion state (`- [x]` / `- [ ]`) — this file drives the tmux phase strip, so keep it one checklist item per line.
4. Flag any requirement that is mapped to no phase, or any phase with zero requirements, instead of silently dropping it.

Show a diff summary of what changed and why.
