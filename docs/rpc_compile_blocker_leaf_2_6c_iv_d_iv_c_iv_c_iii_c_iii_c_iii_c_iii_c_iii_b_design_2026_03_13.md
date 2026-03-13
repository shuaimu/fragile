# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.b Design Note (2026-03-13)

## Scope

This leaf is a deterministic gate replay pass:

- strict single-lane `fragilec` build-only harness replay
- blocker inventory non-increase enforcement versus the fixed `2.6.c.iii` baseline

No parser/codegen/runtime source behavior is changed in this leaf.

## Problem

After optimization leaf `...c.iii.a`, the paired gate leaf `...c.iii.b` must prove:

- no blocker class-rank regression
- no `E0425` count regression

relative to baseline manifest
`/tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt`.

## Plan

1. Rebuild `fragilec` release binary.
2. Run strict `--build-only` replay with deterministic run-root artifacts.
3. Run inventory script with `--baseline-manifest` + `--enforce-nonincreasing`.
4. Re-run full regression suites and record baseline parity.

## Wrong-Approach Check

Validated against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no RPC-target-specific hacks
- no force-native bypasses
- no fake semantic stubs
- generic tooling-only replay and manifest gating

## Commands

```bash
cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict \
python3 scripts/mako_rpcbench_harness.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_b_build_only_20260313_v1 \
  --lanes fragilec \
  --build-only \
  --jobs 4 \
  --build-timeout-seconds 180

python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_b_build_only_20260313_v1 \
  --lanes fragilec \
  --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt \
  --enforce-nonincreasing

cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Results

Run-root:
`/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_b_build_only_20260313_v1`

Harness manifest highlights:

- `lane_fragilec_configure_status=0`
- `lane_fragilec_clean_status=0`
- `lane_fragilec_build_status=124`
- `lane_fragilec_failure_class=build_timeout`
- `no_regression_verdict=not_executed`

Inventory manifest highlights:

- `lane_fragilec_first_failing_compile_class=build_timeout`
- `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
- `lane_fragilec_class_rank_delta_vs_baseline=0`
- `lane_fragilec_e0425_delta_vs_baseline=0`
- `lane_fragilec_nonincrease_gate_pass=true`
- `nonincrease_gate_pass=true`

Full-suite parity:

- `cargo test --workspace --all-targets`: `fragile-clang` lib `740` passed / `46` failed (unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`: `OK (29 ran, 1 skipped)`

## Next Leaf

Proceed to repeat leaf `...c.iii.c` (next optimization + paired gate loop).
