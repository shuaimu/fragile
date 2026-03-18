# M5.1.b.iii Mapping-Driven Sequence/Smart-Pointer Dispatch (2026-03-18)

## Objective

Close TODO leaf `M5.1.b.iii` by replacing hardcoded sequence/smart-pointer
alias family detection with mapping-driven dispatch for covered parser-output
placeholder families.

## Scope and Sizing

This leaf stays below 1000 LOC and targets one bounded codegen update:

- add parser-output mapping-driven family dispatch for:
  - `vector` (`stl_vector_placeholder`)
  - `shared_ptr` (`stl_shared_ptr_placeholder`)
  - `unique_ptr` (`stl_unique_ptr_placeholder`)
- apply dispatch in both:
  - unresolved type closure alias resolution
  - missing-stub concrete alias target resolution
- preserve legacy behavior when parser-output mapping context is absent

Out of scope:

- remaining mapped-family diagnostics/guardrail coverage expansion (`M5.1.b.iv`)
- complete legacy fallback lane removal (`M5.1.c`)

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches`:

- no target-specific conditionals
- no force-native bypass
- no synthetic semantic fallback method bodies
- unsupported mapped sequence/smart-pointer shapes remain explicit unresolved
  placeholders when mapping-controlled lanes cannot resolve

## Design

### 1. Mapping-driven family dispatch specs

Added dispatch tables in `AstCodeGen` for mapping-controlled
sequence/smart-pointer families:

- `stl_vector_placeholder` -> `vector_` / `std_vector_`
- `stl_shared_ptr_placeholder` -> `shared_ptr_` / `std_shared_ptr_`
- `stl_unique_ptr_placeholder` -> `unique_ptr_` / `std_unique_ptr_`

Family matching now resolves through these specs instead of hardcoded
branch checks.

### 2. Mapping-driven alias target derivation

Added `parser_output_sequence_smart_pointer_alias_target_from_rust_name(...)`:

1. match mapped family via dispatch table,
2. read canonical type prefix from parser-output mappings,
3. parse lowered suffix element lane,
4. build canonical alias target: `<mapped_prefix><element>`.

### 3. Active-path wiring

Updated active alias-resolution call paths:

- `resolve_container_alias_target(...)`
  - now attempts mapping-driven sequence/smart-pointer alias derivation
  - mapping-controlled sequence/smart-pointer families no longer fall back to
    legacy hardcoded family detection lanes
- `resolve_missing_stub_concrete_alias_target(...)`
  - now prefers mapping-driven sequence/smart-pointer alias derivation
  - legacy unqualified-vector + generic container fallback is skipped for
    mapping-controlled sequence/smart-pointer families

Legacy behavior for non-mapped runs (empty parser-output mappings) remains
unchanged.

## Validation

Focused tests:

- `cargo test -p fragile-clang close_unresolved_type_reference_gaps_with_placeholder_mapping -- --nocapture`
- `cargo test -p fragile-clang resolve_missing_stub_concrete_alias_target_prefers_mapping_driven -- --nocapture`
- `cargo test -p fragile-clang parser_output_stl_placeholder_mapping -- --nocapture`

Added tests:

- `test_close_unresolved_type_reference_gaps_with_placeholder_mapping_dispatches_vector_family_via_mapping_prefix`
- `test_close_unresolved_type_reference_gaps_with_placeholder_mapping_dispatches_shared_ptr_family_via_mapping_prefix`
- `test_resolve_missing_stub_concrete_alias_target_prefers_mapping_driven_vector_prefix`
- `test_resolve_missing_stub_concrete_alias_target_prefers_mapping_driven_unique_ptr_prefix`

## User Notes

1. Keep parser-output placeholder-kind keys and dispatch specs aligned.
2. When new pre-generated sequence/smart-pointer families are added, extend the
   dispatch table before removing any legacy fallback lanes.
3. Preserve dual-path coverage:
   - mapping-aware dispatch behavior,
   - legacy behavior without parser-output mapping context.
