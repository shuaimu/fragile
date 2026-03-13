# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.c.c.c.c.c.a Design (2026-03-13)

## Scope

Reduce NTTP array-reference type-compatibility overhead by avoiding
canonicalized Rust-surface type-string comparison when structural mismatch
already proves incompatibility and no spelling-sensitive type nodes are present.

## Design Rationale

- `infer_non_type_array_ref_template_arg` previously compared non-equal
  element/pointee types via `to_rust_type_str()` unconditionally.
- Canonicalized string comparison is only needed when named/dependent spellings
  can represent equivalent types (`Named("char")` vs `Char { signed: true }`).
- This leaf adds `cpp_type_has_spelling_sensitive_components` and uses it to
  gate canonicalized comparison.
- For structurally-different non-spelling-sensitive shapes, inference now exits
  early without string allocation.

## Correctness Constraints

- No target-specific behavior for `test_rpc` / `rpcbench`.
- Preserve canonicalized equivalence for named/dependent spellings.
- No semantic fallback stubs.
- Keep literal-bound extraction semantics unchanged.

## User Manual

1. Run focused regressions:
   - `cargo test -p fragile-clang test_function_template_type_arg_inference_nttp_array_ref_accepts_canonicalized_element_spelling -- --nocapture`
   - `cargo test -p fragile-clang test_function_template_type_arg_inference_nttp_array_ref_accepts_canonicalized_nested_pointer_element_spelling -- --nocapture`
   - `cargo test -p fragile-clang test_function_template_type_arg_inference_nttp_array_ref_uses_literal_bound -- --nocapture`
2. Build replay driver:
   - `cargo build --release -p fragile-cli --bin fragilec`
3. Capture strict replay artifacts:
   - 120s profile/timing:
     - `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_c_a_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_c_a_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
   - 300s profile/timing:
     - `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_c_a_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_c_c_c_c_a_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`
4. Verify full-suite baseline parity:
   - `cargo test --workspace --all-targets`
   - `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Expected Evidence Markers

- 120s profile:
  - `status=codegen_after_template_collection`
- 300s profile:
  - `status=codegen_after_template_instantiation_generation`
  - `input_bytes=567404`
- Replay manifest:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
