# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.a Design Note (2026-03-13)

## Scope

Leaf: `2.6.c.iv.d.iv.a`  
Objective: implement the next generic optimization in the
`codegen_after_template_instantiation_generation` ->
`codegen_after_top_level_generation` window.

## Size/Complexity Check

This leaf is a focused codegen + regression update (`<500 LOC`) in one Rust
file plus docs/TODO updates.

## Wrong-Approach Guard

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-specific conditionals
- no force-native bypass
- no fake semantic bodies or compile-only stubs
- generic codegen path optimization only

## Plan

1. Identify top-level generation path that still clones large structures.
2. Replace clone-heavy namespace-merge index retrieval with ownership transfer.
3. Strengthen reopened-namespace regression assertions to catch duplicate/missing
   emissions.
4. Re-run focused tests, strict replay evidence capture, and full suites.

## Changes Implemented

Updated `crates/fragile-clang/src/ast_codegen.rs`:

- in `generate_top_level`, changed merged namespace index retrieval from:
  - `self.merged_namespace_children.get(&module_key).cloned()`
  to:
  - `self.merged_namespace_children.remove(&module_key)`
- this removes an avoidable `Vec<usize>` clone per merged namespace module
  emission (first occurrence path).

Regression update:

- strengthened `test_reopened_namespace_merges_all_children_without_dropping_entries`
  by asserting `fn lane_a(` and `fn lane_b(` each appear exactly once.

## Commands Executed

```bash
cargo test -p fragile-clang reopened_namespace_merges_all_children_without_dropping_entries -- --nocapture
cargo test -p fragile-clang problematic_callshape -- --nocapture

cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_a_callshape_profile_120_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_a_stage_timing_120_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 120

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_a_callshape_profile_300_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_a_stage_timing_300_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 300

cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Deterministic Evidence

Artifacts:

- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_a_callshape_profile_120_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_a_callshape_profile_300_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_a_stage_timing_120_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_a_stage_timing_300_v1.txt`

Replay/profile highlights:

- 120s profile: `status_history=codegen_started`
- 300s profile:
  `status_history=codegen_started,codegen_after_template_collection,codegen_after_template_instantiation_generation`
- replay remains timeout-bound on `src/rrr/base/misc.cpp`

Full-suite status:

- `cargo test --workspace --all-targets`: known baseline remains
  (`46` existing `fragile-clang` lib failures, unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`:
  `29` passed, `1` skipped

## Outcome

Leaf `2.6.c.iv.d.iv.a` is complete: top-level namespace merge emission now
avoids per-module `Vec<usize>` clone overhead while preserving merged namespace
semantics under strengthened focused regression coverage.
