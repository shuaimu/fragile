# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.i Design (2026-03-13)

## Scope

Selected leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.c.i`

Task intent: implement the next generic pre-`codegen_after_top_level_generation`
hot-path optimization and lock behavior with focused regressions.

## Size Analysis

Estimated implementation size is small (<500 LOC):

- targeted recursion-guard changes in `ast_codegen`
- one focused regression
- no parser/runtime protocol changes

No further decomposition was required for this leaf.

## Plan

1. Reduce recursion overhead in template prepasses by skipping descent for leaf
   nodes (`children.is_empty()`).
2. Preserve semantic traversal for non-empty children in both explicit branches
   and default branch recursion.
3. Add focused regression locking default-branch recursion behavior.
4. Capture deterministic strict replay profile/timing artifacts.
5. Run full regression suites and verify baseline parity.

## Wrong-Approach Guard Check

Validated against `docs/fragile-dev-book.md` section `1.3` and
`docs/dev/wrong.md`:

- no target-specific hacks
- no force-native bypass
- no synthetic semantic fallback/stub behavior
- generic traversal optimization only

## Implementation

Updated:

- `crates/fragile-clang/src/ast_codegen.rs`

Key changes:

- Added `has_children` guards in `collect_template_definitions_with_namespace_stack`
  to avoid recursive calls on leaf nodes.
- Added equivalent `has_children` guards in `collect_template_usages` across
  explicit and default recursion branches.
- Added focused regression:
  - `test_collect_template_usages_descends_default_branch_with_children`

The regression locks that default-branch recursion still descends through
non-empty wrapper nodes and collects template instantiation usage (`Widget<int>`).

## Commands Executed

```bash
cargo test -p fragile-clang test_collect_template_usages_descends_default_branch_with_children -- --nocapture
cargo test -p fragile-clang test_collect_template_definitions_with_namespace_restores_sibling_paths -- --nocapture
cargo test -p fragile-clang test_collect_template_info_keeps_inline_namespace_alias_for_usage_scan -- --nocapture
cargo build --release -p fragile-cli --bin fragilec
FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_i_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_i_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120
FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_i_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_i_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Evidence Summary

Replay profiles:

- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_i_callshape_profile_120_v1.txt`
  - `status=codegen_after_template_collection`
  - `status_history=codegen_started,codegen_after_template_collection`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_c_i_callshape_profile_300_v1.txt`
  - `status=codegen_after_template_instantiation_generation`
  - `status_history=codegen_started,codegen_after_template_collection,codegen_after_template_instantiation_generation`
  - `input_bytes=575929`

Comparison to previous optimization leaf
`2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.a` (`input_bytes=575274` at 300s):

- checkpoint bytes increased by `+655` (no improvement in this metric).

Replay manifest (`/tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313/rpc_compile_blocker_replay_manifest.txt`):

- `replay_01_status=124`
- `replay_01_timed_out=true`
- `replay_01_first_failure_class=build_timeout`
- `replay_01_blocker_file=src/rrr/base/misc.cpp`

Regression suites:

- `cargo test --workspace --all-targets`: `fragile-clang` lib
  `737` passed / `46` failed (known baseline failure count unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`:
  `OK`, `29` ran, `1` skipped
