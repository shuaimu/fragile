# M5.2 Mapping Completeness Checks (2026-03-18)

## Objective

Close TODO leaf `M5.2` by enforcing active parser-output handoff mapping
completeness for covered STL placeholder families so covered-family lowering does
not silently fall back away from mapped canonical targets.

## Scope and Sizing

This leaf is small (<1000 LOC):

- add post-codegen completeness validator in parser-output handoff
- add focused unit/integration coverage for fail/pass paths
- update docs and TODO status

## Wrong-Approach Check

Reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches` before
implementation:

- no target-specific hacks
- no force-native bypasses
- no fake semantic method-body stubs
- no silent fallback acceptance for covered-family mapping gaps

## Design

### 1. Active handoff completeness gate

Added a second deterministic active handoff validator in
`transpile_parser_output_to_rust_with_options(...)` after codegen emission:

- `validate_parser_output_handoff_mapping_completeness_for_covered_families(...)`

### 2. Covered-family completeness detector

Added helper flow in `fragile-clang`:

- `parser_output_struct_name_from_line(...)`
- `parser_output_covered_family_spec_for_lowered_name(...)`
- `parser_output_mapping_completeness_violations_for_covered_families(...)`

Validator behavior for covered families (present in observed placeholder mapping
set):

- Reject covered-family alias lines that resolve to non-canonical targets.
- Reject covered-family unresolved placeholder-struct blocks emitted by final
  unresolved-type closure path.
- Do not flag canonical mapped target definitions themselves (`std_map_*`,
  `std_unordered_map_*`, etc.) when those names appear as emitted type items.

### 3. Updated integration expectation

For mapped associative families with unsupported concrete suffixes, active
parser-output handoff now fails with mapping-completeness diagnostics instead of
silently succeeding with unresolved placeholder structs.

## Validation

Focused:

- `cargo test -p fragile-clang parser_output_mapping_completeness -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_blocks_legacy_associative_std_collections_alias_lanes -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_mapped_associative_supported_families_use_pre_generated_alias_targets -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Notes

1. Active parser-output handoff now enforces canonical mapping completeness for
   covered mapped families.
2. Covered-family unresolved placeholder closure in active handoff is now a
   deterministic error path.
