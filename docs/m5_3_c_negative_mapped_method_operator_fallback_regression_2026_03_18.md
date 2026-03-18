# M5.3.c Negative Mapped Method-Operator Fallback Regression (2026-03-18)

## Context

Top unfinished P0 leaf after `M5.3.b`: `M5.3.c`.

## Scope Sizing

Estimated scope was small (<1000 LOC):

- `crates/fragile-clang/src/lib.rs` regression/validator updates: ~120 LOC.
- `TODO.md` status updates: 2 LOC.
- Design/dev-book notes: <120 LOC.

No further decomposition required for this leaf.

## Implemented Leaf

- `M5.3.c Add negative regression proving mapped method/operator lanes fail deterministically when unresolved placeholder fallback would be required.`

## Design Decision

Use two deterministic parser-output handoff regressions in `lib.rs`:

1. Direct completeness-validator regression with mapped sequence/smart-pointer
   method/operator lanes and unresolved placeholder structs.
2. Active handoff integration regression that reproduces unresolved covered-family
   structs (`vector_`, `unique_ptr_`) under mapped placeholder context.

Also strengthen completeness validation to detect covered-family unresolved
placeholder structs regardless of whether a marker comment is present.

Why this is the best fit:

- It verifies deterministic failure behavior at the exact handoff validation gate.
- It avoids fake stubs and preserves strict failure semantics for unresolved
  mapped families.
- It closes a real validator blind spot where unresolved covered-family structs
  without the marker comment were not being flagged.

## Implementation Summary

Changes in `crates/fragile-clang/src/lib.rs`:

- Added/kept negative regression:
  - `parser_output_mapping_completeness_validation_rejects_sequence_smart_pointer_placeholder_fallback_with_method_operator_lanes`
- Added integration negative regression:
  - `parser_output_codegen_active_handoff_mapped_sequence_smart_pointer_unresolved_shapes_fail_mapping_completeness`
- Updated mapping completeness detector:
  - `parser_output_mapping_completeness_violations_for_covered_families(...)`
  - now flags covered-family unresolved struct fallbacks directly (not only
    marker-comment-delimited placeholder blocks).

## Wrong-Approach Compliance

Checked against `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md`.

- No target-specific hacks were introduced.
- No fake semantic method bodies were introduced.
- No force-native bypasses were introduced.

## Validation

Focused commands run:

- `cargo test -p fragile-clang parser_output_mapping_completeness_validation_rejects_sequence_smart_pointer_placeholder_fallback_with_method_operator_lanes -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_mapped_sequence_smart_pointer_unresolved_shapes_fail_mapping_completeness -- --nocapture`
- `cargo test -p fragile-clang test_parser_output_mapping_ -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_ -- --nocapture`
