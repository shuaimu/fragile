# 2026-03-14 CI Leaf 2.8.b.ii Deterministic Build-Phase Replay

## Scope

Leaf `2.8.b.ii`:

- Make CI build-phase replay deterministic to completion (no prolonged no-progress tail hangs).
- Persist a final `build_phase_test` exit-status artifact.

## LOC analysis

This leaf stays below the requested size bound:

- `scripts/ci_command_capture.py`: generic helper (< 250 LOC)
- `tests/python/test_ci_command_capture.py`: fixture coverage (< 220 LOC)
- TODO/doc evidence updates.

## Wrong-approach check

Checked against `docs/dev/wrong.md` and `docs/fragile-dev-book.md` section `1.3` before changes:

- no target-specific transpiler/codegen hacks,
- no force-native source bypass,
- no fake semantic fallback stubs.

The change is an execution-harness improvement only.

## Design

Added a generic replay utility:

- `scripts/ci_command_capture.py`

Key behavior:

- Runs arbitrary commands with two timeout guards:
  - inactivity timeout (`inactivity_timeout_seconds`)
  - wall timeout (`wall_timeout_seconds`)
- Persists deterministic artifacts under a run root:
  - `<name>.stdout.log`
  - `<name>.stderr.log`
  - `<name>.status`
  - `<name>.manifest.txt`
- Uses process-group execution (`start_new_session=True`) and group kill on timeout to avoid orphan descendant hangs.
- Uses non-blocking final pipe drain so inherited stdio handles from detached descendants cannot stall artifact finalization.
- Returns deterministic statuses:
  - command exit code on normal completion,
  - `124` on timeout (`timeout_reason=wall_timeout|inactivity_timeout`),
  - `127` for command-not-found.

## Regression coverage

Added fixture tests in `tests/python/test_ci_command_capture.py`:

- success path status/log capture,
- inactivity-timeout kill classification,
- wall-timeout classification,
- command-not-found classification,
- inherited-stdio background-descendant regression (runner exits without blocking).

## Replay evidence

Run root:

- `/tmp/fragile_ci_leaf_2_8b_ii_20260314_v2`

Commands executed:

- `python3 scripts/ci_command_capture.py ... --name build_phase_build --command cargo build --verbose`
- `python3 scripts/ci_command_capture.py ... --name build_phase_test --command cargo test --verbose`

Captured statuses:

- `build_phase_build.status=0`
- `build_phase_test.status=124`

Manifest classification:

- `build_phase_test.manifest.txt` includes `timed_out=true` and `timeout_reason=wall_timeout`.

Build-phase failure inventory in captured logs remains baseline-red and deterministic:

- first failing id in captured output: `test_e2e_access_specifiers`
- representative failing family includes `test_e2e_insertion_sort`, `test_e2e_binary_search_tree`, `test_e2e_pthread`, `test_variadic_template_transpile`.

## Outcome

Leaf `2.8.b.ii` is complete: CI-aligned build-phase replay now finalizes deterministically with a persisted terminal `build_phase_test` status artifact, enabling bounded follow-up fix work in `2.8.b.iii`.

