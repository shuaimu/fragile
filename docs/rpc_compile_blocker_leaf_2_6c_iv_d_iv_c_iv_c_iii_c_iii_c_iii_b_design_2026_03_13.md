# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.b Design (2026-03-13)

## Scope

Selected leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.b`

Task intent: rerun strict single-lane (`fragilec`) build-only replay and enforce
blocker-inventory non-increase versus the fixed `2.6.c.iii` baseline after
optimization leaf `...c.iii.a`.

## Size Analysis

This is an execution-and-validation leaf (well under 500 LOC):

- no parser/codegen source modifications required
- deterministic artifact refresh and non-increase gate verification only

No further decomposition was required.

## Plan

1. Rebuild release `fragilec` from current tree.
2. Execute strict single-lane build-only replay for a fresh `...c.iii.b` run
   root.
3. Enforce blocker inventory non-increase against `2.6.c.iii` baseline
   manifest.
4. Run full regression suites and verify baseline parity.
5. Record deterministic evidence in TODO/dev-book artifacts.

## Wrong-Approach Guard Check

Validated against `docs/fragile-dev-book.md` section `1.3` and
`docs/dev/wrong.md`:

- no target-specific hacks
- no force-native bypass
- no semantic stub/fake-body synthesis
- deterministic replay+gating only

## Commands Executed

```bash
cargo build --release -p fragile-cli --bin fragilec
FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_b_build_only_20260313 \
  --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180
python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_b_build_only_20260313 \
  --lanes fragilec \
  --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt \
  --enforce-nonincreasing
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Evidence Summary

Run root:
`/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_b_build_only_20260313`

Replay manifest highlights (`benchmark_harness_manifest.txt`):

- `lane_fragilec_configure_status=0`
- `lane_fragilec_clean_status=0`
- `lane_fragilec_build_status=124`
- `lane_fragilec_failure_class=build_timeout`
- `no_regression_verdict=not_executed`

Inventory manifest highlights (`rpc_compile_blocker_inventory_manifest.txt`):

- `lane_fragilec_first_failing_compile_class=build_timeout`
- `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
- `lane_fragilec_class_rank_delta_vs_baseline=0`
- `lane_fragilec_e0425_delta_vs_baseline=0`
- `lane_fragilec_nonincrease_gate_pass=true`
- `nonincrease_gate_pass=true`

Regression suites:

- `cargo test --workspace --all-targets`: `fragile-clang` lib `736` passed / `46` failed (known baseline, unchanged failure count)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`: `OK`, `29` ran, `1` skipped
