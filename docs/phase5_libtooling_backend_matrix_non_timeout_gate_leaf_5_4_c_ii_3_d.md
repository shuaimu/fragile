# Phase 5.4.c.ii.3.d - Backend-matrix non-timeout gate rerun

Date: 2026-02-28  
Status: Completed

## Scope

Leaf `5.4.c.ii.3.d` requires a fresh ignored strict backend-matrix replay proving `backend=libtooling` no longer times out and is not classified as `compile_timeout`.

## Changes

1. Added explicit non-timeout gate assertions in:
   - `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`
   - `test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure`
   - New assertions require:
     - `!libtooling.build_timed_out`
     - `libtooling.build_status != 124`
     - `libtooling.first_failure_class != "compile_timeout"`

2. Reran the ignored strict backend-matrix replay to validate the new gate.

## Validation

Commands:

```bash
cargo test -p fragile-clang --test real_world_rapidjson_tests test_parse_backend_matrix_delta_snapshot_from_manifest_line -- --nocapture
cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure -- --ignored --nocapture
```

Evidence run root:

- `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_230969_1772255078158776485`

Manifest `backend=libtooling` line:

- `configure_status=0`
- `build_status=2`
- `build_timed_out=false`
- `first_failure_class=other_rustc_error`
- `first_failure_e0425_count=0`
- `e0425_delta_vs_baseline=0`
- `timeout_incidence_delta_vs_baseline=0`

The first-failure class file and stderr marker also confirm non-timeout classification:

- `backend_libtooling/first_failing_compile_class.txt` -> `other_rustc_error`
- `backend_libtooling/first_failing_compile_stderr.txt` starts with:
  - `[fragilec] fragile rustc object compile failed for .../example/capitalize/capitalize.cpp`
  - no timeout sentinel marker.

## Outcome

`5.4.c.ii.3` can now be closed with concrete replay evidence that `libtooling` backend-matrix execution is non-timeout and non-`compile_timeout`.
