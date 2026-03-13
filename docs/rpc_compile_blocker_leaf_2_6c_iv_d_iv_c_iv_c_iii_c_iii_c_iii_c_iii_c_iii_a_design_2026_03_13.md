# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.a Design Note (2026-03-13)

## Scope

This leaf implements a bounded generic codegen hot-path optimization in function-template candidate matching.

No parser behavior, runtime semantics, or RPC-target-specific conditionals are introduced.

## Problem

`collect_fn_template_instantiation` evaluated multiple candidate template keys for a call-site.
During each candidate iteration it rebuilt the same `call_args` vector and re-sanitized the same callee name.

That per-candidate allocation/rewrite churn is avoidable in the pre-`codegen_after_top_level_generation` path.

## Change

Updated `crates/fragile-clang/src/ast_codegen.rs`:

- In `collect_fn_template_instantiation`:
  - Precompute `call_args: Vec<&ClangNode>` once per call-site before candidate iteration.
  - Precompute `sanitized_fn_name` once and reuse for all candidate mangled-name construction.
- In `resolve_fn_template_call_name_from_args`:
  - Precompute `sanitized_fn_name` once and reuse per candidate.

## Correctness Guard

Added focused regression:

- `test_collect_fn_template_instantiation_uses_leaf_index_candidate_after_mismatch`

This locks that candidate scanning still skips an incompatible unqualified candidate and correctly selects a compatible qualified leaf-index candidate.

## Validation Commands

```bash
cargo test -p fragile-clang test_collect_fn_template_instantiation_uses_leaf_index_candidate_after_mismatch -- --nocapture
cargo test -p fragile-clang test_collect_fn_template_candidate_keys_uses_leaf_index_entries -- --nocapture
cargo test -p fragile-clang test_template_match_type_normalization_preserves_ref_prefix_compatibility -- --nocapture

cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_a_callshape_profile_120_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_a_stage_timing_120_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 120

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_a_callshape_profile_300_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_a_stage_timing_300_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 300
```

## Results

- 120s profile status: `codegen_after_template_collection`
- 300s profile status: `codegen_after_template_instantiation_generation`
- 300s checkpoint bytes: `input_bytes=574217` (`+467` vs prior leaf `573750`)
- Replay blocker class unchanged: timeout on `src/rrr/base/misc.cpp`
- Full suite parity retained:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib `740` passed / `46` failed
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`: `OK (29 ran, 1 skipped)`

## Next Leaf

Proceed to paired gate leaf `...c.iii.b` (strict build-only replay + inventory non-increase gate vs `2.6.c.iii` baseline).
