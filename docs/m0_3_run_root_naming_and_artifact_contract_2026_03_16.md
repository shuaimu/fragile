# M0.3 Run-Root Naming and Artifact Contract (2026-03-16)

## Objective

Close TODO leaf `M0.3` by defining and enforcing:

- milestone run-root naming for M0 capture tooling
- required artifact contract for M0.1 and M0.2 runs

## Scope and Sizing

This leaf is bounded and under 1000 LOC:

- new shared helper: `scripts/mako_rpc_milestone_contract.py`
- small wiring updates in:
  - `scripts/mako_rpc_strict_baseline.py`
  - `scripts/mako_rpc_parser_backend_ab.py`
- focused test additions in:
  - `tests/python/test_mako_rpc_milestone_contract.py`
  - existing M0.1/M0.2 script tests

No decomposition was required.

## Wrong-Approach Check

Reviewed `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md` before implementation.

Confirmed this change remains orchestration/contract only:

- no target-specific compile/parser/codegen behavior
- no semantic stubs/fallback method bodies
- no masked-success shortcuts

## Contract Definition

Run-root naming contract version `1`:

- pattern:
  - `^fragile_(m0_1_strict_baseline|m0_2_parser_backend_ab)_\d{8}T\d{6}Z_p\d+$`
- default run-root path generation now uses this format for both scripts when `--run-root` is omitted.
- explicit `--run-root` is still allowed; scripts record whether the provided name matches the contract.

Required artifact contracts:

- `M0.1` contract file:
  - `<run_root>/strict_baseline_required_artifacts_manifest.txt`
  - covers command/status logs and required stage manifests under the run root.
- `M0.2` contract file:
  - `<run_root>/parser_backend_ab_required_artifacts_manifest.txt`
  - covers parent A/B artifacts and required child strict baseline manifests.

Each contract manifest records:

- `required_artifact_count`
- per-artifact relpath + existence flag
- `missing_required_artifact_count`
- run-root naming validity fields

## Validation

Focused tests:

- `python3 -m unittest tests/python/test_mako_rpc_milestone_contract.py -v`
- `python3 -m unittest tests/python/test_mako_rpc_strict_baseline.py -v`
- `python3 -m unittest tests/python/test_mako_rpc_parser_backend_ab.py -v`

Full Python suite:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Full workspace suite (non-regression check):

- `cargo test --workspace --all-targets`

