# M3.2.a STL Boundary Placeholder Node-Kind Emission (2026-03-17)

## Objective

Close TODO leaf `M3.2.a` by emitting canonical STL placeholder `node_kind`
values when parser-clang encounters STL boundary declarations/expressions
detected via direct `std::` symbols and alias/using-aware resolution.

## Scope and Sizing

This leaf is below 1000 LOC and scoped to parser-clang lowering plus fixture
coverage:

- thread STL symbol-resolution context into AST flattening
- resolve canonical STL family per node using direct + alias/using-aware paths
- emit canonical placeholder node kinds at STL boundaries
- add deterministic fixture assertions for emitted placeholder kinds

Deep-subtree pruning is explicitly left for follow-up `M3.2.b`.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific conditionals
- no force-native bypasses
- no synthesized semantic fallback bodies
- unresolved/ambiguous STL cases remain unresolved rather than guessed

## Design

### Boundary-aware node-kind mapping

In `crates/fragile-parser-clang/src/lib.rs`:

- built a `StlResolutionContext` once from translation-unit nodes
- updated flattening to carry namespace scope path and resolve per-node STL
  family from:
  - direct `std::...` detection
  - alias symbol-table matches
  - using declaration/directive visible candidates
- added boundary mapper:
  - `map_parser_node_kind_with_stl_boundary`
  - emits canonical placeholder node kinds when family is resolved
  - falls back to default kind mapping otherwise

Canonical placeholder kinds emitted:

- `stl_vector_placeholder`
- `stl_map_placeholder`
- `stl_unordered_map_placeholder`
- `stl_string_placeholder`
- `stl_optional_placeholder`
- `stl_variant_placeholder`
- `stl_tuple_placeholder`
- `stl_shared_ptr_placeholder`
- `stl_unique_ptr_placeholder`

### Fixture regression extension

Updated fixture:

- `crates/fragile-parser-clang/tests/fixtures/m3_1_d/src/stl_symbol_detection.cpp`

Added test coverage:

- `crates/fragile-parser-clang/tests/stl_symbol_detection_fixture_tests.rs`

Assertions validate deterministic placeholder kind emission for direct and
alias/using-transit boundary variables.

## Validation

Focused:

- `cargo test -p fragile-parser-clang`

Full regression:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

`fragile-parser-clang` now emits canonical STL placeholder `node_kind` values
at detected STL boundaries (including alias/using-introduced boundaries),
providing stable parser-output contracts for downstream placeholder mapping.
