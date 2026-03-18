# M4.2.a Ordered-Map Core Surface (2026-03-17)

## Objective

Close TODO leaf `M4.2.a` by adding a concrete pre-generated ordered-map runtime
surface for current `std::map<int, int>` fixture behavior in `fragile-stl`.

## Scope and Sizing

This leaf is under 1000 LOC and intentionally narrow:

- add deterministic ordered-map implementation in `fragile-stl`
- add focused runtime tests for insert/lookup/update/erase/order semantics
- no codegen mapping cutover in this leaf (`M5` remains responsible)

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific conditionals
- no fake hardcoded success-return methods
- no force-native bypasses
- implemented real mutable map state transitions (not semantic no-op stubs)

## Design

Added:

- `crates/fragile-stl/src/ordered_map.rs`
- `crates/fragile-stl/tests/ordered_map_tests.rs`

`std_map_int__int` implementation details:

- deterministic ordered storage (`Vec<std_pair_int__int>` sorted by key)
- `op_index` supports insertion-on-miss and mutable slot access
- `insert_or_assign` supports deterministic update behavior
- `find`/`count`/`erase` support lookup and removal behavior
- `size`/`empty`/`clear` and `begin`/`end` container boundary operations

This creates a concrete pre-generated `std::map<int,int>` operation surface in
`fragile-stl` while leaving placeholder-to-module mapping cutover to `M5`.

## Validation

Focused:

- `cargo test -p fragile-stl --test ordered_map_tests -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Manual

For ordered-map runtime behavior in pre-generated STL surfaces:

1. Use `std_map_int__int::op_index(key)` for insertion-on-access semantics.
2. Use `insert_or_assign(key, value)` when explicit assignment intent is clearer.
3. Use `find`/`count`/`erase` for lookup/removal operations.
4. Expect deterministic sorted key order in `as_slice()` for verification paths.
