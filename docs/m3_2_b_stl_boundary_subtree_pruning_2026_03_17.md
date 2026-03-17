# M3.2.b STL Boundary Subtree Pruning (2026-03-17)

## Objective

Close TODO leaf `M3.2.b` by pruning AST descendants once a node is emitted as a
known STL boundary placeholder.

## Scope and Sizing

This leaf is under 1000 LOC and scoped to parser-clang flattening behavior and
regression tests:

- stop recursion below emitted STL placeholder boundaries
- keep non-STL lowering behavior unchanged
- add fixture + unit coverage for pruning behavior

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific conditionals
- no force-native bypasses
- no synthesized semantic fallback bodies
- no guessed STL-family fallback for unresolved/ambiguous cases

## Design

### Boundary pruning in flattening

In `crates/fragile-parser-clang/src/lib.rs`:

- `flatten_clang_ast_node` now records whether STL placeholder emission
  occurred for the current node.
- If placeholder emission occurred, recursion returns immediately after
  emitting the node (children are not lowered).
- Non-placeholder nodes continue normal recursive lowering.

### Regression coverage

Unit coverage:

- `flatten_clang_ast_nodes_prunes_descendants_for_stl_placeholder_boundaries`
  verifies:
  - STL boundary `VarDecl` emits placeholder node kind
  - placeholder boundary has no child nodes in flattened output
  - non-STL `VarDecl` still lowers descendants

Fixture coverage updates:

- extended fixture source:
  - `crates/fragile-parser-clang/tests/fixtures/m3_1_d/src/stl_symbol_detection.cpp`
  - added initialized STL boundary vars (`direct_vec_init`,
    `imported_vec_init`)
- extended integration test:
  - `crates/fragile-parser-clang/tests/stl_symbol_detection_fixture_tests.rs`
  - asserts initialized STL boundary placeholders have no lowered descendants

## Validation

Focused:

- `cargo test -p fragile-parser-clang`

Full regression:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

Parser output now treats detected STL boundaries as opaque leaves: once a node
is emitted as an STL placeholder, no deep STL internals are lowered beneath
that node.
