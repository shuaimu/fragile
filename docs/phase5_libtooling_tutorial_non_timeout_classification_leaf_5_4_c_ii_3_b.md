# Phase 5.4.c.ii.3.b - Tutorial non-timeout first-failure classification

Date: 2026-02-28  
Status: Completed

## Scope

Leaf `5.4.c.ii.3.b` converts the post-export `tutorial.cpp` LibTooling blocker from timeout-bound replay classification to a deterministic non-timeout first-failure class.

## Root cause

`run_command_with_timeout()` captured child stdout/stderr through pipes but only read them after process exit (`wait_with_output` path).  
When `fragilec` returned large rustc diagnostics, child writes could block on full pipes, causing a false timeout classification (`status=124`, `compile_timeout`) even though rustc had already reached deterministic failure.

## Changes

1. Updated timeout helper to drain child output concurrently:
   - `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`
   - `run_command_with_timeout()` now:
     - takes child stdout/stderr handles immediately after spawn,
     - drains both streams via reader threads while polling for timeout,
     - joins readers and reconstructs `Output` after child exit/kill.

2. Added focused regression for false-timeout prevention:
   - `test_run_command_with_timeout_drains_large_stderr_without_false_timeout`
   - Uses a large-stderr fixture command and asserts:
     - `timed_out == false`
     - exit status preserved (`1`)
     - full stderr payload captured.

3. Updated ignored tutorial replay gate to require deterministic non-timeout classification:
   - `test_real_world_rapidjson_strict_tutorial_backend_surface_delta_capture`
   - LibTooling now asserts:
     - `compile_timed_out == false`
     - `compile_status != 124`
     - `first_failure_class == unresolved_name_or_type_e0425`
     - first failing stderr contains rustc blocker + `error[E0425]`, and no AST export failure marker.

## Validation

Targeted:

```bash
cargo test -p fragile-clang --test real_world_rapidjson_tests test_run_command_with_timeout_drains_large_stderr_without_false_timeout -- --nocapture
cargo test -p fragile-clang --test real_world_rapidjson_tests test_rapidjson_strict_cmake_backend_matrix_local_fixture_classifies_backend_timeout -- --nocapture
cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_tutorial_backend_surface_delta_capture -- --ignored --nocapture
```

Evidence run root:

- `/tmp/fragile_real_world_rapidjson_strict_tutorial_backend_surface_delta_62309_1772252038282048350`
- Manifest libtooling line now records:
  - `compile_status=1`
  - `compile_timed_out=false`
  - `first_failure_class=unresolved_name_or_type_e0425`
  - `first_failure_e0425_count=33`
  - `sidecar_exists=true`
  - `transpile_status=completed`

## Follow-up

- `5.4.c.ii.3.c`: resolve backend-matrix LibTooling `E0425` delta regression.
- `5.4.c.ii.3.d`: re-run ignored strict backend matrix and require `build_timed_out=false` / `first_failure_class!=compile_timeout`.
