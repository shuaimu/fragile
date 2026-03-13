# RPC Compile Closure Leaf 2.6.c.i Design (2026-03-13)

## Objective

Leaf `2.6.c.i` captures a fresh strict single-lane `fragilec` build-only replay baseline after
`2.6.b.iii`, together with deterministic blocker inventory and top replay artifacts for the next
fix loop.

## Scope Sizing and Decomposition

Task `2.6.c` is too broad for a single <500 LOC implementation because it can require multiple
compile-blocker fix/replay loops before strict build success (`status=0`).

`TODO.md` was decomposed into small leaves:

- `2.6.c.i` capture fresh strict build-only + blocker triage baseline
- `2.6.c.ii` implement generic fix for first blocker class from `2.6.c.i`
- `2.6.c.iii` rerun strict build-only and enforce non-increase against `2.6.c.i`
- `2.6.c.iv` iterate `c.ii`/`c.iii` until strict build-only lane reaches `status=0`

This keeps each execution leaf bounded and auditable.

## Wrong-Approach Check

Checked against project constraints and `docs/dev/wrong.md`:

- no RPC-target name conditionals
- no force-native bypass
- no synthetic fallback method bodies
- no fake success signaling

This leaf is artifact capture only; no semantic compile behavior was masked.

## Execution

Fresh strict build-only run:

- command:
  - `FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`
- key manifest results (`benchmark_harness_manifest.txt`):
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`

Inventory capture:

- command:
  - `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec`
- key manifest results (`rpc_compile_blocker_inventory_manifest.txt`):
  - `lane_fragilec_first_failing_compile_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`

Top replay capture:

- command:
  - `python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
- key manifest results (`rpc_compile_blocker_replay_manifest.txt`):
  - `replay_01_blocker_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`

## Next Leaf

Proceed to `2.6.c.ii`: implement a generic fix for the blocker class captured from this baseline
using focused regressions.
