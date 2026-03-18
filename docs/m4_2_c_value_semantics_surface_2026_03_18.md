# M4.2.c Value-Semantics Core Surfaces (2026-03-18)

## Objective

Close TODO leaf `M4.2.c` by adding concrete pre-generated value-semantics
runtime surfaces for fixture-required STL shapes:

- `std::optional<int>`
- `std::tuple<int, int>`
- `std::variant<int, long>`

## Scope and Sizing

This leaf is under 1000 LOC and intentionally narrow:

- implement value-semantics surfaces in `fragile-stl`
- add focused runtime tests for engagement/index/assignment/copy semantics
- no codegen placeholder mapping cutover in this leaf (`M5` remains
  responsible)

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific conditionals
- no fake hardcoded success-return methods
- no force-native bypasses
- implemented real mutable state/value transitions

## Design

Updated:

- `crates/fragile-stl/src/comparison.rs`

Added runtime surfaces:

- `std_optional_int`:
  - engaged flag and value storage
  - `has_value`/`op_bool`/`value_or`
  - `emplace`/`assign`/`reset`
  - pointer and dereference accessors
- `std_tuple_int__int`:
  - direct two-field value storage
  - constructors, element getters, assignment
- `std_variant_int__long`:
  - active-alternative index and dual typed storage
  - constructors, `index`, holds checks
  - `emplace_*` and typed accessors

Added focused tests:

- `crates/fragile-stl/tests/value_semantics_tests.rs`

This preserves deterministic value semantics and keeps mapping cutover to `M5`.

## Validation

Focused:

- `cargo test -p fragile-stl --test value_semantics_tests -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Manual

For pre-generated value-semantics surfaces:

1. Use `std_optional_int` for nullable `int` value storage with explicit
   engagement checks.
2. Use `std_tuple_int__int` for deterministic two-element value aggregation.
3. Use `std_variant_int__long` with `index()` and typed accessors to manage the
   active alternative.
