# M1.1 ParserOutput v1 Schema (2026-03-17)

## Objective

Close TODO leaf `M1.1` by defining a concrete `ParserOutput v1` schema with
explicit STL placeholder node kinds.

## Scope and Sizing

This leaf is bounded well below 1000 LOC:

- add machine-readable schema:
  `docs/schemas/parser_output_v1.schema.json`
- add canonical fixture corpus entry:
  `docs/fixtures/parser_output_v1_full_placeholders.json`
- add focused schema/fixture regressions:
  `tests/python/test_parser_output_v1_schema.py`
- mark `M1.1` complete in `TODO.md`

No additional decomposition is required.

## Wrong-Approach Check

Reviewed:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This leaf intentionally avoids forbidden behavior:

- no target-specific hacks
- no force-native bypasses
- no fake semantic fallback/stub method bodies

## Design

`ParserOutput v1` is defined as a strict JSON schema with:

- pinned version marker (`schema_version = 1.0.0`)
- translation-unit metadata block
- node list with explicit `node_kind` enum
- diagnostic list

Explicit STL placeholder node kinds in schema:

- `stl_vector_placeholder`
- `stl_map_placeholder`
- `stl_unordered_map_placeholder`
- `stl_string_placeholder`
- `stl_optional_placeholder`
- `stl_variant_placeholder`
- `stl_tuple_placeholder`
- `stl_shared_ptr_placeholder`
- `stl_unique_ptr_placeholder`

Canonical fixture covers all placeholder kinds and maps each to its declared
`stl_placeholder.family`.

## Validation

Focused:

- `python3 -m unittest tests/python/test_parser_output_v1_schema.py -v`

Full Python:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Full workspace:

- `cargo test --workspace --all-targets`

## User Manual

Schema and canonical fixture are checked in at:

- `docs/schemas/parser_output_v1.schema.json`
- `docs/fixtures/parser_output_v1_full_placeholders.json`

Use the fixture as the reference shape when implementing `M1.2` metadata
expansion and `M1.3` deterministic serialization/round-trip tests.
