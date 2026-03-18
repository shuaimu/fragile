# M5.1.b.ii Mapping-Driven Associative Alias Dispatch (2026-03-18)

## Objective

Close TODO leaf `M5.1.b.ii` by replacing hardcoded parser-output associative
alias target derivation with mapping-driven family dispatch for covered
placeholder families (`map`, `unordered_map`).

## Scope and Sizing

This leaf stays below 1000 LOC and targets one bounded replacement:

- remove hardcoded parser-output map/unordered-map branch logic in associative
  alias-target derivation
- drive family matching from a dispatch table keyed by parser-output placeholder
  node kinds and accepted lowered prefixes
- keep concrete lane policy unchanged (`int/int` mapped pre-generated surfaces)
- preserve legacy non-mapping behavior when parser-output mappings are absent

Out of scope:

- sequence/smart-pointer family dispatch replacement (`M5.1.b.iii`)
- full removal of legacy fallback lanes (`M5.1.c`)

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches`:

- no target-specific conditionals
- no force-native bypass
- no fake semantic fallback method bodies
- unsupported mapped associative shapes remain explicit unresolved placeholders

## Design

### 1. Associative family dispatch table

Introduced parser-output associative dispatch specs in `AstCodeGen`:

- placeholder kind + lowered-prefix pairs:
  - `stl_map_placeholder` -> `map_` / `std_map_`
  - `stl_unordered_map_placeholder` -> `unordered_map_` / `std_unordered_map_`
- family control detection now resolves through this dispatch matcher instead of
  hardcoded per-family checks.

### 2. Mapping-driven alias target derivation

`parser_output_associative_alias_target_from_rust_name(...)` now:

1. resolves the mapped family via dispatch matcher,
2. fetches canonical prefix from parser-output mapping,
3. builds concrete mapped alias target using supported suffix policy.

Current supported concrete mapped suffix remains conservative:

- `int/int` -> `*_int__int`

### 3. Behavior invariants

- With parser-output mapping context:
  - mapped associative families are mapping-controlled,
  - alias target prefixes are sourced from mapping entries (not hardcoded),
  - unsupported mapped shapes do not silently fall back to
    `std::collections::*` lanes.
- Without mapping context:
  - existing legacy associative alias behavior remains unchanged.

## Validation

Focused tests:

- `cargo test -p fragile-clang close_unresolved_type_reference_gaps_with_placeholder_mapping -- --nocapture`
- `cargo test -p fragile-clang parser_output_stl_placeholder_mapping -- --nocapture`

Added tests:

- `test_close_unresolved_type_reference_gaps_with_placeholder_mapping_dispatches_map_family_via_mapping_prefix`
- `test_close_unresolved_type_reference_gaps_with_placeholder_mapping_dispatches_unordered_map_family_via_mapping_prefix`

These assert parser-output mapping prefixes are honored for associative families
and that hardcoded `std_map`/`std_unordered_map` prefixes are not assumed.

## User Notes

1. Keep parser-output placeholder kind keys and dispatch spec table aligned.
2. Extend supported associative concrete suffixes explicitly as pre-generated
   surfaces become available.
3. Maintain dual-path tests:
   - mapping-aware dispatch lane,
   - legacy lane without parser-output mappings.
