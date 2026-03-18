# M6.2 Actionable STL-Shape Diagnostics Payload (2026-03-18)

## Objective

Complete TODO leaf `M6.2` by ensuring unsupported STL-shape failures expose
stable actionable metadata:

- location
- symbol
- shape fingerprint
- missing mapping key

## Scope and Size

This leaf is small (<1000 LOC):

- extend parser-core diagnostic data structures
- plumb parser node source location in parser backend flattening
- enrich active mapping failure diagnostics in `fragile-clang`
- add deterministic focused tests

No semantic fallback behavior changes and no target-specific logic.

## Wrong-Approach Guard

Checked against `docs/fragile-dev-book.md` section `1.3` and
`docs/dev/wrong.md`:

- no fake semantic method bodies
- no rollback pattern expansion
- no force-native bypassing
- no silent swallowing of unsupported-shape failures

## Design

1. Parser-core payload model:
   - `ParserNode` includes source coordinates (`source_file`, `source_line`,
     `source_column`).
   - `ParserDiagnostic` includes optional structured payload.
   - `DiagnosticPayload` carries deterministic actionable fields and stable
     display formatting.
   - `UnsupportedStlShapeError::to_parser_diagnostic()` now emits payload.

2. Parser backend source-location plumbing:
   - `fragile-parser-clang` fills parser node source coordinate fields from AST
     node locations while flattening parser output.

3. Active mapping failure enrichment:
   - In `resolve_parser_output_stl_placeholder_mappings(...)`, unsupported
     placeholder diagnostics now use:
     - symbol: `name` -> `cpp_type` -> `node_kind`
     - location: node source location -> TU source path fallback
     - shape fingerprint: `family(cpp_type)` when available
     - missing key: explicit node-kind key for unknown placeholder kind

4. Regression coverage:
   - parser-core payload/determinism tests
   - fragile-clang mapping-path tests validating actionable fields and
     node-location preference

## Validation

Focused:

- `cargo test -p fragile-parser-core`
- `cargo test -p fragile-parser-clang`
- `cargo test -p fragile-clang parser_output_stl_placeholder_mapping_ -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
