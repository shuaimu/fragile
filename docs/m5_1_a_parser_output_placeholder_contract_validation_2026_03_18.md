# M5.1.a Parser-Output STL Placeholder Contract Validation (2026-03-18)

## Objective

Close TODO leaf `M5.1.a` by adding deterministic parser-output STL placeholder
family resolution and pre-generated contract mapping validation in the
parser-output handoff codegen path.

## Scope and Sizing

This leaf is below 1000 LOC and intentionally narrow:

- add a canonical node-kind -> placeholder-family resolver for parser-output STL placeholders
- validate that each seen placeholder family resolves to a pre-generated STL family contract entry
- fail fast on unknown placeholder node kinds
- add focused unit tests for successful resolution and deterministic failure

Out of scope for this leaf:

- replacing existing codegen lowering heuristics with mapping-driven lowering (`M5.1.b`)
- removing legacy unresolved STL alias fallback emission (`M5.1.c`)
- mapping manifest emission in generated output (`M5.1.d`)

## Wrong-Approach Check

Reviewed and enforced against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`:

- no target-specific hacks
- no force-native bypass
- no fake semantic fallback methods/bodies
- no silent acceptance of unknown placeholder kinds

The implementation chooses deterministic error reporting instead of fallback behavior.

## Design

### 1. Canonical placeholder kind mapping

Introduced a single mapping table in `fragile-clang` handoff code:

- `stl_vector_placeholder` -> `vector`
- `stl_map_placeholder` -> `map`
- `stl_unordered_map_placeholder` -> `unordered_map`
- `stl_string_placeholder` -> `string`
- `stl_optional_placeholder` -> `optional`
- `stl_variant_placeholder` -> `variant`
- `stl_tuple_placeholder` -> `tuple`
- `stl_shared_ptr_placeholder` -> `shared_ptr`
- `stl_unique_ptr_placeholder` -> `unique_ptr`

### 2. Contract-backed resolution

The parser-output handoff now resolves each seen STL placeholder node kind to its
family, then resolves that family through
`pre_generated_stl_family_contract_entry_v1(...)`.

If a node kind is unknown, transpilation fails with a deterministic error that
includes the unsupported node kind and supported set.

If a known family has no contract entry, transpilation fails with a deterministic
missing-contract error.

### 3. Active-path wiring

`transpile_parser_output_to_rust_with_options(...)` now runs this mapping
validation before initiating the libclang reparse/codegen stage.

This ensures placeholder mapping integrity is checked at the handoff boundary.

## Validation

Focused tests run:

- `cargo test -p fragile-clang parser_output_stl_placeholder_mapping -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_rejects_unknown_stl_placeholder_kind_before_parse -- --nocapture`

Added tests:

- resolve known placeholder kinds to canonical pre-generated prefixes
- reject unknown placeholder kind with deterministic diagnostics
- reject unknown placeholder kind in parser-output handoff before parse stage

## User Notes

For new STL placeholder families:

1. Add parser placeholder node kind emission.
2. Add node-kind -> family mapping in `fragile-clang` handoff mapping table.
3. Add matching `fragile-stl` family contract entry in layout contract.
4. Add/adjust tests for both success mapping and unknown-kind rejection.
