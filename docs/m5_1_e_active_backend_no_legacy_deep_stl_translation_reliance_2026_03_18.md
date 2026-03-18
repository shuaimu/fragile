# M5.1.e Active Backend No Legacy Deep STL Translation Reliance (2026-03-18)

## Objective

Close TODO leaf `M5.1.e` by validating active parser-output handoff backend runs
do not rely on legacy deep STL translation path lanes for covered placeholder
families.

## Scope and Sizing

This leaf is small (<1000 LOC):

- add deterministic post-codegen validation in parser-output handoff path
- add focused regression tests for validation behavior
- add integration coverage for supported covered associative family aliasing

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches`:

- no synthetic semantic method-body stubs
- no force-native bypasses
- no target-specific special-case hacks
- no silent fallback acceptance for covered-family legacy deep STL lanes

## Design

### 1. Active handoff validation gate

Added a deterministic validation gate in
`transpile_parser_output_to_rust_with_options(...)` after codegen emission:

- `validate_parser_output_handoff_no_legacy_deep_stl_translation_path_for_covered_families(...)`

The gate inspects generated type aliases and fails if covered parser-output
families resolve through legacy deep STL fallback alias lanes.

### 2. Covered-family violation detector

Added alias detector helpers in `fragile-clang` handoff surface:

- `parser_output_type_alias_binding_from_line(...)`
- `parser_output_first_legacy_deep_stl_alias_violation(...)`
- `parser_output_legacy_deep_stl_translation_path_violations_for_covered_families(...)`

Detection is prefix-indexed (`match_indices` over associative alias prefixes)
and validates only matched alias lines, avoiding whole-output per-line
allocation scans on very large generated handoff outputs.

Current covered-family checks enforce:

- when `stl_map_placeholder` is covered, no `map_*` / `std_map_*` alias may
  resolve to `std::collections::BTreeMap<...>`
- when `stl_unordered_map_placeholder` is covered, no
  `unordered_map_*` / `std_unordered_map_*` alias may resolve to
  `std::collections::HashMap<...>`

### 3. Regression coverage

Added unit validation tests:

- reject covered-family legacy deep STL alias fallback shapes
- allow non-covered-family alias shapes

Added active handoff integration coverage for supported covered associative
families (`map_int__int`, `unordered_map_int__int`) proving canonical
pre-generated alias targets are used and legacy `std::collections` fallback
aliases are not used.

## Validation

Focused:

- `cargo test -p fragile-clang parser_output_legacy_deep_stl_translation_path_validation_rejects_covered_fallback_aliases -- --nocapture`
- `cargo test -p fragile-clang parser_output_legacy_deep_stl_translation_path_validation_allows_noncovered_aliases -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_mapped_associative_supported_families_use_pre_generated_alias_targets -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_blocks_legacy_associative_std_collections_alias_lanes -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Notes

1. Active parser-output handoff now contains an explicit validator preventing
   covered-family regressions back to legacy deep STL fallback lanes.
2. Covered-family legacy deep STL fallback reliance is surfaced as a
   deterministic error instead of silently producing fallback aliases.
