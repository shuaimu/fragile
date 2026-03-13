# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.b Design Notes (2026-03-13)

## Scope

Leaf `2.6.c.iv.d.iv.b` requires a post-`2.6.c.iv.d.iv.a` strict replay and
inventory gate verification only:

- rerun strict single-lane `fragilec` build-only harness replay
- enforce blocker inventory non-increase against `2.6.c.iii` baseline
- confirm non-worsening blocker class rank and `E0425` deltas

## LOC Budget Analysis

This leaf is operational validation and does not require parser/codegen/runtime
changes. Expected implementation LOC is approximately `0` in production code
and tests.

## Execution Plan

1. Run strict build-only harness replay with deterministic run root.
2. Run blocker inventory non-increase gate against the `2.6.c.iii` baseline
   manifest.
3. Record manifest evidence for configure/clean/build status, blocker class,
   first failing file, and delta fields.
4. Re-run full regression suites and confirm no regression versus known
   baseline.

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` Section `1.3` and `docs/dev/wrong.md`:

- no target-name-specific code path
- no force-native bypass
- no synthesized semantic fallback body injection
- deterministic evidence capture only

## Deterministic Evidence

Commands executed:

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_b_build_only_20260313 \
  --lanes fragilec \
  --build-only \
  --jobs 4 \
  --build-timeout-seconds 180
```

```bash
python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_b_build_only_20260313 \
  --lanes fragilec \
  --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt \
  --enforce-nonincreasing
```

Manifest highlights:

- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_b_build_only_20260313/benchmark_harness_manifest.txt`
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_b_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt`
  - `lane_fragilec_first_failing_compile_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `lane_fragilec_nonincrease_gate_pass=true`
  - `nonincrease_gate_pass=true`

## Regression Suite Results

- `cargo test --workspace --all-targets`
  - `fragile-clang` lib: `726 passed`, `46 failed` (known baseline, unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
  - `Ran 29 tests`, `OK (skipped=1)`

## Outcome

Leaf `2.6.c.iv.d.iv.b` is satisfied: post-`iv.a` strict replay remains
timeout-bound on `src/rrr/base/misc.cpp`, and inventory non-increase gate
passes with no blocker-class or `E0425` regression versus `2.6.c.iii`.
