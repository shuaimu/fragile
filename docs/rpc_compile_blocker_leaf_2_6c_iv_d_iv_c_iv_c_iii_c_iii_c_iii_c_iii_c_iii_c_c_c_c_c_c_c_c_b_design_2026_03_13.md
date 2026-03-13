# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.c.c.c.c.b Design (2026-03-13)

## Scope

Run the strict build-only replay plus blocker inventory non-increase gate after
leaf `...c.c.c.c.c.c.c.c.c.a`, and verify baseline parity against `2.6.c.iii`.

## Design Rationale

- This is a gate leaf, not a code-change leaf.
- The objective is to prove that the prior optimization did not worsen blocker
  class severity or unresolved-name (`E0425`) counts versus the fixed baseline.
- The comparison is anchored to:
  - `/tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt`
- Success criteria:
  - strict build-only replay artifacts captured
  - inventory non-increase gate passes (`nonincrease_gate_pass=true`)
  - full Rust/Python regression suites remain at known baseline outcomes

## Correctness Constraints

- No target-specific behavior for `test_rpc` / `rpcbench`.
- No parser/codegen semantic fallback additions.
- No force-native bypass.
- Preserve deterministic replay/inventory workflow and evidence capture.

## User Manual

1. Build strict replay driver:
   - `cargo build --release -p fragile-cli --bin fragilec`
2. Run strict single-lane build-only replay:
   - `FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_b_build_only_20260313_v1 --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`
3. Enforce blocker inventory non-increase:
   - `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_b_build_only_20260313_v1 --lanes fragilec --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt --enforce-nonincreasing`
4. Verify full-suite baseline parity:
   - `cargo test --workspace --all-targets`
   - `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Expected Evidence Markers

- replay manifest (`benchmark_harness_manifest.txt`):
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
- inventory manifest (`rpc_compile_blocker_inventory_manifest.txt`):
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `lane_fragilec_nonincrease_gate_pass=true`
  - `nonincrease_gate_pass=true`
