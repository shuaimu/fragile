# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.c.c.c.b Design (2026-03-13)

## Scope

Run the strict build-only replay/inventory non-increase gate for the current
repeat cycle and verify class/E0425 non-worsening against the fixed
`2.6.c.iii` baseline.

## Design Rationale

- This leaf is a validation gate paired with optimization leaf
  `...c.c.c.c.c.c.c.a`.
- It confirms no blocker-class regression while preserving deterministic
  replay artifacts.
- It does not introduce parser/codegen/runtime behavior changes.

## Correctness Constraints

- No target-specific behavior for `test_rpc`/`rpcbench`.
- No force-native bypass.
- No synthetic semantic stubs.
- Require non-increase gate pass against baseline inventory manifest.

## User Manual

1. Build release driver:
   - `cargo build --release -p fragile-cli --bin fragilec`
2. Run strict build-only harness:
   - `FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_b_build_only_20260313_v1 --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`
3. Enforce non-increase gate:
   - `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_b_build_only_20260313_v1 --lanes fragilec --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt --enforce-nonincreasing`
4. Run full suites:
   - `cargo test --workspace --all-targets`
   - `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Expected Evidence Markers

- `benchmark_harness_manifest.txt`:
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
- `rpc_compile_blocker_inventory_manifest.txt`:
  - `lane_fragilec_first_failing_compile_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `lane_fragilec_nonincrease_gate_pass=true`
  - `nonincrease_gate_pass=true`
