# Phase 5.4.c.i.3.c: Direct `capitalize.cpp` Hotspot Fix (No Timeout Gate)

Date: 2026-02-28

## Goal

Apply a hotspot-targeted parser/codegen fix from the stage-timing evidence in 5.4.c.i.3.b and require direct strict `capitalize.cpp` LibTooling replay to produce a sidecar within timeout bounds (no `compile_timeout`).

## Observed hotspot state before this leaf

After 5.4.c.i.3.b timing capture, direct replay already showed transpile stages completing quickly for LibTooling, but first-failure still included unresolved-name/type artifacts (`E0425`) tied to allocator-qualified RapidJSON StringBuffer spellings.

## Changes

1. RapidJSON StringBuffer alias normalization in strict codegen normalization pass:
   - Added normalization rewrite:
     - `GenericStringBuffer_UTF8_char__CrtAllocator` -> `GenericStringBuffer_UTF8`
   - File: `crates/fragile-clang/src/ast_codegen.rs`

2. Added C stdio `fprintf` extern declaration to the shared preamble stubs:
   - File: `crates/fragile-stl/src/file_header.rs`
   - This closes unresolved-name call-shape artifacts where generated code references `fprintf`.

3. Added focused regression coverage:
   - `crates/fragile-clang/src/ast_codegen.rs`
     - `test_preamble_emits_fprintf_extern_declaration`
     - `test_normalize_rapidjson_strict_baseline_artifacts_normalizes_string_buffer_alias`
   - `crates/fragile-clang/src/types.rs`
     - `test_rapidjson_generic_string_buffer_alias_normalization`

4. Tightened ignored direct replay gate:
   - `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`
   - `test_real_world_rapidjson_strict_capitalize_backend_surface_delta_capture` now requires:
     - `compile_timed_out=false`
     - non-timeout status/class
     - `sidecar_exists=true`

## Validation

Targeted tests:

- `cargo test -p fragile-clang --lib test_rapidjson_generic_string_buffer_alias_normalization -- --nocapture`
- `cargo test -p fragile-clang --lib test_preamble_emits_fprintf_extern_declaration -- --nocapture`
- `cargo test -p fragile-clang --lib test_normalize_rapidjson_strict_baseline_artifacts_normalizes_string_buffer_alias -- --nocapture`
- `cargo test -p fragile-stl`
- `cargo build -p fragile-cli --bin fragilec`
- `cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_capitalize_backend_surface_delta_capture -- --ignored --nocapture`

## Replay evidence

Latest direct replay run root:

- `/tmp/fragile_real_world_rapidjson_strict_capitalize_backend_surface_delta_3476060_1772238977899700230`

Manifest:

- `.../strict_capitalize_backend_surface_delta_logs/strict_capitalize_backend_surface_delta_manifest.txt`

Relevant LibTooling line:

- `backend=libtooling compile_status=1 compile_timed_out=false first_failure_class=other_rustc_error first_failure_e0425_count=0 sidecar_exists=true`

This confirms the timeout gate for this leaf is cleared while preserving deterministic sidecar/timing artifacts for follow-on regression burn-down.
