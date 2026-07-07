---
description: Implement one requirement test-first (red → green → refactor)
argument-hint: "<requirement ID, e.g. TR-03-002>"
---
Implement requirement $ARGUMENTS using strict TDD (TR-00-001).

1. Look up $ARGUMENTS in `REQUIREMENTS.md`; restate its **shall** statement and **Accept** criteria.
2. **Red:** write automated tests that encode each Accept criterion and confirm they fail for the right reason. Use the stack's idiomatic test tooling (Rust: `cargo test` / `#[tokio::test]`; frontend/mobile: their test runners).
3. **Green:** write the minimum implementation to pass.
4. **Refactor:** clean up while keeping tests green; match surrounding code style.
5. Report each Accept criterion → the test(s) covering it.

Do not implement behavior beyond what the requirement and its tests specify.
