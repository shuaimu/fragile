# Phase 5.3.d.i: Strict replay backend-toggle regression gate for E0425 deltas

Date: 2026-02-26

## Scope

Implement `5.3.d.i` by adding a deterministic strict replay gate that runs with:

- `FRAGILEC_PARSER_BACKEND=libclang`
- `FRAGILEC_PARSER_BACKEND=hybrid`
- `FRAGILEC_PARSER_BACKEND=libtooling`

and asserts there is no unresolved-name/type (`error[E0425]`) delta relative to the
current strict baseline manifest entry.

This leaf stayed small (<500 LOC), implemented in
`crates/fragile-clang/tests/real_world_rapidjson_tests.rs`.

## Design

Added a local fixture-based strict replay helper and test:

- Helper:
  `run_local_strict_backend_toggle_e0425_delta_replay_fixture`
- Test:
  `test_rapidjson_strict_backend_toggle_local_fixture_keeps_e0425_delta_at_baseline`

Fixture behavior:

1. Creates a tiny C++ source with template + specialization-like call shape.
2. Runs `fragilec -std=c++11 -c ...` in strict mode three times with backend toggle env.
3. Captures per-backend logs:
   - `compile.status/stdout/stderr`
   - `fragilec_driver.log`
   - `first_failing_compile_command.txt`
   - `first_failing_compile_stderr.txt`
   - `first_failing_compile_class.txt`
4. Counts `error[E0425]` occurrences in first-failure stderr for each backend.
5. Writes `strict_backend_toggle_manifest.txt` including baseline (`libclang`) and
   backend deltas.

Manifest root:

- `/tmp/fragile_rapidjson_strict_backend_toggle_e0425_delta_*/strict_backend_toggle_logs/strict_backend_toggle_manifest.txt`

## Regression gate semantics

- Baseline backend: `libclang`
- For each backend (`libclang`, `hybrid`, `libtooling`):
  - `first_failure_e0425_count` must match baseline
  - `compile_status` must match baseline
  - first-failure classification must be consistent with count
    (`unresolved_name_or_type_e0425` iff count > 0)

This ensures backend toggling does not introduce new unresolved-name/type deltas in strict replay.

## Validation

Executed and passing:

- `cargo test -p fragile-clang --test real_world_rapidjson_tests test_rapidjson_strict_backend_toggle_local_fixture_keeps_e0425_delta_at_baseline -- --nocapture`
