# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.c.c.c.c.a Design (2026-03-13)

## Scope

Reduce template-inference hot-path overhead in NTTP array-reference matching by
avoiding unnecessary type-string materialization when element and pointee types
are already structurally equal.

## Design Rationale

- `infer_non_type_array_ref_template_arg` validates compatibility between
  pattern array element type and instantiated pointer pointee type.
- Before this leaf, it always compared both sides via `to_rust_type_str()`,
  even for structurally identical types.
- This leaf adds a structural fast path (`element == pointee`) and only falls
  back to Rust-surface string comparison when structural equality is false.
- Canonicalized fallback remains necessary for equivalent spellings that are
  not structurally identical (for example `Named("char")` vs
  `Char { signed: true }`).

## Correctness Constraints

- No target-specific behavior for `test_rpc` / `rpcbench`.
- No synthetic fallback bodies or semantic stubs.
- Preserve existing NTTP array-ref inference semantics:
  - reject incompatible element/pointee types
  - accept canonicalized equivalent spellings
  - keep literal-bound extraction behavior unchanged

## User Manual

1. Run focused inference regressions:
   - `cargo test -p fragile-clang test_function_template_type_arg_inference_nttp_array_ref_accepts_canonicalized_element_spelling -- --nocapture`
   - `cargo test -p fragile-clang test_function_template_type_arg_inference_nttp_array_ref_uses_literal_bound -- --nocapture`
2. Build replay driver:
   - `cargo build --release -p fragile-cli --bin fragilec`
3. Capture strict replay artifacts:
   - 120s profile/timing:
     - `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_a_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_a_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
   - 300s profile/timing:
     - `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_a_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_a_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`
4. Validate full-suite parity:
   - `cargo test --workspace --all-targets`
   - `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Expected Evidence Markers

- 120s profile:
  - `status=codegen_after_template_collection`
- 300s profile:
  - `status=codegen_after_template_instantiation_generation`
  - `input_bytes=567527`
- Replay manifest:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
