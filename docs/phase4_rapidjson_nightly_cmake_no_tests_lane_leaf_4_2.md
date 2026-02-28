# Phase 4.2: RapidJSON strict no-tests runtime lane in nightly CI

## Scope
- Phase 4 hardening (`P1`), leaf `4.2`.
- Goal: keep an always-on nightly lane that executes the strict RapidJSON CMake no-tests full-build replay including `condense`/`pretty` runtime checks.

## Change
- Updated workflow matrix in `.github/workflows/rapidjson-nightly.yml` to include:
  - `test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure`
- Updated nightly workflow coverage guard in `real_world_rapidjson_tests.rs`:
  - Added the same test name to `RAPIDJSON_NIGHTLY_REQUIRED_TEST_NAMES`.

## Evidence
- Focused guard coverage:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_rapidjson_nightly_workflow_keeps_matrix_coverage -- --nocapture`
- Full regression suite:
  - `cargo test`
