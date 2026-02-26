# Phase 5.4.a.i: Local strict CMake backend-matrix replay fixture

Date: 2026-02-26

## Scope

Implement a deterministic local replay gate for strict CMake backend matrix coverage:

- `FRAGILEC_PARSER_BACKEND=libclang`
- `FRAGILEC_PARSER_BACKEND=hybrid`
- `FRAGILEC_PARSER_BACKEND=libtooling`

The gate asserts there is no new first-failure delta (build status, class, `error[E0425]` count)
relative to the `libclang` baseline in this local fixture.

This leaf stayed small (<500 LOC), implemented in:

- `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`

## Design

Added:

- helper: `run_local_strict_cmake_no_tests_backend_matrix_capture_fixture`
- test: `test_rapidjson_strict_cmake_backend_matrix_local_fixture_keeps_baseline_deltas`

The helper:

1. Reuses the existing local strict-CMake first-failure fixture project.
2. Runs configure/build three times (one build dir per backend).
3. Captures per-backend logs (`cmake_*`, `fragilec_driver.log`, first-failure files).
4. Computes per-backend first-failure metadata (`class`, `E0425` count).
5. Writes:
   `strict_cmake_backend_matrix_local_fixture_manifest.txt`
   with baseline + delta fields.

The fake `fragilec` wrapper now logs `parser_backend=<value>` so the test can verify
backend env propagation in the driver log.

## Validation

Executed and passing:

- `cargo test -p fragile-clang --test real_world_rapidjson_tests test_rapidjson_strict_cmake_backend_matrix_local_fixture_keeps_baseline_deltas -- --nocapture`

Then full regression:

- `cargo test`
