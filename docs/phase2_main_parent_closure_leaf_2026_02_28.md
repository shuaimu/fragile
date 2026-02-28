# Phase 2 parent closure: main rollback/drop task

## Scope
- Phase 2 (`P4`) first remaining unchecked parent task:
  - `Fix main rollback/drop behavior so real example main survives codegen + rustc object emission.`

## Analysis
- This parent task was stale: detailed entrypoint breakdown (`7.1`..`7.7`) was already complete with evidence and regression coverage.
- Remaining work is closure hygiene + guardrails, not new transpiler behavior.
- Estimated change size is small (<500 LOC) and isolated to TODO/test/docs updates.

## Change
- Marked the Phase 2 main rollback/drop parent as complete in `TODO.md` with explicit evidence summary.
- Added a deterministic TODO guard test `test_todo_keeps_phase2_main_parent_task_closed` in `real_world_rapidjson_tests.rs`.

## Verification
- Focused guards and behavior checks:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_todo_keeps_phase2_main_parent_task_closed -- --nocapture`
  - `cargo test -p fragile-clang --lib test_main_function_is_preserved_when_rollback_patterns_match -- --nocapture`
  - `cargo test -p fragile-cli --bin fragilec strict_compile_degraded_main_shape_still_exports_main_symbol -- --nocapture`
- Full suite:
  - `cargo test`
