# RPC Compile Blocker Leaf 2.6.b.i Design (2026-03-13)

## Objective

Leaf `2.6.b` was initially too broad for a single small patch because the current strict replay blocker
was only `build_timeout` with no deterministic compile-unit identity in blocker artifacts.

Leaf `2.6.b.i` narrows scope to deterministic timeout-blocker extraction:

- classify timeout builds as `build_timeout`
- extract the last active compile unit from harness `build.stdout` when rustc/transpile markers are absent

## Scope Sizing

`2.6.b` full closure likely requires parser/codegen/type-lowering fixes across one or more translation units
(estimated >500 LOC risk), so it was decomposed into `2.6.b.i`..`2.6.b.iii`.

`2.6.b.i` implementation is small (<500 LOC):

- script update: `scripts/mako_rpc_compile_blocker_inventory.py`
- focused tests: `tests/python/test_mako_rpc_compile_blocker_inventory.py`
- TODO/docs updates

## Decision

Keep the inventory script as the source of deterministic blocker summaries and extend it to support timeout triage:

- detect timeout from `build.status==124` or timeout marker text
- classify as `build_timeout`
- for timeout-only captures, derive compile unit from `Building CXX object <obj>.o` lines in `build.stdout`

This avoids semantic guesswork and enables deterministic follow-up replay in leaf `2.6.b.ii`.

## Wrong-Approach Check

Aligned with project constraints and `docs/dev/wrong.md`:

- no RPC target-specific transpiler behavior
- no fake semantic method bodies / no forced compile-success paths
- no force-native bypass
- blocker fields are derived from real harness artifacts only

## Implementation

Updated `scripts/mako_rpc_compile_blocker_inventory.py`:

- added timeout classification helper (`build_timeout`)
- added CMake object-line parsing for timeout fallback extraction from `build.stdout`
- preserved rustc/transpile file extraction precedence from `build.stderr`
- kept backward compatibility when `build.stdout` is absent (empty fallback)

Updated `tests/python/test_mako_rpc_compile_blocker_inventory.py`:

- `test_inventory_classifies_build_timeout_and_extracts_active_compile_file`
- test fixture now writes `build.stdout` artifacts

Updated docs:

- `TODO.md` (`2.6.b` decomposition + `2.6.b.i` completion evidence)
- `docs/rpc_compile_blocker_inventory_user_manual.md`

## Deterministic Evidence

Re-ran inventory on strict build-only root:

- run root: `/tmp/fragile_rpc_leaf_2_6a_build_only_20260313`
- command:
  - `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6a_build_only_20260313 --lanes fragilec`
- key manifest fields:
  - `lane_fragilec_first_failing_compile_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
  - `lane_fragilec_first_failing_compile_e0425_count=0`

## Validation

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/python/test_mako_rpc_compile_blocker_inventory.py -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/python -p 'test_*.py' -v`
- `cargo test --workspace` (known pre-existing baseline red cluster remains in `fragile-clang`)
- `FRAGILE_ENABLE_DEGRADED_FALLBACK=1 cargo test --workspace` (known pre-existing baseline red cluster remains)
