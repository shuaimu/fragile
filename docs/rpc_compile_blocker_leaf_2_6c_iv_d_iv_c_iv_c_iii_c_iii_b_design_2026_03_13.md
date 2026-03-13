# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.b Design (2026-03-13)

## Scope and sizing

Leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.b`

This is a replay/inventory verification leaf with no parser/codegen source
changes expected and therefore well below the requested LOC threshold.

## Problem

After finishing leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.a`, we must verify that
strict build-only replay and blocker inventory metrics remain non-regressive
versus the established `2.6.c.iii` baseline manifest.

## Wrong-approach check

Validated against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-name-specific hacks
- no native-source bypass / force-native fallback
- no fake semantic fallback bodies
- deterministic evidence-only validation

## Implementation

No code changes were required for this leaf.

Actions performed:

- rebuilt release `fragilec`
- ran strict fragilec single-lane build-only replay to a fresh run root
- enforced inventory non-increase against baseline manifest
- reran full test suites to confirm baseline parity

## Validation

Executed commands:

- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_b_build_only_20260313 --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`
- `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_b_build_only_20260313 --lanes fragilec --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt --enforce-nonincreasing`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Evidence highlights:

- replay status (`benchmark_harness_manifest.txt`):
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
- non-increase gate (`rpc_compile_blocker_inventory_manifest.txt`):
  - `lane_fragilec_first_failing_compile_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `lane_fragilec_nonincrease_gate_pass=true`
  - `nonincrease_gate_pass=true`
- full suite parity:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib `734 passed / 46 failed` (unchanged known baseline)
  - Python suite: `Ran 29 tests`, `OK (skipped=1)`

## Outcome

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.b` is complete. Blocker class/E0425
metrics remain non-worsening versus baseline and strict replay remains
timeout-bound on `src/rrr/base/misc.cpp`.
