# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.c.i Design (2026-03-13)

## Scope and sizing

Leaf: `2.6.c.iv.d.iv.c.iv.c.iii.c.i`

The selected work is a localized generic codegen optimization in
`crates/fragile-clang/src/ast_codegen.rs` and is well below the requested LOC
threshold.

## Problem

The pre-`codegen_after_top_level_generation` path still had avoidable clone
churn in vtable generation:

- `generate_all_vtable_structs` cloned all vtable entries before filtering for
  root polymorphic classes.
- `generate_all_static_vtables` cloned all vtable entries before filtering out
  abstract classes.

This unnecessarily clones payloads that are deterministically skipped.

## Wrong-approach check

Validated against `docs/fragile-dev-book.md` Section 1.3 and
`docs/dev/wrong.md`:

- no target-specific hacks
- no force-native fallback behavior
- no synthesized semantic method stubs
- generic data-flow optimization with preserved semantics

## Implementation

Updated `crates/fragile-clang/src/ast_codegen.rs`:

- Added `collect_root_vtable_class_names` and
  `collect_concrete_vtable_class_names` helper selectors.
- Reworked `generate_all_vtable_structs` and
  `generate_all_static_vtables` to select class names first and clone only
  selected vtable entries.
- Added focused regression tests:
  - `test_collect_root_vtable_class_names_skips_derived_entries`
  - `test_collect_concrete_vtable_class_names_skips_abstract_entries`

## Validation

Executed commands:

- `cargo test -p fragile-clang test_collect_root_vtable_class_names_skips_derived_entries -- --nocapture`
- `cargo test -p fragile-clang test_collect_concrete_vtable_class_names_skips_abstract_entries -- --nocapture`
- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_i_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_i_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_i_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_c_i_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Evidence highlights:

- profile 120s: `status=codegen_after_template_collection`
- profile 300s: `status=codegen_after_template_instantiation_generation`,
  `input_bytes=568059`
- replay manifest:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
- full suites:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib
    `732 passed / 46 failed` (known baseline)
  - Python suite: `Ran 29 tests`, `OK (skipped=1)`

## Outcome

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.c.i` is complete. The optimization removes
clone-all staging in vtable generation while preserving behavior and baseline
suite outcomes.
