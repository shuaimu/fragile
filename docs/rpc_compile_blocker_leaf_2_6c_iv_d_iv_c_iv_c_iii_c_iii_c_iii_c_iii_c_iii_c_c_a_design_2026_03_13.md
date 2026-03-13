# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.a Design Note (2026-03-13)

## Scope

This leaf implements a bounded generic codegen hot-path optimization in
function-template candidate-key collection.

No parser/runtime behavior changes, target-specific branches, or force-native
bypasses are introduced.

## Problem

`collect_fn_template_candidate_keys` allocated a transient `HashSet<String>` and
performed extra `String` clones for every call-site scan.
In strict timeout replays, this path is exercised repeatedly while collecting
function-template instantiations before top-level generation.

## Changes

Updated `crates/fragile-clang/src/ast_codegen.rs`:

1. Replaced per-call `HashSet` dedupe with deterministic in-place `Vec`
   dedupe (`iter().any`).
2. Preserved candidate priority:
   - namespaced call-path key
   - unqualified key
   - leaf-index/fallback qualified keys
3. Added focused regression:
   - `test_collect_fn_template_candidate_keys_deduplicates_and_keeps_priority_order`

## Wrong-Approach Check

Conforms to Section 1.3 and `docs/dev/wrong.md`:

- no RPC-target-specific conditionals
- no force-native bypasses
- no fake semantic stubs
- generic codegen optimization only

## Validation Commands

```bash
cargo test -p fragile-clang test_collect_fn_template_candidate_keys_deduplicates_and_keeps_priority_order -- --nocapture
cargo test -p fragile-clang test_collect_fn_template_instantiation_uses_leaf_index_candidate_after_mismatch -- --nocapture
cargo test -p fragile-clang test_template_match_type_normalization_preserves_ref_prefix_compatibility -- --nocapture

cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_a_callshape_profile_120_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_a_stage_timing_120_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 120

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_a_callshape_profile_300_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_iii_c_iii_c_c_a_stage_timing_300_v1.txt \
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
- 300s profile:
  - `status=codegen_after_template_instantiation_generation`
  - `input_bytes=574915`
- comparison vs prior leaf (`input_bytes=573589`):
  - delta `+1326`
- replay blocker status unchanged:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
- full suite parity retained:
  - `cargo test --workspace --all-targets`:
    `fragile-clang` lib `742` passed / `46` failed (failure count unchanged)
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`:
    `OK (29 ran, 1 skipped)`

## Next Leaf

Proceed to paired gate leaf
`2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.iii.c.iii.c.c.b`.
