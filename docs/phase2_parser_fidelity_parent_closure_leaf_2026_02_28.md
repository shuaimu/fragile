# Phase 2 parent closure: parser-fidelity `document.h` const-member assignment task

## Scope
- Phase 2 (`P4`) remaining unchecked parent task in this iteration:
  - `Fix parser fidelity issue causing document.h const-member assignment failure.`

## Analysis
- This parent task was stale: the parser-fidelity breakdown work (`1.1`..`1.4`) was already completed and the ordered failure-class ledger marks item 1 as `CLEARED`.
- Remaining work for this leaf is closure hygiene plus a deterministic TODO guard to prevent accidental reopening.
- Estimated change size is small (<500 LOC), limited to TODO/test/docs updates.

## Change
- Marked the Phase 2 parser-fidelity parent task complete in `TODO.md` with explicit evidence summary.
- Added deterministic TODO guard `test_todo_keeps_phase2_parser_fidelity_parent_task_closed` in `real_world_rapidjson_tests.rs`.

## Verification
- Focused guards and parser-fidelity checks:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_todo_keeps_phase2_parser_fidelity_parent_task_closed -- --nocapture`
  - `cargo test -p fragile-clang --lib test_parse_file_accepts_rapidjson_const_member_assignment_with_semantic_tolerance -- --nocapture`
  - `cargo test -p fragile-clang --lib test_parse_file_reports_non_matching_rapidjson_const_member_assignment_shape -- --nocapture`
- Full suite:
  - `cargo test`
