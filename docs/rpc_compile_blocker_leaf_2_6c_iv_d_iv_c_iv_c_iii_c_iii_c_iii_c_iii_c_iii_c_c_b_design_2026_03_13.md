# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.b Design Note (2026-03-13)

## Scope

This leaf is a deterministic strict build-only replay and blocker inventory
non-increase gate rerun after optimization leaf
`2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.a`.

No parser/codegen/runtime behavior changes are introduced.

## Problem

The paired gate must verify that the latest optimization did not worsen blocker
class severity or `E0425` counts versus the fixed `2.6.c.iii` baseline
inventory manifest.

## Plan

1. Rebuild release `fragilec`.
2. Run strict single-lane (`fragilec`) build-only harness replay.
3. Run blocker inventory with baseline comparison and enforced non-increase.
4. Re-run full regression suites and confirm baseline parity.

## Wrong-Approach Check

Conforms to Section 1.3 and `docs/dev/wrong.md`:

- no RPC-target-specific conditionals
- no force-native bypasses
- no fake semantic stubs
- generic tooling-level replay and inventory gating only

## Validation Commands

```bash
cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict \
python3 scripts/mako_rpcbench_harness.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_b_build_only_20260313_v1 \
  --lanes fragilec \
  --build-only \
  --jobs 4 \
  --build-timeout-seconds 180

python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_b_build_only_20260313_v1 \
  --lanes fragilec \
  --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt \
  --enforce-nonincreasing

cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Results

- replay status (`benchmark_harness_manifest.txt`):
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
  - `no_regression_verdict=not_executed`
- inventory gate (`rpc_compile_blocker_inventory_manifest.txt`):
  - `lane_fragilec_first_failing_compile_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `lane_fragilec_nonincrease_gate_pass=true`
  - `nonincrease_gate_pass=true`
- full suite parity retained:
  - `cargo test --workspace --all-targets`:
    `fragile-clang` lib `742` passed / `46` failed (failure count unchanged)
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`:
    `OK (29 ran, 1 skipped)`

## Next Leaf

Proceed to repeat leaf
`2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c`.
