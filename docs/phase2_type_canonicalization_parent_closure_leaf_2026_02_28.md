# Phase 2 parent closure: type-canonicalization task

## Scope
- Phase 2 (`P4`) first remaining unchecked parent task in this iteration:
  - `Fix libc/libstd type canonicalization (__FILE, atomic flag types, void aliases).`

## Analysis
- This parent task was stale because item-4 breakdown work (`4.1`..`4.4`) was already completed with focused regressions and strict replay evidence.
- Remaining work in this iteration is closure hygiene and TODO guard enforcement.
- Estimated change size is small (<500 LOC) and limited to TODO/test/docs updates.

## Change
- Marked the Phase 2 type-canonicalization parent task complete in `TODO.md` with evidence summary.
- Added deterministic TODO guard `test_todo_keeps_phase2_type_canonicalization_parent_task_closed` in `real_world_rapidjson_tests.rs`.

## Verification
- Focused guards and behavior checks:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_todo_keeps_phase2_type_canonicalization_parent_task_closed -- --nocapture`
  - `cargo test -p fragile-clang --lib test_file_like_aliases_lower_to_opaque_c_void -- --nocapture`
  - `cargo test -p fragile-clang --lib test_std_identity_aliases_lower_to_generated_identity_type -- --nocapture`
  - `cargo test -p fragile-clang --lib test_cxx_atomic_base_impl_bool_alias_normalizes_to_impl_bool -- --nocapture`
- Full suite:
  - `cargo test`
