# RPC Compile Blocker Leaf 2.6.b.ii Design (2026-03-13)

## Objective

Leaf `2.6.b.ii` requires a focused replay helper flow that can take timeout-derived compile
units from `2.6.b.i` and produce deterministic non-timeout first-blocker diagnostics when
possible.

## Scope Sizing

Implementation was small (<500 LOC):

- script update: `scripts/mako_rpc_compile_blocker_replay.py`
- focused tests: `tests/python/test_mako_rpc_compile_blocker_replay.py`
- TODO/docs updates

No further decomposition was needed for this leaf.

## Decision

Extend replay helper resolution and classification generically:

- include timeout family in replay priority/classification (`build_timeout`)
- support deterministic compile-db matching for timeout-derived relative blocker files
  (for example `src/rrr/base/misc.cpp`) via:
  - absolute candidate matching from harness roots
  - stable suffix matching fallback for absolute compile-db file paths
- resolve fallback replay source paths against harness roots (`workspace_root`, `mako_root`)
  instead of assuming workspace-relative paths only

This enables timeout-derived replay candidates to hit their real compile commands and emit
first-failure diagnostics without target-specific shortcuts.

## Wrong-Approach Check

Aligned with project constraints and `docs/dev/wrong.md`:

- no target-name transpiler/codegen branching
- no fake semantic method bodies or forced compile-success behavior
- no force-native bypass
- replay outcomes are produced from real command execution only

## Implementation

Updated `scripts/mako_rpc_compile_blocker_replay.py`:

- added `build_timeout` to blocker priority ordering
- added timeout-aware `first_failure_class` classification
- added candidate source-path derivation from harness roots
- added deterministic compile-db match ranking:
  - exact candidate path match first
  - suffix match fallback for relative timeout blocker files
  - stable sort tie-breakers
- updated fallback replay path resolution to use resolved source paths

Updated `tests/python/test_mako_rpc_compile_blocker_replay.py`:

- `test_replay_timeout_derived_relative_blocker_uses_compile_db_suffix_match`
- `test_replay_timeout_derived_relative_blocker_fallback_prefers_mako_source`

## Deterministic Evidence

Fixture run root proving non-timeout blocker capture from timeout-derived compile unit:

- run root: `/tmp/fragile_rpc_leaf_2_6b_ii_fixture_20260313/run`
- command:
  - `python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6b_ii_fixture_20260313/run --lanes fragilec --max-replays 1 --timeout-seconds 60`
- manifest highlights:
  - `replay_01_blocker_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
  - `replay_01_command_source=compile_commands`
  - `replay_01_status=1`
  - `replay_01_timed_out=false`
  - `replay_01_first_failure_class=unresolved_name_or_type_e0425`

Real strict run-root replay still confirms deterministic timeout-derived command selection,
but remains timeout-bound for this TU under current environment:

- run root: `/tmp/fragile_rpc_leaf_2_6a_build_only_20260313`
- `replay_01_command_source=compile_commands`
- `replay_01_blocker_file=src/rrr/base/misc.cpp`

## Validation

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/python/test_mako_rpc_compile_blocker_replay.py -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/python -p 'test_*.py' -v`
- `cargo test --workspace` (known pre-existing baseline red cluster remains in `fragile-clang`)
- `FRAGILE_ENABLE_DEGRADED_FALLBACK=1 cargo test --workspace` (known pre-existing baseline red cluster remains)
