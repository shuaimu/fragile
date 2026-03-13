# RPC Compile Blocker Leaf 2.6.a Design (2026-03-13)

## Objective

Leaf `2.6` requires strict targeted `fragilec` replay evidence for `test_rpc`/`rpcbench` build closure,
but the direct full lane rerun is operationally heavy and mixes compile triage with runtime/perf gates.

Leaf `2.6.a` introduces deterministic build-only replay support so compile blocker iteration can proceed
with bounded-time single-lane strict captures.

## Scope Sizing

Estimated implementation was small (<500 LOC), so no additional decomposition was needed for `2.6.a`.

- script update: `scripts/mako_rpcbench_harness.py`
- regression tests: `tests/python/test_mako_rpcbench_harness.py`
- TODO/docs updates

## Decision

Add two generic execution controls to the existing harness:

- `--lanes`: deterministic lane subset selection (`clang`, `fragilec`)
- `--build-only`: execute `configure`/`clean`/`build` only, and skip `test_rpc` + rpcbench runtime

This keeps all outputs deterministic and explicit, while preventing runtime/perf verdicts from blocking
compile-triage loops.

## Wrong-Approach Check

Aligned with project constraints and `docs/dev/wrong.md`:

- no target-name conditionals for RPC binaries
- no synthesized semantic method bodies or fake compile-success behavior
- no force-native bypass
- skipped runtime/perf steps are explicitly recorded as skipped artifacts (`status=-1`), not hidden

## Implementation

Updated `scripts/mako_rpcbench_harness.py`:

- replaced fixed lane tuple usage with selected `cfg.lanes`
- added `--lanes` parsing/validation with deterministic dedup preserving user order
- added `--build-only` mode in execution path:
  - run configure/clean/build only
  - emit skipped `test_rpc` and trial runtime artifacts with status `-1`
  - classify lane failure from configure/clean/build only
  - force comparison `no_regression_verdict=not_executed`
- persisted new manifest metadata:
  - `lanes=<selected>`
  - `build_only=true|false`

Updated `tests/python/test_mako_rpcbench_harness.py`:

- `test_invalid_lane_name_is_rejected`
- `test_execution_mode_build_only_fragilec_lane_skips_runtime_and_qps_gate`

## Strict Replay Evidence

Bounded strict build-only replay run:

- run root: `/tmp/fragile_rpc_leaf_2_6a_build_only_20260313`
- command shape:
  - `FRAGILEC_MODE=strict ... --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`
- key manifest fields:
  - `lanes=fragilec`
  - `build_only=true`
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
  - `lane_fragilec_test_rpc_status=-1`
  - `no_regression_verdict=not_executed`

This closes `2.6.a` and provides deterministic evidence for the next blocker-fix leaf.

## Validation

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/python/test_mako_rpcbench_harness.py -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/python -p 'test_*.py' -v`
- `cargo test --workspace` (known pre-existing baseline red cluster remains in `fragile-clang`)
- `FRAGILE_ENABLE_DEGRADED_FALLBACK=1 cargo test --workspace` (known pre-existing baseline red cluster remains)
