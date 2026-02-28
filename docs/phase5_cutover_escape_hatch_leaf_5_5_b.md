# Phase 5.5.b: Preserve `libclang` Escape Hatch During Hardening

Date: 2026-02-28

## Goal

After switching strict default backend to LibTooling (`5.5.a`), keep an explicit
`libclang` fallback lane available and regression-guarded for one hardening
window.

## Changes

1. Strict CLI regression coverage (`crates/fragile-cli/src/bin/fragilec.rs`)
   - Added `strict_compile_source_with_libclang_backend_exports_main_symbol`.
   - Test compiles a TU with helper + `main` via
     `strict_compile_source_to_object_with_backend(..., ParserBackend::Libclang)`
     and asserts object emission + `main` symbol export.

2. RapidJSON local baseline fixture lane (`crates/fragile-clang/tests/real_world_rapidjson_tests.rs`)
   - `compile_example_with_cxx_env` now pins:
     - `FRAGILEC_PARSER_BACKEND=libclang`
   - This keeps the local fragilec-driver no-STL baseline deterministic as an
     explicit escape-hatch replay lane while LibTooling remains default.

## Validation

- Focused strict CLI backend regression:
  - `cargo test -p fragile-cli --bin fragilec strict_compile_source_with_libclang_backend_exports_main_symbol -- --nocapture`
- Focused RapidJSON local escape-hatch baseline:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_rapidjson_fragilec_driver_no_stl_examples_local_fixture_success -- --nocapture`
- Full workspace regression:
  - `cargo test`

All passed in this iteration.
