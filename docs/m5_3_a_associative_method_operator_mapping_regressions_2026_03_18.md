# M5.3.a Associative Method/Operator Mapping Regressions (2026-03-18)

## Context

Top unfinished P0 milestone task: `M5.3 Add focused regressions for method/operator mapping correctness`.

## Scope Sizing

Estimated implementation size for the first leaf is small (<1000 LOC):

- `TODO.md` leaf expansion + status update: ~5 LOC.
- Focused regression test in `crates/fragile-clang/src/ast_codegen.rs`: ~180 LOC.
- Dev-book/design notes: <100 LOC.

No additional decomposition was required for `M5.3.a` itself.

## Leaf Breakdown

`M5.3` was expanded into:

- `M5.3.a` mapped associative operator/method call lowering regressions.
- `M5.3.b` mapped sequence/smart-pointer method call lowering regressions.
- `M5.3.c` deterministic failure regression when mapped method/operator lanes would require unresolved fallback.

Executed leaf in this iteration: `M5.3.a`.

## Design Decision

Add regression coverage at `AstCodeGen` level with parser-output mapping context enabled, instead of introducing new runtime shims or fallback logic.

Why this is the best fit for `M5.3.a`:

- It directly exercises operator/method call lowering (`operator[]` -> `op_index`, `insert_or_assign` lane).
- It verifies mapping-aware alias closure to canonical pre-generated target (`map_int__int -> std_map_int__int`).
- It is deterministic and isolated from target-specific project behavior.

## Implementation Summary

Added test:

- `test_parser_output_mapping_associative_call_lowering_uses_canonical_operator_and_method_lanes`

Assertions cover:

- Canonical mapping-aware alias closure is present:
  - `pub type map_int__int = std_map_int__int;`
- Operator lane lowering uses canonical method form:
  - `.op_index(key)`
- Method lane remains canonical pre-generated surface:
  - `.insert_or_assign(key, value)`
- No leaked C++ operator spellings:
  - `operator[]` absent in emitted Rust call-sites
- No legacy deep STL associative fallback lane:
  - `std::collections::BTreeMap` absent

## Wrong-Approach Compliance

Checked against `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md`.

- No target-specific hacks were added.
- No semantic stubs/fake method bodies were added.
- No semantic mapping fallback (`std::map -> BTreeMap`) was introduced.

## Validation

Focused test commands run:

- `cargo test -p fragile-clang test_parser_output_mapping_associative_call_lowering_uses_canonical_operator_and_method_lanes -- --nocapture`
- `cargo test -p fragile-clang test_close_unresolved_type_reference_gaps_with_placeholder_mapping -- --nocapture`
