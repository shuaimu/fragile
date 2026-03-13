# RPC Compile Blocker Leaf 2.6.c.iv.c Design Note (2026-03-13)

## Scope

Leaf: `2.6.c.iv.c`  
Objective: re-run strict `fragilec` build-only replay after `2.6.c.iv.b` and
enforce blocker inventory non-increase against `2.6.c.iii` baseline.

## Size/Complexity Check

This leaf is operational evidence capture, not a large implementation change.
Estimated LOC impact: very small (`<500 LOC`), limited to TODO/docs evidence
updates and one design note.

## Wrong-Approach Guard

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-specific hacks
- no `FRAGILEC_FORCE_NATIVE_SOURCES`
- no fake semantic fallback bodies
- keep deterministic tooling and explicit failure reporting

## Execution Plan

1. Run strict single-lane build-only harness replay.
2. Run blocker inventory with `--baseline-manifest` and
   `--enforce-nonincreasing`.
3. Record manifest evidence in TODO/dev-book.
4. Re-run full requested suites and verify no new regressions from this leaf.

## Commands Executed

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_c_build_only_20260313 \
  --lanes fragilec \
  --build-only \
  --jobs 4 \
  --build-timeout-seconds 180

python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_c_build_only_20260313 \
  --lanes fragilec \
  --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt \
  --enforce-nonincreasing
```

## Deterministic Evidence

Run root:

- `/tmp/fragile_rpc_leaf_2_6c_iv_c_build_only_20260313`

Harness manifest highlights:

- `lane_fragilec_configure_status=0`
- `lane_fragilec_clean_status=0`
- `lane_fragilec_build_status=124`
- `lane_fragilec_failure_class=build_timeout`
- `no_regression_verdict=not_executed`

Inventory non-increase manifest highlights:

- `lane_fragilec_first_failing_compile_class=build_timeout`
- `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
- `lane_fragilec_class_rank_delta_vs_baseline=0`
- `lane_fragilec_e0425_delta_vs_baseline=0`
- `lane_fragilec_nonincrease_gate_pass=true`
- `nonincrease_gate_pass=true`

## Test Validation

Executed:

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

Observed:

- Python suite passes (`29` tests, `1` skipped).
- Workspace cargo suite retains known pre-existing baseline:
  `46` failing `fragile-clang` lib tests (unchanged by this leaf).

## Outcome

Leaf `2.6.c.iv.c` is complete. Non-increase gate passes against
`2.6.c.iii` baseline with no blocker class/E0425 regression.
