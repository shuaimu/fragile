# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.b Design (2026-03-13)

## Scope

Leaf `2.6.c.iv.d.iv.c.iv.b` is a deterministic replay/inventory gate following
optimization leaf `c.iv.a`.

Estimated implementation size is very small (<500 LOC; docs/evidence updates
only), so no further decomposition is needed for this leaf.

## Goal

Confirm no blocker-severity or unresolved-name regression versus the
`2.6.c.iii` baseline manifest after the `c.iv.a` codegen optimization.

## Plan

1. Rebuild release `fragilec`.
2. Run strict single-lane build-only replay with a fresh run root.
3. Run blocker inventory with `--enforce-nonincreasing` against baseline.
4. Run full regression suites and require baseline parity.
5. Record evidence in TODO/dev-book.

## Wrong-Approach Guardrails

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-specific hacks
- no force-native source bypass
- no fake semantic stubs to force success

## Commands

- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_b_build_only_20260313 --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`
- `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_b_build_only_20260313 --lanes fragilec --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt --enforce-nonincreasing`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Results

Replay manifest (`benchmark_harness_manifest.txt`):

- `lane_fragilec_configure_status=0`
- `lane_fragilec_clean_status=0`
- `lane_fragilec_build_status=124`
- `lane_fragilec_failure_class=build_timeout`

Inventory manifest (`rpc_compile_blocker_inventory_manifest.txt`):

- `lane_fragilec_first_failing_compile_class=build_timeout`
- `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
- `lane_fragilec_class_rank_delta_vs_baseline=0`
- `lane_fragilec_e0425_delta_vs_baseline=0`
- `lane_fragilec_nonincrease_gate_pass=true`
- `nonincrease_gate_pass=true`

Suite status:

- workspace cargo baseline unchanged (`fragile-clang` lib `728` passed / `46`
  failed)
- Python suite passes (`29`, skipped `1`)

## Conclusion

Leaf `2.6.c.iv.d.iv.c.iv.b` is complete. No blocker regression was introduced
relative to the `2.6.c.iii` baseline.
