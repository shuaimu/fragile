# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.a Design (2026-03-13)

## Scope

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.c.a` requires one generic codegen hot-path optimization in the dominant pre-`codegen_after_top_level_generation` timeout window.

## Problem

`collect_fn_template_instantiation` and `resolve_fn_template_call_name_from_args` were allocating temporary `Vec<&ClangNode>` values only to pass call arguments into template type-argument inference. This introduces avoidable per-call churn in a hot template-instantiation path.

## Design Decision

- Remove transient call-arg reference-vector allocations and pass direct slices from AST node storage.
- Update inference API to accept `Option<&[ClangNode]>` so callers can use direct slices without adapter allocations.
- Keep behavior and matching semantics unchanged.
- Lock behavior with focused NTTP inference regression coverage.

## Wrong-Approach Check

Validated against `docs/fragile-dev-book.md` section 1.3 and `docs/dev/wrong.md`:

- no benchmark-target special casing
- no force-native source bypass
- no fake semantic stubs/fallback method bodies

## Implementation Summary

File touched:

- `crates/fragile-clang/src/ast_codegen.rs`

Key changes:

- `collect_fn_template_instantiation` now passes `&call_node.children[1..]` directly.
- `resolve_fn_template_call_name_from_args` now passes `call_arg_nodes` directly.
- `infer_fn_template_type_args(..., instantiated_args)` changed from `Option<&[&ClangNode]>` to `Option<&[ClangNode]>`.
- Added regression `test_collect_fn_template_instantiation_infers_nttp_from_direct_call_arg_slice`.
- Updated existing NTTP inference tests to use direct slices.

## Validation

Targeted tests:

- `cargo test -p fragile-clang test_collect_fn_template_instantiation_infers_nttp_from_direct_call_arg_slice -- --nocapture`
- `cargo test -p fragile-clang test_collect_fn_template_instantiation_uses_leaf_index_candidate_after_mismatch -- --nocapture`
- `cargo test -p fragile-clang test_function_template_type_arg_inference_nttp_array_ref_uses_literal_bound -- --nocapture`
- `cargo test -p fragile-clang test_function_template_type_arg_inference_nttp_array_ref_does_not_fallback_to_pointer_type -- --nocapture`

Strict replay evidence:

- Built release driver: `cargo build --release -p fragile-cli --bin fragilec`
- Replay commands:
  - `FRAGILEC_MODE=strict ... --timeout-seconds 120`
  - `FRAGILEC_MODE=strict ... --timeout-seconds 300`
- Profile artifacts:
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_a_callshape_profile_120_v1.txt`
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_a_callshape_profile_300_v1.txt`
- Stage timing artifacts:
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_a_stage_timing_120_v1.txt`
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_c_a_stage_timing_300_v1.txt`

Observed markers:

- `120s`: `status=codegen_after_template_collection`
- `300s`: `status=codegen_after_template_instantiation_generation`, `input_bytes=566366`
- Replay manifest (`/tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313/rpc_compile_blocker_replay_manifest.txt`):
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`

Full suites:

- `cargo test --workspace --all-targets` -> `fragile-clang` lib `744` passed / `46` failed (known baseline failure count unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'` -> `OK` (`29` ran, `1` skipped)
