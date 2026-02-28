# Phase 2 parent closure: array-decay and pointer-cast lowering task

## Scope
- Phase 2 (`P4`) first remaining unchecked parent task in this iteration:
  - `Fix array decay and pointer cast lowering ([T; N] to pointer forms).`

## Analysis
- This parent task was stale because item-5 breakdown work (`5.1`..`5.4`) was already completed with focused regressions and strict replay evidence.
- Remaining work in this iteration is closure hygiene and TODO guard enforcement.
- Estimated change size is small (<500 LOC), limited to TODO/test/docs updates.

## Change
- Marked the Phase 2 array-decay/pointer-cast parent task complete in `TODO.md` with evidence summary.
- Added deterministic TODO guard `test_todo_keeps_phase2_array_decay_parent_task_closed` in `real_world_rapidjson_tests.rs`.

## Verification
- Focused guards and behavior checks:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_todo_keeps_phase2_array_decay_parent_task_closed -- --nocapture`
  - `cargo test -p fragile-clang --lib test_constructor_pointer_param_array_argument_decays_to_mut_ptr -- --nocapture`
  - `cargo test -p fragile-clang --lib test_call_expr_array_decay_prefers_mut_ptr_from_implicit_decay_type -- --nocapture`
  - `cargo test -p fragile-clang --lib test_call_pointer_param_borrows_value_lvalue_before_base_pointer_cast -- --nocapture`
- Full suite:
  - `cargo test`
