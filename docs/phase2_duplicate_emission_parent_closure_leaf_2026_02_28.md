# Phase 2 parent closure: duplicate-emission task

## Scope
- Phase 2 (`P4`) first remaining unchecked parent task in this iteration:
  - `Fix duplicate emission pipeline (helpers/types/templates) to eliminate E0428 families.`

## Analysis
- This parent task was stale because item-2 breakdown work (`2.1`..`2.7`) was already complete and validated.
- Remaining work for this iteration is closure hygiene and regression guard coverage.
- Estimated size is small (<500 LOC), limited to TODO/test/docs updates.

## Change
- Marked the Phase 2 duplicate-emission parent task complete in `TODO.md` with evidence summary.
- Added deterministic TODO guard `test_todo_keeps_phase2_duplicate_emission_parent_task_closed` in `real_world_rapidjson_tests.rs`.

## Verification
- Focused guards and behavior checks:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_todo_keeps_phase2_duplicate_emission_parent_task_closed -- --nocapture`
  - `cargo test -p fragile-clang --lib test_preamble_owned_helper_functions_are_not_reemitted -- --nocapture`
  - `cargo test -p fragile-clang --lib test_preamble_owned_types_and_aliases_are_not_redefined -- --nocapture`
- Full suite:
  - `cargo test`
