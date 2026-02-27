# Phase 5.4.a.ii: Real-world strict CMake backend-matrix replay

Date: 2026-02-26

## Scope

Complete the ignored real-world RapidJSON strict CMake backend-matrix replay for:

- `FRAGILEC_PARSER_BACKEND=libclang` (baseline)
- `FRAGILEC_PARSER_BACKEND=libtooling`

The replay must persist and validate a per-run delta manifest under:

- `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_*`

## Implementation

Updated `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`:

- Added `unique_prefixed_dir(prefix: &str)` and switched
  `run_rapidjson_strict_cmake_no_tests_backend_matrix_capture` to allocate
  unique run roots at:
  `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_<pid>_<nanos>`.
- Extended `strict_cmake_backend_matrix_manifest.txt` with:
  - `run_root=<abs path>`
  - existing baseline + per-backend delta fields (`configure_status_delta_vs_baseline`,
    `build_status_delta_vs_baseline`, `class_delta_vs_baseline`,
    `e0425_delta_vs_baseline`).
- Strengthened ignored test
  `test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure`
  to validate:
  - run-root path prefix contract (`/tmp/..._backend_matrix_*`)
  - required artifact presence
  - baseline marker
  - `run_root` marker
  - per-backend manifest/delta fields.

## Validation

Run leaf test:

```bash
cargo test -p fragile-clang --test real_world_rapidjson_tests \
  test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure \
  -- --ignored --nocapture
```

Then run full suite:

```bash
cargo test
```
