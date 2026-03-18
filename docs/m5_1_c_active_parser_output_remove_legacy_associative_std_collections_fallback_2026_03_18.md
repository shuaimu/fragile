# M5.1.c Remove Active-Path Legacy Associative std::collections Fallback (2026-03-18)

## Objective

Close TODO leaf `M5.1.c` by removing active parser-output handoff fallback
emission of unresolved mapped associative family aliases to
`std::collections::*`.

## Scope and Sizing

This leaf is small (<1000 LOC):

- add parser-output handoff context tracking in `AstCodeGen`
- gate legacy associative fallback lanes in:
  - unresolved type-reference closure
  - missing-stub concrete alias resolution
- refresh deterministic regressions and active handoff test expectations

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches`:

- no semantic fake method-body stubs
- no force-native/target-specific bypass
- no semantic remapping shortcut to hide unresolved mapped-family shapes

Unsupported mapped associative shapes remain explicit unresolved placeholders.

## Design

### 1. Explicit parser-output mapping context flag

Added:

- `parser_output_stl_placeholder_mapping_context_enabled: bool`

This is enabled whenever parser-output handoff calls
`set_parser_output_stl_placeholder_mappings(...)`, including empty mapping
sets. This distinguishes active parser-output context from non-handoff codegen.

### 2. Remove legacy associative fallback in active parser-output context

Introduced a mapped-family matcher for parser-output associative lanes:

- `map_` / `std_map_`
- `unordered_map_` / `std_unordered_map_`

Then enforced in both active fallback surfaces:

1. `resolve_container_alias_target(...)` used by unresolved closure:
   - in active parser-output context, mapped associative names no longer fall
     through to legacy `stl_associative_container_alias_target_from_rust_name`
     (`std::collections::{BTreeMap, HashMap}`).
2. `resolve_missing_stub_concrete_alias_target(...)`:
   - in active parser-output context, mapped associative names no longer route
     through legacy `std::collections::*` fallback aliases.

### 3. Test updates

- Added closure regression for empty active mapping context to ensure mapped
  associative families do not emit `std::collections::*` aliases.
- Added missing-stub regression to prove:
  - baseline non-handoff behavior remains unchanged,
  - active parser-output context blocks legacy associative fallback lanes.
- Updated parser-output handoff integration test to assert active handoff blocks
  legacy associative `std::collections` fallback even when parser-output nodes
  do not include explicit placeholder entries.

## Validation

Focused:

- `cargo test -p fragile-clang close_unresolved_type_reference_gaps_with_empty_parser_output_mapping_context_blocks_legacy_associative_std_collections_aliases -- --nocapture`
- `cargo test -p fragile-clang resolve_missing_stub_concrete_alias_target_with_empty_parser_output_mapping_context_blocks_legacy_associative_std_collections_aliases -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_blocks_legacy_associative_std_collections_alias_lanes -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Notes

1. Active parser-output handoff now treats mapped associative unresolved lanes
   as mapping-controlled and does not silently demote them to
   `std::collections::*`.
2. Non-parser-output codegen behavior for legacy map/unordered_map fallback
   remains unchanged.
