# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.a Design (2026-03-13)

## Scope

Selected leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.iii.a`

Task intent: implement the next generic codegen hot-path optimization in the
pre-`codegen_after_top_level_generation` window and lock behavior with focused
regressions.

## Size Analysis

Estimated change size is small (well under 500 LOC):

- targeted `ast_codegen` traversal optimization
- one focused regression test
- no parser/runtime protocol changes

No further decomposition was required for leaf `...c.iii.a`.

## Plan

1. Optimize template-definition namespace traversal to reduce per-namespace path
   allocations.
2. Preserve inline-namespace alias semantics and template key behavior.
3. Add focused regression coverage to prevent namespace-path leakage across
   sibling traversals.
4. Capture deterministic strict replay profiling/timing artifacts.
5. Run full regression suites and verify baseline parity.

## Wrong-Approach Guard Check

Validated against `docs/fragile-dev-book.md` section `1.3` and
`docs/dev/wrong.md`:

- no target-specific hacks
- no force-native bypass
- no synthetic semantic fallback/stub behavior
- generic codegen traversal optimization only

## Implementation

Updated:

- `crates/fragile-clang/src/ast_codegen.rs`

Key change:

- `collect_template_definitions_with_namespace` now delegates to
  `collect_template_definitions_with_namespace_stack`.
- The new helper tracks namespace path using a mutable stack (`push`/`pop`)
  instead of cloning namespace-path vectors on each namespace descent.
- Inline namespace alias registration behavior is preserved.

Focused regression added:

- `test_collect_template_definitions_with_namespace_restores_sibling_paths`

This test locks that sibling namespace traversals keep independent paths
(`alpha::Widget`, `beta::Widget`) and do not leak stack state
(`alpha::beta::Widget`).

## Commands Executed

```bash
cargo test -p fragile-clang test_collect_template_definitions_with_namespace_restores_sibling_paths -- --nocapture
cargo test -p fragile-clang test_collect_template_info_keeps_inline_namespace_alias_for_usage_scan -- --nocapture
cargo test -p fragile-clang test_collect_template_info_builds_fn_template_leaf_index_for_namespaced_templates -- --nocapture
cargo test -p fragile-clang test_collect_fn_template_candidate_keys_uses_leaf_index_entries -- --nocapture
cargo build --release -p fragile-cli --bin fragilec
FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_a_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_a_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120
FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_a_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_a_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Evidence Summary

Replay profiles:

- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_a_callshape_profile_120_v1.txt`
  - `status=codegen_after_template_collection`
  - `status_history=codegen_started,codegen_after_template_collection`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_iii_a_callshape_profile_300_v1.txt`
  - `status=codegen_after_template_instantiation_generation`
  - `status_history=codegen_started,codegen_after_template_collection,codegen_after_template_instantiation_generation`
  - `input_bytes=575274`

Comparison to prior completed optimization leaf
`2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.i` (`input_bytes=572773` at 300s):

- checkpoint bytes increased by `+2501` (no improvement in this metric).

Replay manifest (`/tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313/rpc_compile_blocker_replay_manifest.txt`):

- `replay_01_status=124`
- `replay_01_timed_out=true`
- `replay_01_first_failure_class=build_timeout`
- `replay_01_blocker_file=src/rrr/base/misc.cpp`

Regression suites:

- `cargo test --workspace --all-targets`: `fragile-clang` lib
  `736` passed / `46` failed (known baseline failure count unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`:
  `OK`, `29` ran, `1` skipped
