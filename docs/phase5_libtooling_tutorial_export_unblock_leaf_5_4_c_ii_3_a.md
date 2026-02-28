# Phase 5.4.c.ii.3.a - LibTooling tutorial AST-export unblock

Date: 2026-02-28  
Status: Completed

## Scope

Leaf `5.4.c.ii.3.a` unblocks the `tutorial.cpp` LibTooling parse/export failure identified in `5.4.c.ii.2` (`AST export failed with code 1` on `rapidjson/document.h` const-member assignment) and locks the resulting post-export blocker classification in the direct strict tutorial replay.

## Changes

1. Enabled delayed template-body parsing for strict LibTooling C++ parses:
   - `crates/fragile-clang/src/lib.rs`
   - `libtooling_parser_for_path()` now appends `-fdelayed-template-parsing` for C++.

2. Preserved strict diagnostic semantics before LibTooling export:
   - `crates/fragile-clang/src/lib.rs`
   - `parse_libtooling_context()` now runs a libclang precheck via `parser_for_path_with_options(...).parse_file(...)` before LibTooling export.
   - This keeps existing scoped RapidJSON semantic tolerance behavior while still rejecting non-tolerated user diagnostics.

3. Added focused strict-CLI regressions for the new LibTooling behavior:
   - `crates/fragile-cli/src/bin/fragilec.rs`
   - `strict_compile_libtooling_ignores_rapidjson_const_assignment_parser_diagnostic`
   - `strict_compile_libtooling_does_not_ignore_non_rapidjson_const_assignment_diagnostic`

4. Removed stale-binary replay drift in real-world tests:
   - `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`
   - `ensure_fragilec_binary()` now builds `fragilec` once per test process (cached path via `OnceLock`) so ignored replays consume current code.

5. Updated ignored strict tutorial replay assertions to the new post-export state:
   - `test_real_world_rapidjson_strict_tutorial_backend_surface_delta_capture`
   - Now expects libtooling sidecar emission and a timeout-classified rustc blocker (not AST export failure).

## Validation

Targeted tests:

```bash
cargo test -p fragile-cli --bin fragilec strict_compile_libtooling_ -- --nocapture
cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_tutorial_backend_surface_delta_capture -- --ignored --nocapture
```

Observed strict tutorial replay evidence:

- Run root: `/tmp/fragile_real_world_rapidjson_strict_tutorial_backend_surface_delta_4038912_1772247674009414280`
- LibTooling manifest line:
  - `compile_status=124`
  - `compile_timed_out=true`
  - `first_failure_class=compile_timeout`
  - `first_failure_e0425_count=33`
  - `sidecar_exists=true`
  - `transpile_status=completed`
- `first_failing_compile_stderr.txt` no longer contains `AST export failed with code 1`; it now starts from `[fragilec] fragile rustc object compile failed ...`.

## Follow-up blockers (new leaves under 5.4.c.ii.3)

1. `5.4.c.ii.3.b`: remove timeout-bound rustc classification in direct tutorial replay.
2. `5.4.c.ii.3.c`: resolve backend-matrix E0425 delta regression after export unblock.
3. `5.4.c.ii.3.d`: require strict backend-matrix libtooling replay to finish with `build_timed_out=false`.

Additional matrix evidence from this iteration:

- Run root: `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_4048369_1772247951263464222`
- Manifest currently shows:
  - `backend=libtooling build_status=124 build_timed_out=true first_failure_class=compile_timeout first_failure_e0425_count=118 e0425_delta_vs_baseline=118 timeout_incidence_delta_vs_baseline=1`
  - Non-increase gate failed against prior baseline due E0425 delta regression.
