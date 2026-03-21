# P0.b.2.b Variant Removal Dependency Map (Pre-Cutover)

Date: 2026-03-21
Task: `P0.b.2.b.0` (pre-cutover)

## Purpose

`P0.b.2.b` is date-gated to on/after **2026-04-18**. This note captures the
exact dependency map for removing:

- `StrictParserBackend::Libtooling`
- `ParserCoreCodegenEscapeHatch::Libtooling`

from production drivers (`fragile-driver`, `fragilec`) without broad cutover-day
guesswork.

## Wrong-Approach Guard Check

Re-checked:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This pre-cutover work adds planning/contracts only. No target-specific hack, no
force-native bypass, and no semantic stub path was introduced.

## Inventory: `crates/fragile-driver/src/lib.rs`

Key variant declarations and references (from `rg -n` inventory):

- `36`: `enum StrictParserBackend { Libtooling, ... }`
- `42`: `enum ParserCoreCodegenEscapeHatch { Libtooling, ... }`
- `594`: `parse_parser_backend_value("libtooling") -> StrictParserBackend::Libtooling`
- `622`: `strict_parser_backend_label` match includes `StrictParserBackend::Libtooling`
- `636`: `parse_codegen_escape_hatch_value("libtooling") -> ParserCoreCodegenEscapeHatch::Libtooling`
- `910`: `strict_parser_backend_from_legacy_backend` maps `ClangParserBackend::Libtooling`
- `1270`: strict-path branch `matches!(parser_backend, StrictParserBackend::Libtooling)`
- `1289`: escape-hatch branch checks `Some(ParserCoreCodegenEscapeHatch::Libtooling)`
- `1699+`: unit tests constructing/asserting `StrictParserBackend::Libtooling`
- `1752+`: unit tests constructing/asserting `ParserCoreCodegenEscapeHatch::Libtooling`

## Inventory: `crates/fragile-cli/src/bin/fragilec.rs`

Key variant declarations and references:

- `41`: `enum StrictParserBackend { Libtooling, ... }`
- `47`: `enum ParserCoreCodegenEscapeHatch { Libtooling, ... }`
- `1342`: `parse_parser_backend_value("libtooling") -> StrictParserBackend::Libtooling`
- `1370`: `strict_parser_backend_label` match includes `StrictParserBackend::Libtooling`
- `1384`: `parse_codegen_escape_hatch_value("libtooling") -> ParserCoreCodegenEscapeHatch::Libtooling`
- `1410`: `strict_parser_backend_from_legacy_backend` maps `ClangParserBackend::Libtooling`
- `1990`: strict-path branch `matches!(parser_backend, StrictParserBackend::Libtooling)`
- `2009`: escape-hatch branch checks `Some(ParserCoreCodegenEscapeHatch::Libtooling)`
- `3243+`, `3308+`, `3723+`: unit tests constructing/asserting Libtooling variants

## Coupling Boundary with `P0.b.2.c`

`P0.b.2.b` (variant removal) and `P0.b.2.c` (backend string/help contract
removal) are tightly coupled:

- Removing enum variants first will not compile unless parse/label/helper match
  sites stop constructing removed variants.
- Therefore `P0.b.2.b.1` and `P0.b.2.c` must be applied in the same cutover PR
  (or same stack) with compile checks between slices.

## Ordered Cutover Slices (on/after 2026-04-18)

1. `P0.b.2.b.1`: remove enum variants + remove direct construction from internal
   legacy-adapter matches.
2. `P0.b.2.c`: remove `"libtooling"` parse acceptance, labels, and help/env
   contract references that depend on removed variants.
3. `P0.b.2.b.2`: update driver/CLI tests to parser-core-only invariants.

## Validation Checkpoints

Run after each slice:

```bash
cargo test -p fragile-driver
cargo test -p fragile-cli
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
```

Final full gates for the cutover PR:

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```
