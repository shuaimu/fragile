# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.a Design Note (2026-03-13)

## Scope

- TODO leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.a`
- Objective: implement the next generic pre-top-level codegen hot-path optimization and lock behavior with focused regressions.
- Estimated implementation size: `< 500 LOC`.

## Plan

1. Inspect current strict replay checkpoints/artifacts and target the hottest pre-top-level path.
2. Reduce clone-heavy candidate processing in `collect_fn_template_instantiation` without changing matching semantics.
3. Add a focused regression to lock unresolved function-template slot concretization behavior.
4. Capture deterministic strict replay profiling/timing artifacts at 120s and 300s.
5. Re-run full regression suites and require baseline parity.

## Wrong-Approach Guardrail Check

Checked against `docs/fragile-dev-book.md` section 1.3 and `docs/dev/wrong.md`:

- no target-specific (`rpcbench`/`test_rpc`) code paths
- no force-native bypass
- no fake semantic stubs
- generic codegen optimization only

## Implementation Summary

- Updated `crates/fragile-clang/src/ast_codegen.rs`:
  - `collect_fn_template_instantiation` now tracks only candidate metadata (`mangled_name`, template key, inferred type args) and defers payload concretization until final selected/fallback resolution.
  - Added helper `build_concrete_fn_template_info` to materialize unresolved parameter/return slots once.
  - Removed per-candidate eager `FnTemplateInfo` clone churn in the candidate scan loop.
- Added focused regression:
  - `test_build_concrete_fn_template_info_rewrites_unresolved_param_and_return_slots`

## Deterministic Commands

- `cargo test -p fragile-clang test_build_concrete_fn_template_info_rewrites_unresolved_param_and_return_slots -- --nocapture`
- `cargo test -p fragile-clang test_generate_fn_template_instantiations_consumes_pending_map_and_generates_functions -- --nocapture`
- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_a_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_a_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_a_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_a_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Result Summary

- 120s replay profile:
  - `status=codegen_after_template_collection`
- 300s replay profile:
  - `status=codegen_after_template_instantiation_generation`
  - `input_bytes=573413`
- Comparison vs previous leaf (`...c.iii.c.iii.c.iii.c.i` `input_bytes=575929`): `-2516` bytes.
- Replay manifest remains timeout-bound:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
- Full-suite parity:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib `738` passed / `46` failed (unchanged failure count)
  - Python suite: `OK`, `29` ran, `1` skipped

## Conclusion

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.a` is complete. The optimization reduces function-template candidate scan clone overhead while preserving behavior and improves 300s checkpoint byte volume without altering blocker class.
