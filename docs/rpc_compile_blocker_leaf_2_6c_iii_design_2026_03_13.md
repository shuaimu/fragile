# RPC Compile Blocker Leaf 2.6.c.iii Design (2026-03-13)

## Scope

Leaf `2.6.c.iii`: rerun strict build-only replay and enforce blocker inventory
non-increase against `2.6.c.i` baseline.

## Analysis

This leaf is bounded and low-LOC:

- no parser/codegen implementation changes required
- deterministic harness/inventory execution and artifact verification only
- documentation and TODO evidence updates only

Estimated change size stayed well below the requested threshold.

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-specific compiler/codegen conditionals
- no force-native bypass
- no synthetic/fake semantic method body additions
- failure state remains explicit (`build_timeout`) and is not masked

## Execution

### 1) Strict single-lane build-only replay

Command:

- `FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py --run-root /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313 --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`

Observed deterministic status in `benchmark_harness_manifest.txt`:

- `lane_fragilec_configure_status=0`
- `lane_fragilec_clean_status=0`
- `lane_fragilec_build_status=124`
- `lane_fragilec_failure_class=build_timeout`

### 2) Blocker inventory non-increase gate vs baseline

Command:

- `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313 --lanes fragilec --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt --enforce-nonincreasing`

Observed deterministic gate fields in `rpc_compile_blocker_inventory_manifest.txt`:

- `lane_fragilec_first_failing_compile_class=build_timeout`
- `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
- `lane_fragilec_class_rank_delta_vs_baseline=0`
- `lane_fragilec_e0425_delta_vs_baseline=0`
- `lane_fragilec_nonincrease_gate_pass=true`
- `nonincrease_gate_pass=true`

## Outcome

Leaf `2.6.c.iii` is complete: blocker severity/E0425 counts are non-increasing vs
`2.6.c.i` baseline and the gate passes deterministically.

Next iteration remains `2.6.c.iv` (repeat `2.6.c.ii`-`2.6.c.iii` until build status reaches `0`).
