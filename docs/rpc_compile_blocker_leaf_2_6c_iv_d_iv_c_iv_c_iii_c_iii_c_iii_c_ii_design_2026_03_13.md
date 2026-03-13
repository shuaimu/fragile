# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.ii Design Note (2026-03-13)

## Scope

- TODO leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.ii`
- Objective: rerun strict single-lane `fragilec` build-only replay and enforce blocker non-increase versus `2.6.c.iii` baseline.
- Estimated implementation size: `< 500 LOC` (verification-only leaf; no source-code logic changes required).

## Plan

1. Rebuild release `fragilec` binary to keep replay inputs deterministic.
2. Run strict build-only harness lane with a fresh run root.
3. Run blocker inventory with `--enforce-nonincreasing` against the `2.6.c.iii` baseline manifest.
4. Run full regression suites and require known-baseline parity.
5. Record deterministic evidence in `TODO.md` and `docs/fragile-dev-book.md`.

## Wrong-Approach Guardrail Check

Checked against `docs/fragile-dev-book.md` section 1.3 and `docs/dev/wrong.md`:

- no target-specific parser/codegen branching
- no force-native compile bypass
- no semantic fallback stubs
- only deterministic replay/gating work for this leaf

## Deterministic Commands

- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_ii_build_only_20260313 --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`
- `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_ii_build_only_20260313 --lanes fragilec --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt --enforce-nonincreasing`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Result Summary

- Replay manifest reports:
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
  - `no_regression_verdict=not_executed`
- Inventory manifest reports:
  - `lane_fragilec_first_failing_compile_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `lane_fragilec_nonincrease_gate_pass=true`
  - `nonincrease_gate_pass=true`
- Full-suite parity:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib `737` passed / `46` failed (unchanged baseline)
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`: `OK`, `29` ran, `1` skipped

## Conclusion

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.ii` is complete and confirms no blocker-severity/E0425 regression versus baseline while strict replay remains timeout-bound on `src/rrr/base/misc.cpp`.
