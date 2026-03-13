# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.i Design Note (2026-03-13)

## Scope

- TODO leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.i`
- Objective: implement the next generic codegen hot-path optimization in the pre-top-level window and lock behavior with focused regression coverage.
- Estimated implementation size: `< 500 LOC`.

## Breakdown Update

The open repeat node `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c` was decomposed into bounded leaves:

1. `...c.i` optimize hot path (this leaf)
2. `...c.ii` strict replay + non-increase gate
3. `...c.iii` repeat loop until build-status objective is met

## Plan

1. Precompute instantiated call-signature normalization once per call-site in function-template candidate matching.
2. Reuse precomputed normalized lanes across candidate checks.
3. Preserve reference-prefix compatibility semantics.
4. Add focused regression for normalization behavior.
5. Capture strict replay profile/timing artifacts (120s/300s).
6. Re-run full suites and require baseline parity.

## Wrong-Approach Guardrail Check

Checked against `docs/fragile-dev-book.md` section 1.3 and `docs/dev/wrong.md`:

- no RPC-target-specific branches
- no force-native bypass
- no fake semantic stubs
- generic optimization only

## Implementation Summary

- Updated `crates/fragile-clang/src/ast_codegen.rs`:
  - Added `normalize_template_match_type` and `strip_template_match_ref_prefix`.
  - In `collect_fn_template_instantiation`, precompute normalized instantiated parameter/return lanes once and reuse in candidate comparisons.
  - Removed repeated per-candidate normalization/allocation of identical instantiated signature data.
- Added focused regression:
  - `test_template_match_type_normalization_preserves_ref_prefix_compatibility`

## Deterministic Commands

- `cargo test -p fragile-clang test_template_match_type_normalization_preserves_ref_prefix_compatibility -- --nocapture`
- `cargo test -p fragile-clang test_build_concrete_fn_template_info_rewrites_unresolved_param_and_return_slots -- --nocapture`
- `cargo test -p fragile-clang test_generate_fn_template_instantiations_consumes_pending_map_and_generates_functions -- --nocapture`
- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_i_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_i_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_i_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_i_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Result Summary

- 120s profile:
  - `status=codegen_after_template_collection`
- 300s profile:
  - `status=codegen_after_template_instantiation_generation`
  - `input_bytes=573750`
- Comparison vs previous leaf (`...c.iii.c.iii.c.iii.c.iii.a` `input_bytes=573413`): `+337` bytes.
- Replay manifest remains timeout-bound:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
- Full-suite parity:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib `739` passed / `46` failed (unchanged failure count)
  - Python suite: `OK`, `29` ran, `1` skipped

## Conclusion

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.i` is complete. The optimization reuses instantiated-signature normalization in function-template candidate matching while preserving semantics and baseline regression behavior; strict replay remains timeout-bound on `src/rrr/base/misc.cpp`.
