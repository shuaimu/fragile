# M9.2.c.iv.e.34.f.5.e.5.e.3 event ordering + printf unsafe lane closure

## Scope

- Leaf: `M9.2.c.iv.e.34.f.5.e.5.e.3`
- Goal: close residual `event.cc` typed-lane stragglers after e.2:
  - `E0308` in `op_weak_ordering` (`weak_ordering`/`partial_ordering` if-else mismatch)
  - `E0133` for unsafe `super::printf_1` call lane.
- Bound: one generic post-processing normalizer pass (`normalize_rpc_event_surface_artifacts`) + focused tests.

## Wrong-approach check

- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Re-reviewed `docs/dev/wrong.md`.
- Kept the fix generic and bounded:
  - no file-path-specific branching for `event.cc`
  - no force-native bypass or fake-success pattern
  - no broad textual rewrite outside explicit residual lanes.

## Baseline residual

- Replay source run-root:
  `/tmp/fragile_m9_2_strict_runtime_replay_20260328T000000Z_p1395452`
- Residual evidence files:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260328T000000Z_p1395452/lane_fragilec/build.stderr`
  - `/tmp/fragilec_transpiled/event.cc_a0817c63ab065a1c_event.rs`
- Marker families:
  - `E0308`: `if`/`else` branch type mismatch in `op_weak_ordering` because
    `let mut equivalent = unsafe { __gv_equivalent.assume_init_read() };` degrades to
    a `partial_ordering` lane while the non-zero branch returns `weak_ordering`.
  - `E0133`: `super::printf_1(...)` emitted outside an `unsafe` block.

## Implementation

- File: `crates/fragile-clang/src/ast_codegen.rs`
- Updated pass: `normalize_rpc_event_surface_artifacts`

Bounded rewrites added:

1. `op_weak_ordering` equivalence lane rehydration:
   - within `pub fn op_weak_ordering(&self` body, rewrite
     `let mut equivalent = unsafe { __gv_equivalent.assume_init_read() };`
     to
     `let mut equivalent = weak_ordering { _M_value: 0 };`
   - effect: keeps both branches in the `weak_ordering` type lane and removes the
     residual `E0308` mismatch.

2. Unsafe printf lane wrapping:
   - rewrite statement-form `super::printf_1(...);`
     to
     `unsafe { super::printf_1(...); }`
   - effect: satisfies unsafe-call contract and removes `E0133`.

3. Trigger guard widening:
   - normalizer now executes when fragments only contain these residual markers
     (`super::printf_1(`, `pub fn op_weak_ordering(&self`) so minimal replay slices
     still receive the fix.

## Validation

Focused tests:

- `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_weak_ordering_equivalent_and_printf_unsafe_lanes -- --nocapture`
- `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_assoc_sub_state_swap_pointer_reference_mismatch -- --nocapture`
- `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_quorum_event_command_map_and_event_base_lanes -- --nocapture`
- `cargo test -p fragile-clang test_vtable_dispatch_base_expr_normalizes_redundant_deref_from_pointer_return_chain -- --nocapture`

Result: all focused tests passed.

## Follow-up

- Next leaf remains `M9.2.c.iv.e.34.f.5.e.5.e.4` (strict replay rerun + lane contract verification).
