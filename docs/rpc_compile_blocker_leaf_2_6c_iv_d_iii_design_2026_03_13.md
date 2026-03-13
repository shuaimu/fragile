# RPC Compile Blocker Leaf 2.6.c.iv.d.iii Design Note (2026-03-13)

## Scope

Leaf: `2.6.c.iv.d.iii`  
Objective: re-run strict `fragilec` build-only replay after `2.6.c.iv.d.ii` and
enforce blocker inventory non-increase versus the `2.6.c.iii` baseline.

## Size/Complexity Check

This leaf is a small validation/evidence leaf (`<500 LOC`) with no parser or
codegen algorithm changes required.

## Wrong-Approach Guard

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-specific hacks
- no force-native fallback
- no synthetic semantic bodies
- pure replay/inventory validation against deterministic artifacts

## Plan

1. Run strict single-lane `fragilec` build-only replay on current HEAD.
2. Run blocker inventory with `--enforce-nonincreasing` against
   `/tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313` baseline.
3. Record class/E0425 delta outcome and lane status.
4. Run full regression suites to confirm no broader regressions.

## Commands Executed

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iii_build_only_20260313 \
  --lanes fragilec \
  --build-only \
  --jobs 4 \
  --build-timeout-seconds 180

python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iii_build_only_20260313 \
  --lanes fragilec \
  --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt \
  --enforce-nonincreasing

cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Deterministic Evidence

Run root:

- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iii_build_only_20260313`

Replay manifest highlights (`benchmark_harness_manifest.txt`):

- `lane_fragilec_configure_status=0`
- `lane_fragilec_clean_status=0`
- `lane_fragilec_build_status=124`
- `lane_fragilec_failure_class=build_timeout`
- `no_regression_verdict=not_executed`

Inventory non-increase gate (`rpc_compile_blocker_inventory_manifest.txt`):

- `lane_fragilec_first_failing_compile_class=build_timeout`
- `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
- `lane_fragilec_class_rank_delta_vs_baseline=0`
- `lane_fragilec_e0425_delta_vs_baseline=0`
- `lane_fragilec_nonincrease_gate_pass=true`
- `nonincrease_gate_pass=true`

Baseline reference:

- `/tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt`

Regression suites:

- `cargo test --workspace --all-targets`: known baseline remains
  (`46` existing `fragile-clang` lib failures, unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`:
  `29` passed, `1` skipped

## Outcome

Leaf `2.6.c.iv.d.iii` is complete. Post-`d.ii` strict replay remains timeout-
bound at `src/rrr/base/misc.cpp` with non-worsening blocker class and E0425
counts versus the `2.6.c.iii` baseline.
