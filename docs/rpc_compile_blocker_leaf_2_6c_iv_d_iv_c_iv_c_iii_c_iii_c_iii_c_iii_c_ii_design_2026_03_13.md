# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.ii Design Note (2026-03-13)

## Scope

This leaf is a deterministic gate replay after optimization leaf `...c.iii.c.iii.c.iii.c.iii.c.i`.
No parser/codegen/runtime logic changes are introduced.

## Goal

Re-run strict single-lane fragilec build-only replay and enforce blocker non-increase versus the `2.6.c.iii` baseline inventory manifest.

## Why this is the correct next step

- The paired `...c.ii` leaf is required immediately after `...c.i` optimization leaves.
- It verifies that any optimization did not worsen blocker class severity or `E0425` counts.
- It preserves the iteration loop contract in TODO (`optimize -> gate replay -> repeat`).

## Commands Used

```bash
cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_ii_build_only_20260313_v2 \
  --lanes fragilec \
  --build-only \
  --jobs 4 \
  --build-timeout-seconds 180

python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_ii_build_only_20260313_v2 \
  --lanes fragilec \
  --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt \
  --enforce-nonincreasing
```

## Results

- Build-only manifest remains timeout-bound:
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
- Non-increase gate passes:
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `nonincrease_gate_pass=true`

## Operator Notes

- The harness may exit nonzero in build-timeout cases; this is expected for blocker iterations.
- Gate success is determined by the inventory manifest pass keys, not by build success.
- Continue with the next optimization leaf (`...c.iii`) because blocker class remains unchanged and non-worsening is confirmed.
