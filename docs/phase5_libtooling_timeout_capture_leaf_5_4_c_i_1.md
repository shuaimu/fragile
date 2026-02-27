# Phase 5.4.c.i.1: Bounded-Time Strict Backend-Matrix Capture

## Context
- Phase `5.4.b.i` established that LibTooling-primary strict replay can hit pathological compile latency/non-termination on the first TU (`capitalize.cpp`), which blocked deterministic first-failure evidence updates.
- Existing strict backend-matrix replay logic used unbounded `Command::output()` for build steps, so one hung backend could stall the full replay lane.

## Decision
- Introduce a shared bounded-time runner for build commands:
  - `run_command_with_timeout(command, timeout, context) -> (Output, timed_out)`
  - timeout configured by `RAPIDJSON_STRICT_CMAKE_BACKEND_MATRIX_BUILD_TIMEOUT_SECS` (currently `1200s` for real-world replay).
- Persist timeout outcomes as first-class replay metadata:
  - build status sentinel `124`
  - first-failure class `compile_timeout`
  - manifest field `build_timeout_secs=...`
  - per-backend manifest field `build_timed_out=true|false`

## Implementation Notes
- Updated strict real-world backend-matrix replay (`run_rapidjson_strict_cmake_no_tests_backend_matrix_capture`) to route backend build commands through the timeout helper.
- Updated local strict backend-matrix fixture helper to support deterministic timeout injection:
  - optional per-backend env-controlled sleep (`FRAGILEC_LOCAL_FIXTURE_SLEEP_BEFORE_FAIL_SECS`)
  - optional build timeout parameter for tests.
- Added/updated assertions in backend-matrix tests to validate timeout metadata and persisted artifacts.

## Validation
- Added regression: `test_rapidjson_strict_cmake_backend_matrix_local_fixture_classifies_backend_timeout`
  - forces timeout only on `libtooling` backend
  - asserts `compile_status=124`
  - asserts first-failure class file is `compile_timeout`
  - asserts `cmake_build.stderr` includes timeout diagnostic text.
- Replayed local backend-matrix fixture tests:
  - `cargo test -p fragile-clang strict_cmake_backend_matrix_local_fixture -- --nocapture`
  - result: both local backend-matrix tests passed.

## Follow-up
- Use this bounded-time capture as the guardrail for `5.4.c.i.2+` generated-surface and unresolved-name/call-shape regression reduction work.
