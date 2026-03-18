# M5.1.b.iv Active Parser-Output Legacy Associative Fallback Guard (2026-03-18)

## Objective

Close TODO leaf `M5.1.b.iv` by adding deterministic active parser-output
handoff coverage proving mapping-controlled associative families no longer use
legacy `std::collections::*` alias fallback lanes.

## Scope and Sizing

This leaf is small (<1000 LOC). It adds one end-to-end parser-output handoff
test and documentation updates only.

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches`:

- no synthetic semantic method bodies or fake behavior
- no target-specific bypasses
- no semantic type remapping shortcuts for unsupported mapped lanes

Unsupported mapping-controlled associative shapes remain explicit unresolved
placeholders instead of silently falling back to `std::collections::*`.

## Design

Added a parser-output handoff test in `crates/fragile-clang/src/lib.rs`:

- writes a tiny C++ fixture with unresolved lowered associative spellings:
  - `map_unsigned_int__bool`
  - `unordered_map_unsigned_int__bool`
- runs parser-output handoff twice on the same fixture:
  - baseline without STL placeholder mappings
  - mapping-controlled with `stl_map_placeholder` and
    `stl_unordered_map_placeholder`
- asserts:
  - baseline still uses legacy `std::collections::{BTreeMap, HashMap}` alias
    lanes for those unresolved lowered names
  - mapped run suppresses those legacy alias lanes
  - mapped run keeps unsupported mapped associative shapes explicit as
    unresolved placeholder structs

This validates the active parser-output run path (not just direct closure
helpers) for `M5.1.b.iv`.

## Validation

Focused:

- `cargo test -p fragile-clang parser_output_codegen_mapping_blocks_legacy_associative_std_collections_alias_lanes -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_uses_handoff_metadata_without_libtooling_export -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Notes

1. Keep parser-output placeholder node kinds and mapping keys in sync so
   mapping-controlled family suppression remains deterministic.
2. Expand this active-path guard pattern when adding new mapped families that
   currently have legacy fallback lanes.
