# M4.2.b Unordered-Map Core Surface (2026-03-18)

## Objective

Close TODO leaf `M4.2.b` by adding a concrete pre-generated unordered-map
runtime surface for current `std::unordered_map<int, int>` fixture behavior in
`fragile-stl`.

## Scope and Sizing

This leaf is under 1000 LOC and intentionally narrow:

- add deterministic unordered-map implementation in `fragile-stl`
- add focused runtime tests for insert/lookup/update/erase/collision behavior
- no codegen mapping cutover in this leaf (`M5` remains responsible)

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific conditionals
- no fake hardcoded success-return methods
- no force-native bypasses
- implemented real mutable hash-bucket state transitions (not semantic no-op
  stubs)

## Design

Added:

- `crates/fragile-stl/src/unordered_map.rs`
- `crates/fragile-stl/tests/unordered_map_tests.rs`

`std_unordered_map_int__int` implementation details:

- deterministic fixed-bucket hash storage (16 buckets by default)
- deterministic in-bucket insertion order for collisions
- `op_index` supports insertion-on-miss and mutable slot access
- `insert`/`insert_or_assign` support deterministic update behavior
- `find`/`find_const`/`count`/`contains`/`erase` support lookup and removal
  behavior
- `size`/`empty`/`clear`/`bucket_count` provide container-state operations

This creates a concrete pre-generated `std::unordered_map<int,int>` operation
surface in `fragile-stl` while leaving placeholder-to-module mapping cutover to
`M5`.

## Validation

Focused:

- `cargo test -p fragile-stl --test unordered_map_tests -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Manual

For unordered-map runtime behavior in pre-generated STL surfaces:

1. Use `std_unordered_map_int__int::op_index(key)` for insertion-on-access
   semantics.
2. Use `insert_or_assign(key, value)` when explicit assignment intent is
   clearer.
3. Use `find`/`find_const`/`count`/`contains`/`erase` for lookup/removal
   operations.
4. `as_entries()` exposes deterministic bucket-order snapshots for validation.
