# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.a Design Note (2026-03-13)

## Scope

This leaf implements a bounded generic codegen hot-path optimization in
function-template parameter compatibility checks.

No parser/runtime behavior changes, target-specific branches, or force-native
bypasses are introduced.

## Problem

`collect_fn_template_instantiation` allocated a transient
`Vec<String>` (`instantiated_param_types_ref_stripped`) and performed repeated
reference-prefix strip allocations during template candidate matching.

In strict timeout replays, this path is exercised repeatedly while collecting
function-template instantiations before top-level generation.

## Plan

1. Remove per-compare stripped-type string allocations while preserving
   template compatibility semantics.
2. Add focused regression coverage for reference-prefix compatibility behavior.
3. Re-run targeted template-instantiation regressions.
4. Rebuild release `fragilec`, collect strict replay evidence at 120s/300s,
   and then run full regression suites.

## Wrong-Approach Check

Conforms to Section 1.3 and `docs/dev/wrong.md`:

- no RPC-target-specific conditionals
- no force-native bypasses
- no fake semantic stubs
- generic codegen optimization only

## Changes

Updated `crates/fragile-clang/src/ast_codegen.rs`:

1. Removed `instantiated_param_types_ref_stripped` pre-allocation in
   `collect_fn_template_instantiation`.
2. Changed `strip_template_match_ref_prefix` from returning `String` to `&str`.
3. Added `template_match_types_compatible(lhs_norm, rhs_norm)` helper for
   wildcard/exact/reference-prefix compatibility checks.
4. Added focused regression:
   - `test_template_match_types_compatible_handles_ref_prefix_variants`

## Validation Commands

```bash
cargo test -p fragile-clang test_template_match_types_compatible_handles_ref_prefix_variants -- --nocapture
cargo test -p fragile-clang test_template_match_type_normalization_preserves_ref_prefix_compatibility -- --nocapture
cargo test -p fragile-clang test_collect_fn_template_instantiation_uses_leaf_index_candidate_after_mismatch -- --nocapture

cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_a_callshape_profile_120_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_a_stage_timing_120_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 120

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_a_callshape_profile_300_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_c_a_stage_timing_300_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 300

cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Results

- 120s profile:
  - `status=codegen_after_template_collection`
  - `status_history=codegen_started,codegen_after_template_collection`
- 300s profile:
  - `status=codegen_after_template_instantiation_generation`
  - `status_history=codegen_started,codegen_after_template_collection,codegen_after_template_instantiation_generation`
  - `input_bytes=565063`
- comparison vs prior optimization leaf (`...c.c.a`, `input_bytes=574915`):
  - delta `-9852`
- replay blocker status unchanged:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
- full-suite baseline parity retained:
  - `cargo test --workspace --all-targets`:
    `fragile-clang` lib `743` passed / `46` failed (failure count unchanged)
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`:
    `OK (29 ran, 1 skipped)`

## Next Leaf

Proceed to paired gate leaf
`2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.c.b`.
