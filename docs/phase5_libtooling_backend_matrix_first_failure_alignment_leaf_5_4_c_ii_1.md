# Phase 5.4.c.ii.1: Deterministic first-failure capture under `-k` + timeout

Date: 2026-02-28

## Goal
When strict backend-matrix replay uses `cmake --build -k` and the libtooling backend eventually times out, keep first-failure artifacts aligned to the actual earliest failing TU instead of the trailing driver invocation.

## Problem
Before this change, command selection used the last `fragilec_driver.log` invocation unconditionally. On timeout runs, that often pointed to a trailing TU (for example `tutorial.cpp`) while stderr started with an earlier failure (for example `capitalize.cpp` rustc/type-lowering errors), producing mismatched diagnostics.

## Implementation
Updated `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`:
- Added first-source extraction from captured streams using markers:
  - `[fragilec] fragile rustc object compile failed for <source>`
  - `Error while processing <source>`
- Added source-aware command matching in driver invocations:
  - match by absolute source path when present,
  - fallback to basename match (`/path/foo.cpp` ↔ `foo.cpp` in args).
- Added source-scoped stderr selection:
  - when source markers are present, capture starts at the matched marker,
  - fallback behavior remains unchanged (use full stderr/stdout and last invocation).

## Tests
Added/updated focused regressions:
- `test_select_first_failing_compile_capture_prefers_source_matched_invocation`
- `test_select_first_failing_compile_capture_matches_error_while_processing_marker`
- `test_select_first_failing_compile_capture_falls_back_to_last_invocation_without_source_marker`
- Existing success-path test retained:
  - `test_select_first_failing_compile_capture_returns_none_when_build_succeeds`

## Real-world evidence
Ignored replay rerun:
- `cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure -- --ignored --nocapture`

Latest run root:
- `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_3774874_1772243168637445021`

Observed change:
- `backend_libtooling/first_failing_compile_command.txt` now maps to `example/capitalize/capitalize.cpp` (the first marker in stderr), not trailing `tutorial.cpp`.
- Timeout behavior/classification unchanged and still deterministic:
  - `cmake_build.status=124`
  - `first_failing_compile_class=compile_timeout`

## Outcome
First-failure artifacts are now source-aligned under timeout pressure, making the next blocker-isolation leaf (`5.4.c.ii.2`) actionable and trustworthy.
