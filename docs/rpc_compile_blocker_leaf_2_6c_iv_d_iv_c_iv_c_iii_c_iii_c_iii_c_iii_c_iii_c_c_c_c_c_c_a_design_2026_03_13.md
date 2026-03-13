# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.c.c.a Design (2026-03-13)

## Scope

Optimize the hot path inside `infer_fn_template_type_args` without changing
inference semantics, then validate strict replay profile checkpoints and
regression-suite parity.

## Design Rationale

- Previous logic rescanned every function parameter and return type for each
  template parameter, repeatedly calling
  `cpp_type_contains_template_param(...)`.
- This leaf precomputes two immutable helper structures once per inference
  call:
  - `template_param_param_positions: Vec<Vec<usize>>`
  - `template_param_appears_in_return: Vec<bool>`
- The inference loop then consumes these precomputed results directly.
- This keeps behavior unchanged while reducing repeated scans and temporary
  work in a timeout-sensitive region.

## Correctness Constraints

- Do not introduce target-specific behavior for `test_rpc`/`rpcbench`.
- Do not add fallback/synthetic semantic stubs.
- Preserve existing inference priority order:
  - parameter matches first
  - return-type inference fallback when no parameter match
  - existing NTTP array-ref handling unchanged

## User Manual

1. Run focused inference regressions:
   - `cargo test -p fragile-clang test_function_template_type_arg_inference_tracks_multiple_template_param_positions -- --nocapture`
   - `cargo test -p fragile-clang test_function_template_type_arg_inference_uses_return_type_when_params_do_not_reference_template -- --nocapture`
2. Build replay driver:
   - `cargo build --release -p fragile-cli --bin fragilec`
3. Capture strict replay artifacts:
   - 120s profile/timing:
     - `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_a_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_a_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
   - 300s profile/timing:
     - `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_a_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_a_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`
4. Validate full-suite parity:
   - `cargo test --workspace --all-targets`
   - `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Expected Evidence Markers

- 120s profile:
  - `status=codegen_after_template_collection`
- 300s profile:
  - `status=codegen_after_template_instantiation_generation`
  - `input_bytes=575125`
- Replay manifest:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
