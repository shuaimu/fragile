# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.c.a Design (2026-03-13)

## Scope

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.c.a` adds the next generic codegen hot-path optimization inside the dominant pre-`codegen_after_top_level_generation` timeout window.

## Problem

`infer_fn_template_type_args` repeatedly rescanned template parameter usage and unsized-array-ref markers for every template parameter. This repeated AST/type-walk work in a hot template-instantiation inference loop.

## Design Decision

- Precompute per-template-parameter usage in parameter and return positions once.
- Hoist unsized-array-ref detection out of the per-template-param loop.
- Hoist fallback return-type string materialization out of the loop.
- Keep inference semantics unchanged for non-type array-ref inference and return-position inference.

## Wrong-Approach Check

Validated against `docs/fragile-dev-book.md` section 1.3 and `docs/dev/wrong.md`:

- no target-specific conditionals
- no force-native compile bypass
- no fake fallback semantics

## Implementation Summary

File changed:

- `crates/fragile-clang/src/ast_codegen.rs`

Key changes:

- In `infer_fn_template_type_args`:
  - added `has_unsized_array_ref_param` precompute
  - added `template_param_usage: Vec<(bool, bool)>` precompute (`appears_in_params`, `appears_in_return`)
  - reused precomputed usage flags instead of repeated `cpp_type_contains_template_param` scans
  - hoisted fallback return type string as `fallback_return_ty`
- Added focused regression:
  - `test_function_template_type_arg_inference_uses_return_type_when_params_do_not_reference_template`

## Validation

Targeted tests:

- `cargo test -p fragile-clang test_function_template_type_arg_inference_uses_return_type_when_params_do_not_reference_template -- --nocapture`
- `cargo test -p fragile-clang test_function_template_type_arg_inference_nttp_array_ref_uses_literal_bound -- --nocapture`
- `cargo test -p fragile-clang test_function_template_type_arg_inference_nttp_array_ref_does_not_fallback_to_pointer_type -- --nocapture`
- `cargo test -p fragile-clang test_collect_fn_template_instantiation_infers_nttp_from_direct_call_arg_slice -- --nocapture`

Strict replay evidence:

- Built release driver: `cargo build --release -p fragile-cli --bin fragilec`
- Replay commands:
  - `FRAGILEC_MODE=strict ... --timeout-seconds 120`
  - `FRAGILEC_MODE=strict ... --timeout-seconds 300`
- Profile artifacts:
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_a_callshape_profile_120_v1.txt`
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_a_callshape_profile_300_v1.txt`
- Stage timing artifacts:
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_a_stage_timing_120_v1.txt`
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_c_a_stage_timing_300_v1.txt`

Observed markers:

- `120s`: `status=codegen_after_template_collection`
- `300s`: `status=codegen_after_template_instantiation_generation`, `input_bytes=574305`
- Replay manifest (`/tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313/rpc_compile_blocker_replay_manifest.txt`):
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`

Full suites:

- `cargo test --workspace --all-targets` -> `fragile-clang` lib `745` passed / `46` failed (known baseline failure count unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'` -> `OK` (`29` ran, `1` skipped)

## Outcome

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.c.a` is complete. The optimization reduces repeated template-usage scans in function-template inference, with behavior locked by focused regression tests. The strict replay remains timeout-bound on `src/rrr/base/misc.cpp`; next paired gate leaf is `...c.c.c.c.c.b`.
