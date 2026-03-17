# M3.1.d Deterministic STL Symbol Detection Fixtures (2026-03-17)

## Objective

Close TODO leaf `M3.1.d` by adding deterministic regression fixtures/tests for:

- direct canonical `std::` family detection
- typedef/type-alias STL chain resolution
- using declaration/directive chain STL resolution

## Scope and Sizing

This leaf is below 1000 LOC and constrained to parser/test layers:

- add parser-clang fixture source for STL alias/use patterns
- add integration tests that parse fixtures and assert deterministic outputs
- fix `UsingDeclaration` qualified-name extraction in `fragile-clang` so fixture
  behavior matches real namespace-qualified C++ using declarations
- add focused parse-unit coverage for the `UsingDeclaration` extraction fix

## Wrong-Approach Check

Reviewed before changes:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific hacks
- no force-native bypasses
- no fake semantic stubs/fallback bodies
- ambiguous/invalid fixture code paths are not masked with synthetic behavior

## Design

### Fixture-based regression coverage

Added fixture source:

- `crates/fragile-parser-clang/tests/fixtures/m3_1_d/src/stl_symbol_detection.cpp`

Added integration test:

- `crates/fragile-parser-clang/tests/stl_symbol_detection_fixture_tests.rs`

Coverage assertions:

1. backend parse output is deterministic across repeated runs for fixture input
2. direct detector observes expected families (`vector`, `map`, `optional`)
3. alias-table extraction deterministically resolves canonical targets across:
   - direct `std::...` aliases
   - typedef/type-alias chains
   - using declaration/directive chains

### Upstream parser extraction fix (required by fixture behavior)

`fragile-clang` `UsingDeclaration` extraction previously dropped namespace
segments in common cases (for example yielding `["Bar"]` instead of
`["foo","Bar"]`).

Updated:

- `ClangParser::get_qualified_name` in `crates/fragile-clang/src/parse.rs`

Behavior:

- primary path extracts namespace refs and declaration leaf from using-decl
  children
- fallback paths use referenced cursor and spelling split when needed

Added parser unit tests:

- `test_parse_using_declaration_keeps_qualified_leaf_name`
- `test_parse_using_declaration_keeps_nested_qualified_leaf_name`

## Validation

Focused:

- `cargo test -p fragile-clang test_parse_using_declaration`
- `cargo test -p fragile-parser-clang`

Full regression:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

`fragile-parser-clang` STL symbol-detection behavior now has fixture-backed
deterministic regression coverage that validates direct+typedef+using STL
resolution paths against real parsed C++ source.
