# Phase 5.4.c.ii.3.c - Backend-matrix E0425 delta scope correction

Date: 2026-02-28  
Status: Completed

## Scope

Leaf `5.4.c.ii.3.c` resolves the strict backend-matrix `libtooling` `E0425` delta regression by making first-failure capture reflect only the matched first failing TU, instead of aggregating downstream `-k` keep-going failures from later TUs.

## Root cause

`select_first_failing_compile_capture()` correctly matched the first failing command (`capitalize.cpp`) but `source_scoped_failure_payload()` returned stderr from that marker to end-of-stream.  
Under `cmake --build ... -k`, this included additional failure blocks (`condense`, `filterkeydom`, `tutorial`, etc.), inflating first-failure `E0425` count and classifying `libtooling` as `unresolved_name_or_type_e0425` even when the first TU block had no `E0425`.

## Changes

1. Scoped first-failure payload to a single TU block:
   - `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`
   - `source_scoped_failure_payload()` now slices from matched marker to the next failure marker (`[fragilec] fragile rustc object compile failed for ...` / `Error while processing ...`).

2. Added focused keep-going regression:
   - `test_select_first_failing_compile_capture_scopes_to_first_source_block_only`
   - Verifies later-TU `E0425` diagnostics are excluded from first-failure stderr/class/count.

## Validation

Targeted:

```bash
cargo test -p fragile-clang --test real_world_rapidjson_tests test_select_first_failing_compile_capture_ -- --nocapture
cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure -- --ignored --nocapture
```

Evidence run root:

- `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_150861_1772253650232752277`
- Manifest `backend=libtooling` line now records:
  - `build_status=2`
  - `build_timed_out=false`
  - `first_failure_class=other_rustc_error`
  - `first_failure_e0425_count=0`
  - `e0425_delta_vs_baseline=0`
  - `timeout_incidence_delta_vs_baseline=0`

Additional capture artifact:

- `backend_libtooling/first_failing_compile_stderr.txt` is now 586 lines with a single marker block for `capitalize.cpp` (no downstream TU markers, no `error[E0425]`).

## Follow-up

- `5.4.c.ii.3.d`: rerun ignored strict backend matrix and require `build_timed_out=false` / `first_failure_class!=compile_timeout` before closing 5.4.c.ii.3.
