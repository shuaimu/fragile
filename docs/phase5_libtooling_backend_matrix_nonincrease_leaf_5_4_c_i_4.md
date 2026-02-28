# Phase 5.4.c.i.4: strict backend-matrix non-increase gate (configure/build/class/E0425 + timeout incidence)

Date: 2026-02-28

## Goal
Make strict real-world RapidJSON backend-matrix replay enforce that LibTooling deltas are non-increasing against the latest completed baseline capture, including timeout incidence.

## What changed
- Extended strict backend-matrix manifest emission in `real_world_rapidjson_tests.rs` to include:
  - `timeout_incidence_delta_vs_baseline`
- Added reusable delta snapshot parsing/comparison helpers:
  - `BackendMatrixDeltaSnapshot`
  - `compute_backend_matrix_delta_snapshot`
  - `ensure_backend_matrix_delta_non_increase`
  - `latest_completed_backend_matrix_delta_baseline`
- Added legacy compatibility for historical manifests that predate timeout-delta field:
  - timeout delta is derived from `build_timed_out` and `baseline_build_timed_out` when missing.
- Tightened ignored real-world gate:
  - `test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure`
  - now checks timeout-delta marker and applies non-increase enforcement vs latest completed LibTooling baseline manifest.

## Focused tests added
- `test_parse_backend_matrix_delta_snapshot_from_manifest_line`
- `test_parse_backend_matrix_delta_snapshot_from_legacy_manifest_without_timeout_delta_field`
- `test_ensure_backend_matrix_delta_non_increase_enforces_all_dimensions`

## Execution evidence
Latest strict backend-matrix run root:
- `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_3627392_1772240296118782736`

Latest manifest line (LibTooling):
- `backend=libtooling configure_status=0 build_status=124 build_timed_out=true first_failure_class=compile_timeout first_failure_e0425_count=0 ... build_status_delta_vs_baseline=122 class_delta_vs_baseline=true e0425_delta_vs_baseline=0 timeout_incidence_delta_vs_baseline=1`

Prior baseline manifest used for non-increase comparison:
- `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_3530443_1772239298676904148/strict_cmake_backend_matrix_logs/strict_cmake_backend_matrix_manifest.txt`
- Prior LibTooling delta line:
  - `... build_status_delta_vs_baseline=122 class_delta_vs_baseline=true e0425_delta_vs_baseline=1`
  - timeout incidence delta is derived as `1` from `build_timed_out=true` and `baseline_build_timed_out=false`.

Result:
- Non-increase gate holds (`configure/build/class/timeout` unchanged; `E0425` delta improved `1 -> 0`).
