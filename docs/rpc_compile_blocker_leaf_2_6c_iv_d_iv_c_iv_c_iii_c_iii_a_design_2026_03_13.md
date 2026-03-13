# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.iii.a Design (2026-03-13)

## Scope and sizing

Leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.a`

The selected work is a localized generic codegen optimization in
`crates/fragile-clang/src/ast_codegen.rs` and is below the requested LOC
threshold.

## Problem

Function-template candidate resolution repeatedly scanned all
`fn_template_definitions` keys for `::<fn_name>` suffix matches in two hot
call paths:

- `collect_fn_template_instantiation`
- `resolve_fn_template_call_name_from_args`

This introduces repeated full-map scans during template-heavy codegen.

## Wrong-approach check

Validated against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-specific hacks
- no force-native fallback behavior
- no synthesized semantic method stubs
- generic lookup/index optimization with preserved semantics

## Implementation

Updated `crates/fragile-clang/src/ast_codegen.rs`:

- added `fn_template_keys_by_leaf` index to `AstCodeGen`
- added `rebuild_fn_template_leaf_index` and invoked it from
  `collect_template_info` after definition precollection
- added shared `collect_fn_template_candidate_keys` helper
- switched `collect_fn_template_instantiation` and
  `resolve_fn_template_call_name_from_args` to the shared helper
- retained fallback scanning when index entries are unavailable (for direct test
  paths that bypass full precollection)
- added focused regressions:
  - `test_collect_template_info_builds_fn_template_leaf_index_for_namespaced_templates`
  - `test_collect_fn_template_candidate_keys_uses_leaf_index_entries`

## Validation

Executed commands:

- `cargo test -p fragile-clang test_collect_template_info_builds_fn_template_leaf_index_for_namespaced_templates -- --nocapture`
- `cargo test -p fragile-clang test_collect_fn_template_candidate_keys_uses_leaf_index_entries -- --nocapture`
- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_a_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_a_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_a_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_iii_a_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Evidence highlights:

- profile 120s: `status=codegen_started`
- profile 300s: `status=codegen_after_template_instantiation_generation`,
  `input_bytes=573159`
- comparison to prior `iii.c.i` 300s profile (`input_bytes=568059`): `+5100`
- replay manifest:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
- full suites:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib
    `734 passed / 46 failed` (known baseline)
  - Python suite: `Ran 29 tests`, `OK (skipped=1)`

## Outcome

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.iii.a` is complete. Function-template
candidate lookup now uses a precomputed leaf-name index in the common path;
strict replay remained timeout-bound with no measured checkpoint advancement in
this leaf.
