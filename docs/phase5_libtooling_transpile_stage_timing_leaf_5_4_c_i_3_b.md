# Phase 5.4.c.i.3.b: Strict Transpile Stage Timing Capture

Date: 2026-02-28

## Goal

Add deterministic strict transpile-stage timing capture (`parse`/`export`/`enrichment`/`codegen`) for direct RapidJSON `capitalize.cpp` backend replay so timeout/perf hotspots are observable from persisted artifacts.

## Changes

1. Added stage-timing trace support in `fragile-clang`:
   - New option: `TranspileOptions.stage_timing_trace_path: Option<PathBuf>`.
   - `transpile_cpp_to_rust_with_options(...)` now emits stage events (start/end/skip) and summary/status when a trace path is set.
   - Captured stages:
     - `parse`
     - `export`
     - `enrichment`
     - `codegen`
   - Traces are best-effort and append-as-you-go so partial progress is retained even on failure/timeout.

2. Wired strict driver env plumbing in `fragilec`:
   - New env var: `FRAGILEC_TRANSPILE_STAGE_TIMING_PATH`.
   - Strict compile path forwards this into `TranspileOptions.stage_timing_trace_path`.
   - Updated `fragilec --fragilec-help` environment documentation.

3. Extended direct RapidJSON capitalize backend-surface replay capture:
   - `run_rapidjson_strict_capitalize_backend_surface_delta_capture` now sets per-backend timing trace paths (`backend_*/transpile_stage_timing.log`).
   - Added parsing and manifest fields for timing metadata (`*_parse_ms`, `*_export_ms`, `*_enrichment_ms`, `*_codegen_ms`, `*_total_ms`, last-stage/status markers, timing trace path/existence).
   - Updated ignored real-world assertion coverage to require timing traces and manifest timing markers.

4. Added focused regressions:
   - `crates/fragile-clang/tests/parser_backend_parity_tests.rs`
     - `test_transpile_stage_timing_trace_contains_expected_stages_for_backends`
   - `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`
     - `test_parse_transpile_stage_timing_trace_supports_complete_and_partial_logs`

## Validation

Targeted tests:

- `cargo test -p fragile-clang --test parser_backend_parity_tests test_transpile_stage_timing_trace_contains_expected_stages_for_backends -- --nocapture`
- `cargo test -p fragile-clang --test real_world_rapidjson_tests test_parse_transpile_stage_timing_trace_supports_complete_and_partial_logs -- --nocapture`
- `cargo test -p fragile-cli --bin fragilec -- --nocapture`
- `cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_capitalize_backend_surface_delta_capture -- --ignored --nocapture`

Full suite:

- `cargo test`

All passed.

## Replay Evidence

Latest direct strict capitalize backend-surface replay run root:

- `/tmp/fragile_real_world_rapidjson_strict_capitalize_backend_surface_delta_3367106_1772238106322955670`

Manifest:

- `.../strict_capitalize_backend_surface_delta_logs/strict_capitalize_backend_surface_delta_manifest.txt`

Timing highlights from that run:

- `libclang`: `transpile_total_ms=73352` (`parse_ms=2474`, `codegen_ms=70877`)
- `libtooling`: `transpile_total_ms=616` (`export_ms=403`, `parse_ms=8`, `enrichment_ms=5`, `codegen_ms=199`)

In this captured state, both backends emitted timing traces and sidecars; `libtooling` no longer hit `compile_timeout` in this direct replay and now fails as a regular compile error (`E0425`), with stage timing evidence persisted for follow-on hotspot/regression work.
