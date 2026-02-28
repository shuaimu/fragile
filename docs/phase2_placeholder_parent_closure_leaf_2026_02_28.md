# Phase 2 parent closure: placeholder-degradation task

## Scope
- Phase 2 (`P4`) first remaining unchecked parent task in this iteration:
  - `Fix placeholder degradation for required rapidjson template types (Reader, handlers, writers, streams).`

## Analysis
- This parent task was stale because item-3 breakdown work (`3.1`..`3.4`) was already completed with replay and unit/regression evidence.
- Remaining work in this iteration is closure hygiene plus TODO guard enforcement.
- Estimated size is small (<500 LOC) and limited to TODO/test/docs updates.

## Change
- Marked the Phase 2 placeholder-degradation parent task complete in `TODO.md` with evidence summary.
- Added deterministic TODO guard `test_todo_keeps_phase2_placeholder_parent_task_closed` in `real_world_rapidjson_tests.rs`.

## Verification
- Focused guards and behavior checks:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_todo_keeps_phase2_placeholder_parent_task_closed -- --nocapture`
  - `cargo test -p fragile-clang --lib test_rapidjson_concrete_document_template_impl_emits_resolved_methods_without_generic_surface_fallbacks -- --nocapture`
  - `cargo test -p fragile-clang --lib test_rapidjson_generic_reader_template_impl_emits_runtime_parse_surface_fallbacks -- --nocapture`
- Full suite:
  - `cargo test`
