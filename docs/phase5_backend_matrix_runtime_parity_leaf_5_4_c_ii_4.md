# Phase 5.4.c.ii.4 - Strict backend-matrix runtime parity gate (`condense`/`pretty`)

Date: 2026-02-28  
Status: Completed

## Scope

Leaf `5.4.c.ii.4` requires strict backend-matrix runtime parity assertions for RapidJSON `condense`/`pretty` between `libclang` (baseline) and `libtooling`.

## Changes

1. Extended strict backend-matrix capture in:
   - `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`
   - `run_rapidjson_strict_cmake_no_tests_backend_matrix_capture`

2. Added runtime-target build and runtime capture per backend:
   - `cmake_build_target_condense.{status,stdout,stderr}`
   - `cmake_build_target_pretty.{status,stdout,stderr}`
   - `run_condense.{status,stdout,stderr}`
   - `run_pretty.{status,stdout,stderr}`

3. Extended backend-matrix manifest fields with runtime parity markers:
   - `condense_run_status`
   - `pretty_run_status`
   - `condense_stderr_empty`
   - `pretty_stderr_empty`
   - `condense_output_matches_expected`
   - `pretty_output_matches_expected`
   - `condense_output_matches_baseline`
   - `pretty_output_matches_baseline`
   - `runtime_parity_vs_baseline`
   - `condense_run_status_delta_vs_baseline`
   - `pretty_run_status_delta_vs_baseline`

4. Tightened ignored real-world gate:
   - `test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure`
   - Adds explicit runtime parity assertions for `libtooling` vs `libclang` baseline.

5. Added focused deterministic parity test:
   - `test_strict_backend_matrix_runtime_parity_vs_baseline_checks_status_stdout_and_stderr`
   - Locks parity semantics on runtime status/output and stderr-emptiness parity.

## Validation

Commands:

```bash
cargo test -p fragile-clang --test real_world_rapidjson_tests -- --nocapture
cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure -- --ignored --nocapture
```

Latest run root:

- `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_484423_1772259242504942926`

Manifest highlights (`strict_cmake_backend_matrix_manifest.txt`):

- `backend=libclang ... condense_run_status=-1 pretty_run_status=-1 ... runtime_parity_vs_baseline=true ...`
- `backend=libtooling ... condense_run_status=-1 pretty_run_status=-1 ... runtime_parity_vs_baseline=true ...`
- `condense_run_status_delta_vs_baseline=0`
- `pretty_run_status_delta_vs_baseline=0`
- `condense_output_matches_baseline=true`
- `pretty_output_matches_baseline=true`

## Notes

Current strict backend-matrix runtime artifacts show parity at the same failure mode (`-1` runtime status due missing built binaries after target build failures). This leaf closes on parity gate coverage and enforcement, not on runtime-success restoration.
