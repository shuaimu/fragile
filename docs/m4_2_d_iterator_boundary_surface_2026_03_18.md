# M4.2.d Iterator-Boundary Core Surface (2026-03-18)

## Objective

Close TODO leaf `M4.2.d` by replacing no-op iterator boundary helpers with
concrete pre-generated runtime behavior for rbtree traversal:

- `_Rb_tree_increment`
- `_Rb_tree_decrement`

## Scope and Sizing

This leaf is under 1000 LOC and intentionally narrow:

- implement real successor/predecessor boundary traversal in `fragile-stl`
- add focused runtime regressions for boundary behavior
- no codegen placeholder mapping cutover in this leaf (`M5` remains
  responsible)

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific conditionals
- no fake/no-op fallback iterator bodies
- no force-native bypasses
- implemented real pointer-chain traversal semantics

## Design

Updated:

- `crates/fragile-stl/src/io.rs`

Implementation notes:

- `_Rb_tree_increment` now performs real in-order successor traversal using
  right-subtree minimum search plus ancestor ascent.
- `_Rb_tree_decrement` now performs real in-order predecessor traversal using
  left-subtree maximum search plus ancestor ascent.
- Header/sentinel detection is implemented for decrement boundary behavior
  (`decrement(end)` returns rightmost when header invariants hold).
- Null/orphan parent chains return null deterministically.

Added focused tests:

- `crates/fragile-stl/tests/iterator_boundary_tests.rs`

This removes semantic no-op behavior for iterator boundaries while keeping
placeholder mapping cutover in `M5`.

## Validation

Focused:

- `cargo test -p fragile-stl --test iterator_boundary_tests`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Manual

For pre-generated iterator boundary helpers:

1. Use `_Rb_tree_increment` to advance to in-order successor; rightmost advances
   to the sentinel/header boundary.
2. Use `_Rb_tree_decrement` to move to in-order predecessor; decrementing
   sentinel/header returns rightmost.
3. Null pointers or orphaned nodes return null deterministically.
