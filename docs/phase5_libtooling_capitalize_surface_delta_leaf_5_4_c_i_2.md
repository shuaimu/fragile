# Phase 5.4.c.i.2: Direct `capitalize.cpp` Backend Surface Delta Refresh

Date: 2026-02-27

## Goal
- Capture fresh direct strict replay artifacts for RapidJSON `example/capitalize/capitalize.cpp` across `libclang` and `libtooling` with `FRAGILEC_KEEP_RS=1`.
- Refresh generated-surface delta inventory for the first timed-out TU in LibTooling-primary mode.

## Implementation
- Added helper in `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`:
  - `run_rapidjson_strict_capitalize_backend_surface_delta_capture`
- Added ignored real-world regression:
  - `test_real_world_rapidjson_strict_capitalize_backend_surface_delta_capture`
- Capture behavior:
  - runs strict compile per backend (`libclang`, `libtooling`)
  - sets `FRAGILEC_KEEP_RS=1` so sidecar is preserved next to object output
  - enforces bounded compile timeout (`180s`)
  - writes per-backend compile/first-failure artifacts and a delta manifest
  - records generated-surface inventory when sidecar exists:
    - total lines
    - placeholder count
    - rapidjson placeholder count
    - `std::ffi::c_void` alias count
    - `kParseErrorUnspecificSyntaxError` count

## Evidence Run
- Command:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_capitalize_backend_surface_delta_capture -- --ignored --nocapture`
- Run root:
  - `/tmp/fragile_real_world_rapidjson_strict_capitalize_backend_surface_delta_2762020_1772232332800721640`
- Manifest:
  - `/tmp/fragile_real_world_rapidjson_strict_capitalize_backend_surface_delta_2762020_1772232332800721640/strict_capitalize_backend_surface_delta_logs/strict_capitalize_backend_surface_delta_manifest.txt`

### Baseline (`libclang`)
- `compile_status=0`
- `compile_timed_out=false`
- `sidecar_exists=true`
- sidecar path:
  - `.../backend_libclang/capitalize.fragile.rs`
- inventory:
  - `surface_line_count=39066`
  - `surface_placeholder_count=56`
  - `surface_rapidjson_placeholder_count=2`
  - `surface_c_void_alias_count=172`
  - `surface_parse_unspecific_count=18`

### LibTooling-primary (`libtooling`)
- `compile_status=124`
- `compile_timed_out=true`
- `first_failure_class=compile_timeout`
- `sidecar_exists=false`
- `compile_capitalize.stderr` includes timeout diagnostic for `180s`.

## Outcome
- `5.4.c.i.2` replay artifact refresh is complete with deterministic manifested evidence for both backends.
- Generated-surface inventory is now directly captured in a repeatable test harness, ready for `5.4.c.i.3` fix validation.
