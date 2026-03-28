# M9.2.c.iv.e.34.f.5.e.5.e.2 event assoc-sub-state swap callshape closure

## Scope

- Leaf: `M9.2.c.iv.e.34.f.5.e.5.e.2`
- Goal: close `event.cc` residual `E0308` pointer/reference mismatch where
  `swap_std___assoc_sub_state` expects raw pointers but generated lanes pass
  `&mut *mut ...`.
- Bound: one normalization pass (`normalize_rpc_event_surface_artifacts`) plus
  focused regression tests, no target-specific hacks.

## Wrong-approach check

- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Re-reviewed `docs/dev/wrong.md`.
- Kept fix generic and bounded (no `event.cc`-specific conditional branch).

## Baseline residual

- Replay source run-root:
  `/tmp/fragile_m9_2_strict_runtime_replay_20260328T000000Z_p1395452`
- Residual compile artifact:
  `/tmp/fragile_m9_2_strict_runtime_replay_20260328T000000Z_p1395452/lane_fragilec/build.stderr`
- Marker family:
  - `E0308`
  - expected `*mut std___assoc_sub_state`
  - found `&mut *mut __assoc_sub_state` / `&mut *mut __assoc_state_std_ffi_c_void`

## Implementation

- File: `crates/fragile-clang/src/ast_codegen.rs`
- Normalizer updated: `normalize_rpc_event_surface_artifacts`
- Added bounded rewrite lanes:
  - `swap_std___assoc_sub_state(&mut self.__state_, &mut __rhs.__state_);`
    -> `swap_std___assoc_sub_state(self.__state_ as *mut std___assoc_sub_state, __rhs.__state_ as *mut std___assoc_sub_state);`
  - `swap_std___assoc_sub_state(&mut self.__state_, &mut __f.__state_);`
    -> `swap_std___assoc_sub_state(self.__state_ as *mut std___assoc_sub_state, __f.__state_ as *mut std___assoc_sub_state);`
- Expanded early-return trigger guard so this pass executes when swap residual
  markers are present even in minimal replay fragments.

## Validation

- Focused unit test:
  - `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_assoc_sub_state_swap_pointer_reference_mismatch -- --nocapture`
- Companion guards re-run:
  - `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_quorum_event_command_map_and_event_base_lanes -- --nocapture`
  - `cargo test -p fragile-clang test_vtable_dispatch_base_expr_normalizes_redundant_deref_from_pointer_return_chain -- --nocapture`
- Result: all focused tests pass.

## Follow-up

- Next leaf remains `M9.2.c.iv.e.34.f.5.e.5.e.3`.
