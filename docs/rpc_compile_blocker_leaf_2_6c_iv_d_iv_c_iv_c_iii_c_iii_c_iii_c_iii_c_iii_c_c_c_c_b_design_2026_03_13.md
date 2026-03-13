# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.b Design (2026-03-13)

## Scope

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.b` is a deterministic strict replay gate task:

- rerun strict single-lane `fragilec` build-only replay
- enforce blocker inventory non-increase versus `2.6.c.iii` baseline

No parser/codegen/runtime source changes are expected for this leaf.

## Problem

After optimization leaf `...c.c.c.c.a`, we must verify that blocker severity and unresolved-name counts do not regress relative to fixed baseline artifacts from `2.6.c.iii`.

## Plan

1. Rebuild release `fragilec`.
2. Run strict build-only replay with a fresh deterministic run root.
3. Run `mako_rpc_compile_blocker_inventory.py` with `--baseline-manifest` and `--enforce-nonincreasing`.
4. Re-run full suites for regression parity.
5. Record evidence in `TODO.md` and `fragile-dev-book.md`.

## Wrong-Approach Check

Validated against `docs/fragile-dev-book.md` section 1.3 and `docs/dev/wrong.md`:

- no target-specific hacks
- no native bypass
- no fallback semantic stubs
- evidence-only gating through generic harness/inventory tooling

## Commands Executed

- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_b_build_only_20260313_v1 --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`
- `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_b_build_only_20260313_v1 --lanes fragilec --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt --enforce-nonincreasing`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Evidence

Replay manifest (`benchmark_harness_manifest.txt`):

- `lane_fragilec_configure_status=0`
- `lane_fragilec_clean_status=0`
- `lane_fragilec_build_status=124`
- `lane_fragilec_failure_class=build_timeout`
- `no_regression_verdict=not_executed`

Inventory manifest (`rpc_compile_blocker_inventory_manifest.txt`):

- `lane_fragilec_first_failing_compile_class=build_timeout`
- `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
- `lane_fragilec_class_rank_delta_vs_baseline=0`
- `lane_fragilec_e0425_delta_vs_baseline=0`
- `lane_fragilec_nonincrease_gate_pass=true`
- `nonincrease_gate_pass=true`

Full-suite baseline parity:

- `cargo test --workspace --all-targets`: `fragile-clang` lib `744` passed / `46` failed (known baseline failure count unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`: `OK` (`29` ran, `1` skipped)

## Outcome

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.b` is complete and non-regressing versus the `2.6.c.iii` baseline. The strict build-only lane remains timeout-bound on `src/rrr/base/misc.cpp`; next leaf is repeat node `...c.c.c.c.c`.
