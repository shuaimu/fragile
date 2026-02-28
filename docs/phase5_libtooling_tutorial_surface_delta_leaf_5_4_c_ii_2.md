# Phase 5.4.c.ii.2: Direct strict `tutorial.cpp` backend-surface capture

Date: 2026-02-28

## Goal
Add a direct strict `tutorial.cpp` LibTooling-vs-libclang replay (with stage timing and sidecar metadata) to isolate and classify the first non-timeout blocker after first-failure alignment (`5.4.c.ii.1`).

## Implementation
Updated `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`:
- Refactored single-TU backend-surface replay into a reusable harness:
  - `run_rapidjson_strict_single_tu_backend_surface_delta_capture`
  - config struct: `StrictSingleTuBackendSurfaceCaptureConfig`
- Kept existing `capitalize.cpp` capture through config wrapper.
- Added new `tutorial.cpp` wrapper + artifacts:
  - run-root prefix: `/tmp/fragile_real_world_rapidjson_strict_tutorial_backend_surface_delta_*`
  - manifest: `strict_tutorial_backend_surface_delta_manifest.txt`
  - compile step artifacts per backend: `compile_tutorial.{status,stdout,stderr}`
  - timing trace: `transpile_stage_timing.log`
  - first-failure files: command/stderr/class
- Added ignored real-world regression:
  - `test_real_world_rapidjson_strict_tutorial_backend_surface_delta_capture`

## Focused validation
- Compile-level check:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests tutorial_backend_surface_delta -- --nocapture`
- Real-world ignored replay:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_tutorial_backend_surface_delta_capture -- --ignored --nocapture`

## Latest evidence
Run root:
- `/tmp/fragile_real_world_rapidjson_strict_tutorial_backend_surface_delta_3897896_1772245938987729305`

Manifest summary:
- `backend=libclang compile_status=1 compile_timed_out=false first_failure_class=other_rustc_error sidecar_exists=true transpile_status=completed`
- `backend=libtooling compile_status=1 compile_timed_out=false first_failure_class=non_rustc_error sidecar_exists=false transpile_status=error`

First non-timeout blocker isolated/classified for LibTooling tutorial lane:
- `LibTooling parse failed: AST export failed with code 1`
- Captured against source: `example/tutorial/tutorial.cpp`

## Outcome
`tutorial.cpp` now has a deterministic direct backend A/B replay artifact set with stage metadata and explicit non-timeout blocker classification, unblocking the next fix leaf (`5.4.c.ii.3`).
