# M5.1.d Active Codegen Mapping Manifest Emission (2026-03-18)

## Objective

Close TODO leaf `M5.1.d` by emitting a deterministic mapping manifest from the
active parser-output codegen path for placeholder families observed in parser
output.

## Scope and Sizing

This leaf is small (<1000 LOC):

- emit a parser-output-only mapping manifest block in generated preamble
- keep non-handoff generation unchanged
- add deterministic unit/integration regressions for manifest behavior

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches`:

- no semantic fake method-body stubs
- no force-native bypasses
- no target-specific branch hacks
- no silent fallback masking for unresolved mapped-family behavior

## Design

### 1. Active handoff-only manifest emission

`AstCodeGen` now emits a parser-output mapping manifest block only when
`set_parser_output_stl_placeholder_mappings(...)` has enabled parser-output
mapping context.

Non-parser-output codegen runs do not emit this block.

### 2. Deterministic manifest schema

Manifest lines are emitted in deterministic order using the existing
`BTreeMap` placeholder mapping state.

Block fields:

- `parser_output_mapping_context_enabled=true`
- `parser_output_observed_family_count=<N>`
- explicit empty marker when `N=0`
- per observed family entries:
  - `placeholder_kind`
  - `canonical_type_prefix`

Observed family labels are derived from placeholder node kinds using
`stl_<family>_placeholder` -> `<family>`.

### 3. Regression coverage

Added `AstCodeGen` preamble tests for:

- no manifest emission outside parser-output context
- empty manifest emission in active handoff context with no observed families
- deterministic ordered entries for multiple observed families

Updated active parser-output handoff integration coverage in
`crates/fragile-clang/src/lib.rs` to assert manifest summary and per-family
entries for both empty and populated observed-family sets.

## Validation

Focused:

- `cargo test -p fragile-clang test_preamble_emits_empty_parser_output_placeholder_mapping_manifest_in_handoff_context -- --nocapture`
- `cargo test -p fragile-clang test_preamble_emits_deterministic_parser_output_placeholder_mapping_manifest_entries -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_blocks_legacy_associative_std_collections_alias_lanes -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Notes

1. The manifest is emitted only for active parser-output handoff runs.
2. Empty observed-family sets are explicit and deterministic (`<none>` marker).
3. Family entry ordering is stable across runs because the manifest is sourced
   from `BTreeMap` mapping state.
