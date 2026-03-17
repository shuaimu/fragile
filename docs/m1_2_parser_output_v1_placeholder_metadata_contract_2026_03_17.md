# M1.2 ParserOutput v1 Placeholder Metadata Contract (2026-03-17)

## Objective

Close TODO leaf `M1.2` by defining a concrete placeholder metadata contract in
`ParserOutput v1` for:

- container family
- element/key/value type shape
- allocator/comparator/hash/equal policy shape
- method/operator selector

## Scope and Sizing

This leaf is bounded below 1000 LOC:

- extend schema:
  `docs/schemas/parser_output_v1.schema.json`
- enrich canonical fixture metadata:
  `docs/fixtures/parser_output_v1_full_placeholders.json`
- extend focused regressions:
  `tests/python/test_parser_output_v1_schema.py`
- mark `M1.2` complete in `TODO.md`

No additional decomposition is required.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This leaf remains contract-only and does not introduce forbidden behaviors:

- no target-specific parser/codegen hacks
- no force-native escape usage
- no fake semantic method bodies

## Design

`stl_placeholder` now requires:

- `type_shape`:
  - `element_type`
  - `key_type`
  - `value_type`
  - `element_types` (for tuple/variant-like families)
- `policy_shape`:
  - `allocator`
  - `comparator`
  - `hash`
  - `equal`
  - each policy entry carries `{ shape, cpp_type }`
- `operation_selector`:
  - `selector_kind` (`method` or `operator`)
  - `selector`
  - `arity`

Family-conditioned schema constraints enforce expected shape classes:

- sequence/pointer-like families require scalar `element_type` and empty
  `element_types`
- map families require `key_type` and `value_type`
- tuple/variant families require multi-element `element_types`
- map vs unordered-map policy shape expectations are separated

The canonical fixture now provides representative metadata for all required STL
placeholder kinds.

## Validation

Focused:

- `python3 -m unittest tests/python/test_parser_output_v1_schema.py -v`

Full Python:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Full workspace:

- `cargo test --workspace --all-targets`

## User Manual

Use these as authoritative references for downstream parser/backend work:

- schema contract:
  `docs/schemas/parser_output_v1.schema.json`
- canonical placeholder corpus fixture:
  `docs/fixtures/parser_output_v1_full_placeholders.json`

When adding a new STL placeholder family, update:

1. `node_kind` enum and `stl_placeholder.family` enum
2. family-conditioned `type_shape`/`policy_shape` rules
3. canonical fixture entry with `operation_selector`
4. `tests/python/test_parser_output_v1_schema.py`
