# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.i Design (2026-03-13)

## Scope and sizing

Leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.i`

The selected work is a localized generic codegen optimization in
`crates/fragile-clang/src/ast_codegen.rs` and is below the requested LOC
threshold.

## Problem

Template-usage traversal still cloned namespace-path vectors during recursive
walks even though usage collection does not consume namespace-path values. This
adds avoidable overhead in a hot pre-top-level codegen path.

## Wrong-approach check

Validated against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-specific hacks
- no force-native fallback behavior
- no synthesized semantic method stubs
- generic traversal/data-flow optimization with preserved semantics

## Implementation

Updated `crates/fragile-clang/src/ast_codegen.rs`:

- replaced namespace-aware usage traversal with namespace-agnostic
  `collect_template_usages`
- kept alias recording in definition prepass
  (`collect_template_definitions_with_namespace`) and invoked usage collection
  afterwards in `collect_template_info`
- added focused regression to lock alias behavior after traversal simplification:
  - `test_collect_template_info_keeps_inline_namespace_alias_for_usage_scan`
- retained candidate-index coverage regressions:
  - `test_collect_template_info_builds_fn_template_leaf_index_for_namespaced_templates`
  - `test_collect_fn_template_candidate_keys_uses_leaf_index_entries`

## Validation

Executed commands:

- `cargo test -p fragile-clang test_collect_template_info_builds_fn_template_leaf_index_for_namespaced_templates -- --nocapture`
- `cargo test -p fragile-clang test_collect_template_info_keeps_inline_namespace_alias_for_usage_scan -- --nocapture`
- `cargo test -p fragile-clang test_collect_fn_template_candidate_keys_uses_leaf_index_entries -- --nocapture`
- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_i_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_i_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_i_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_c_i_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Evidence highlights:

- profile 120s: `status=codegen_after_template_collection`
- profile 300s: `status=codegen_after_template_instantiation_generation`,
  `input_bytes=572773`
- comparison to prior `iii.c.iii.a` 300s profile (`input_bytes=573159`): `-386`
- replay manifest:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
- full suites:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib
    `735 passed / 46 failed` (known baseline)
  - Python suite: `Ran 29 tests`, `OK (skipped=1)`

## Outcome

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.c.i` is complete. Template-usage
collection now avoids namespace-path cloning while preserving inline-namespace
alias behavior and baseline suite outcomes.
