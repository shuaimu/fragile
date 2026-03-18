# M5.A1.b Mapping-Completeness Coverage for `string`/`optional`/`variant`/`tuple` (2026-03-18)

## Objective

Close TODO leaf `M5.A1.b` by extending mapped-family completeness enforcement to
the remaining mapped placeholder families:

- `string`
- `optional`
- `variant`
- `tuple`

## Scope and Sizing

`M5.A1` is broader than one patch, but `M5.A1.b` is small (<1000 LOC):

- extend covered-family alias-prefix dispatch for four families
- add deterministic positive/negative mapping-completeness regressions
- add active parser-output handoff negative regression
- update docs and TODO status

## Wrong-Approach Check

Reviewed:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches`
- `docs/dev/wrong.md`

Compliance:

- no target-specific hacks
- no force-native bypasses
- no fake semantic fallback method bodies

## Design and Implementation

### 1. Covered-family mapping prefix expansion

Updated `PARSER_OUTPUT_MAPPED_FAMILY_ALIAS_PREFIX_SPECS` in
`crates/fragile-clang/src/lib.rs` to include:

- `stl_string_placeholder` -> `string` family prefixes:
  - `string_`, `std_string_`, `basic_string_`, `std_basic_string_`
- `stl_optional_placeholder` -> `optional` family prefixes:
  - `optional_`, `std_optional_`
- `stl_variant_placeholder` -> `variant` family prefixes:
  - `variant_`, `std_variant_`
- `stl_tuple_placeholder` -> `tuple` family prefixes:
  - `tuple_`, `std_tuple_`

### 2. Deterministic positive/negative coverage

Added/extended tests in `crates/fragile-clang/src/lib.rs`:

- Positive canonical acceptance:
  - `parser_output_mapping_completeness_validation_allows_canonical_covered_alias_targets`
  now includes canonical aliases for `basic_string_char`, `optional_int`,
  `variant_int__long`, `tuple_int__int`.
- Negative non-canonical alias targets:
  - `parser_output_mapping_completeness_validation_rejects_noncanonical_string_optional_variant_tuple_alias_targets`
- Negative unresolved placeholder structs:
  - `parser_output_mapping_completeness_validation_rejects_string_optional_variant_tuple_placeholder_structs`

### 3. Active handoff integration regression

Added deterministic active parser-output handoff negative test:

- `parser_output_codegen_active_handoff_mapped_string_optional_variant_tuple_unresolved_shapes_fail_mapping_completeness`

The regression verifies mapped handoff fails with explicit completeness errors
when unresolved family-lowered placeholders would otherwise leak through.

## Validation

Focused:

- `cargo test -p fragile-clang parser_output_mapping_completeness_validation_allows_canonical_covered_alias_targets -- --nocapture`
- `cargo test -p fragile-clang parser_output_mapping_completeness_validation_rejects_noncanonical_string_optional_variant_tuple_alias_targets -- --nocapture`
- `cargo test -p fragile-clang parser_output_mapping_completeness_validation_rejects_string_optional_variant_tuple_placeholder_structs -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_mapped_string_optional_variant_tuple_unresolved_shapes_fail_mapping_completeness -- --nocapture`
- `cargo test -p fragile-clang test_parser_output_mapping_ -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_ -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
