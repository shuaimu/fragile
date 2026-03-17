# M1.3 ParserOutput v1 Deterministic Serialization (2026-03-17)

## Objective

Close TODO leaf `M1.3` by adding deterministic serialization and fixture
round-trip tests for ParserOutput placeholder IR.

## Scope and Sizing

This iteration is below 1000 LOC:

- add deterministic serializer helper module:
  `scripts/parser_output_v1_contract.py`
- add canonical deterministic fixture artifact:
  `docs/fixtures/parser_output_v1_full_placeholders.canonical.json`
- add focused serialization/round-trip regressions:
  `tests/python/test_parser_output_v1_serialization.py`
- mark `M1.3` complete in `TODO.md`

No additional decomposition is required.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This leaf is contract/test tooling only:

- no target-specific parser/codegen behavior changes
- no force-native bypasses
- no fake semantic fallback method bodies

## Design

Added deterministic serialization helper functions:

- `dumps_parser_output_canonical(...)`
  - stable pretty JSON with sorted object keys
- `canonical_round_trip(...)`
  - canonical serialize + parse cycle
- `check_canonical_parser_output(...)`
  - strict byte-for-byte fixture verification

Added CLI for regeneration/verification:

- generate canonical output:
  - `python3 scripts/parser_output_v1_contract.py --input <in.json> --canonical-output <out.json>`
- check canonical output:
  - `python3 scripts/parser_output_v1_contract.py --input <in.json> --canonical-output <out.json> --check`

Canonical fixture is now tracked in repository:

- `docs/fixtures/parser_output_v1_full_placeholders.canonical.json`

## Validation

Focused:

- `python3 -m unittest tests/python/test_parser_output_v1_schema.py -v`
- `python3 -m unittest tests/python/test_parser_output_v1_serialization.py -v`

Full Python:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Full workspace:

- `cargo test --workspace --all-targets`

## User Manual

To refresh canonical fixture after intentional schema/fixture changes:

```bash
python3 scripts/parser_output_v1_contract.py \
  --input docs/fixtures/parser_output_v1_full_placeholders.json \
  --canonical-output docs/fixtures/parser_output_v1_full_placeholders.canonical.json
```

To verify deterministic output in CI or local checks:

```bash
python3 scripts/parser_output_v1_contract.py \
  --input docs/fixtures/parser_output_v1_full_placeholders.json \
  --canonical-output docs/fixtures/parser_output_v1_full_placeholders.canonical.json \
  --check
```
