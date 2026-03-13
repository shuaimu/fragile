# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.a Design Note (2026-03-13)

## Scope

This leaf implements a bounded generic codegen hot-path optimization in
function-template candidate scanning.

No parser-behavior changes, runtime semantic stubs, force-native bypasses, or
RPC-target-specific branches are introduced.

## Problem

In `collect_fn_template_instantiation`, per-candidate scanning still performed
avoidable allocation-heavy work before compatibility was established:

- sanitized type-arg vector + mangled instantiation name materialization for
  every inferred candidate
- temporary `Vec<String>` allocation for substituted parameter compatibility
  checks

This sits in the `codegen_after_template_instantiation_generation` hot window.

## Changes

Updated `crates/fragile-clang/src/ast_codegen.rs`:

1. Deferred mangled-name generation

- candidate loop now tracks only `(template_key, type_args)` for
  selected/fallback candidates
- `mangled_name` is built once, after winner selection

2. Shared helper

- added `build_fn_template_mangled_name(sanitized_fn_name, type_args)`
- reused by both:
  - `collect_fn_template_instantiation`
  - `resolve_fn_template_call_name_from_args`

3. Streaming compatibility checks

- replaced allocated substituted-parameter vector with streaming
  `substitute_template_type` + normalized comparison checks

## Correctness Guard

Added focused regression:

- `test_build_fn_template_mangled_name_sanitizes_type_args`

This locks the helper’s mangled-name sanitization semantics so deferred
materialization does not drift output naming behavior.

## Validation Commands

```bash
cargo test -p fragile-clang test_build_fn_template_mangled_name_sanitizes_type_args -- --nocapture
cargo test -p fragile-clang test_template_match_type_normalization_preserves_ref_prefix_compatibility -- --nocapture
cargo test -p fragile-clang test_collect_fn_template_instantiation_uses_leaf_index_candidate_after_mismatch -- --nocapture

cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_a_callshape_profile_120_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_a_stage_timing_120_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 120

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_a_callshape_profile_300_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_a_stage_timing_300_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 300

cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Results

- 120s profile status: `codegen_after_template_collection`
- 300s profile status: `codegen_after_template_instantiation_generation`
- 300s checkpoint bytes: `input_bytes=573589`
- delta vs prior leaf (`574217`): `-628`
- replay blocker class unchanged: timeout on `src/rrr/base/misc.cpp`
- full suite parity retained:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib
    `741` passed / `46` failed (failure count unchanged)
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`:
    `OK (29 ran, 1 skipped)`

## Next Leaf

Proceed to paired gate leaf
`2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.b`.
