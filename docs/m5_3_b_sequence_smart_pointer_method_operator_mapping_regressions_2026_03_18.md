# M5.3.b Sequence/Smart-Pointer Method-Operator Mapping Regressions (2026-03-18)

## Context

Top unfinished P0 leaf after `M5.3.a`: `M5.3.b`.

## Scope Sizing

Estimated scope was small (<1000 LOC):

- New focused regression test in `crates/fragile-clang/src/ast_codegen.rs`: ~220 LOC.
- `TODO.md` leaf status update: 1 LOC.
- Design/dev-book notes: <100 LOC.

No further decomposition required for this leaf.

## Implemented Leaf

- `M5.3.b Add parser-output handoff regression for mapped sequence/smart-pointer method call lowering to canonical pre-generated targets.`

## Design Decision

Use an `AstCodeGen`-level deterministic unit regression under parser-output mapping context, and assert canonical lanes directly in generated Rust.

Why this is the best fit:

- It verifies method/operator lowering behavior without introducing runtime shims.
- It validates both family routing and call-site lowering:
  - sequence (`vector`) method lane
  - smart-pointer (`unique_ptr`) operator lanes
- It keeps checks generic and target-independent.

## Implementation Summary

Added test:

- `test_parser_output_mapping_sequence_smart_pointer_call_lowering_uses_canonical_method_operator_lanes`

Coverage assertions include:

- Mapping-aware canonical routing for sequence/smart-pointer spellings:
  - `vector_int -> std_vector<i32>` (alias closure)
  - `unique_ptr` spellings normalize to `std_unique_ptr<i32>` (alias or direct signature normalization)
- Sequence method lane:
  - generated call includes `.push_back(value)`
- Smart-pointer operator lanes:
  - `operator*` lowers via `.op_deref()`
  - `operator->` lowers via `.op_arrow()`

## Wrong-Approach Compliance

Checked against `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md`.

- No target-specific hacks were introduced.
- No semantic stub/fallback method bodies were introduced.
- No semantic remapping fallback lanes (`std::collections::*`) were introduced for this work.

## Validation

Focused test commands run:

- `cargo test -p fragile-clang test_parser_output_mapping_sequence_smart_pointer_call_lowering_uses_canonical_method_operator_lanes -- --nocapture`
- `cargo test -p fragile-clang test_parser_output_mapping_ -- --nocapture`
