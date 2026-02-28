# Phase 5.4.c.i.3.a: Strict LibTooling Exporter Scope Controls

Date: 2026-02-27

## Goal

Add bounded exporter-scope controls for strict LibTooling runs so we can reduce traversal work without changing default library behavior, and lock the behavior with focused regression coverage.

## Changes

1. Added exporter option plumbing in `fragile-ast-exporter`:
   - Rust API:
     - `export_ast_with_options(...)`
     - `export_ast_cbor_with_options(...)`
   - C++ option:
     - `-skip-system-headers`
   - Visitor behavior under the option:
     - Prune declaration traversal for system-header decls.
     - Prune statement traversal for non-main-file statement trees.

2. Added strict-path wiring in `fragile-clang` / `fragilec`:
   - `TranspileOptions.libtooling_skip_system_headers` (default `false`).
   - `LibToolingParser::with_skip_system_headers(bool)` and parse-time forwarding to exporter options.
   - Strict compile path in `fragilec` now sets `libtooling_skip_system_headers: true`.

3. Added focused exporter regression:
   - `crates/fragile-ast-exporter/tests/integration_test.rs`
   - `test_export_ast_skip_system_headers_filters_isystem_decls`
   - Verifies `-isystem` declaration filtering while preserving local declaration export.

## Validation

Targeted tests:

- `cargo test -p fragile-ast-exporter --test integration_test test_export_ast_skip_system_headers_filters_isystem_decls -- --nocapture`
- `cargo test -p fragile-cli --bin fragilec -- --nocapture`
- `cargo test -p fragile-clang --test parser_backend_parity_tests -- --nocapture`

All passed.

## Replay Evidence

Direct strict capitalize backend-surface replay was rerun:

- Command:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_capitalize_backend_surface_delta_capture -- --ignored --nocapture`
- Latest relevant run root with this scope-control state:
  - `/tmp/fragile_real_world_rapidjson_strict_capitalize_backend_surface_delta_3164086_1772236441497402057`
- Manifest:
  - `.../strict_capitalize_backend_surface_delta_logs/strict_capitalize_backend_surface_delta_manifest.txt`

Observed result remained:

- `backend=libtooling compile_status=124`
- `compile_timed_out=true`
- `first_failure_class=compile_timeout`
- `sidecar_exists=false`

So 5.4.c.i.3.a is complete (scope-control infrastructure + regression lock), but timeout root cause remains and is carried into 5.4.c.i.3.b.
