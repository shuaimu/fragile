# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.c.c.b Design (2026-03-13)

## Scope

Execute the paired strict build-only replay non-increase gate after leaf
`...c.c.c.c.c.c.a`, verify blocker severity/E0425 deltas remain non-worsening
against the fixed `2.6.c.iii` baseline, and record deterministic evidence.

## Design Rationale

- This leaf is a gate/verification step, not a code-edit step.
- The correct approach is to rerun strict harness + blocker inventory with
  `--enforce-nonincreasing` and compare against the stable baseline manifest.
- Full-suite reruns are still required to ensure no hidden regressions were
  introduced by prior leaf changes.

## Correctness Constraints

- No target-specific bypasses for RPC tasks.
- No synthetic fallback behavior in parser/codegen/runtime.
- Keep gate criteria deterministic:
  - class rank delta non-positive
  - E0425 delta non-positive
  - executable comparison parity retained

## User Manual

1. Build strict replay driver:
   - `cargo build --release -p fragile-cli --bin fragilec`
2. Run strict single-lane build-only replay:
   - `FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_b_build_only_20260313_v1 --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 180`
3. Enforce blocker non-increase gate:
   - `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_b_build_only_20260313_v1 --lanes fragilec --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt --enforce-nonincreasing`
4. Re-run full suites:
   - `cargo test --workspace --all-targets`
   - `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Expected Evidence Markers

- `benchmark_harness_manifest.txt`:
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
- `rpc_compile_blocker_inventory_manifest.txt`:
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `lane_fragilec_nonincrease_gate_pass=true`
  - `nonincrease_gate_pass=true`
- Full suite parity snapshot:
  - `fragile-clang` lib failure count unchanged (`46`)
  - Python suite `OK (skipped=1)`
