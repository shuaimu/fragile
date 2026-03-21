# P0.b.2.b.1.b `fragile-driver` Variant-Removal Patch Map (Pre-Cutover)

Date: 2026-03-21
Task: `P0.b.2.b.1.b.0` (pre-cutover)

## Purpose

`P0.b.2.b.1.b` is gated to on/after **2026-04-18**. This document gives a
line-anchored, file-local patch map for `crates/fragile-driver/src/lib.rs` so
cutover-day edits are bounded, ordered, and compile-checkable.

## Wrong-Approach Guard Check

Re-reviewed before preparing this map:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific bypasses, no force-native shortcuts, and no fake semantic
stubs are introduced by this pre-cutover leaf.

## LOC Sizing (`P0.b.2.b.1.b`)

Estimated cutover edits in `crates/fragile-driver/src/lib.rs`:

- `P0.b.2.b.1.b.1`: enum variant removal (`StrictParserBackend`,
  `ParserCoreCodegenEscapeHatch`) at the declaration site: ~12-24 LOC
- `P0.b.2.b.1.b.2`: legacy-backend adapter mapping removal in
  `strict_parser_backend_from_legacy_backend`: ~10-20 LOC
- `P0.b.2.b.1.b.3`: strict-path compile-break cleanup for now-missing variants
  in local matches/tests: ~20-55 LOC

Expected total: **~42-99 LOC** (<1000 LOC target).

## Ownership Boundary (`P0.b.2.b.1.b` vs adjacent leaves)

`P0.b.2.b.1.b` owns only enum-variant deletion + direct compile-break cleanup
caused by deleted variants in `fragile-driver`.

It does **not** own string/help contract removals (owned by `P0.b.2.c`):

- `parse_parser_backend_value`
- `strict_parser_backend_label`
- `parse_parser_core_codegen_escape_hatch_value`

It does **not** own full legacy codepath removal (owned by `P0.b.2.d`/`P0.b.2.e`):

- escape-hatch env/policy/logging/reporting function removal
- complete removal of `use_libtooling_codegen_escape_hatch` routing and
  `ClangParserBackend::Libtooling` fallback branches

## Line-Anchored Edit Map (`crates/fragile-driver/src/lib.rs`)

Primary anchors from current file revision:

- `35-44`: `StrictParserBackend` and `ParserCoreCodegenEscapeHatch`
  declarations
- `591-643`: parser backend and escape-hatch value parsing functions
- `620-624`: `strict_parser_backend_label`
- `908-923`: `strict_parser_backend_from_legacy_backend`
- `1269-1275`: strict parser backend Libtooling policy gate match
- `1287-1290`: parser-core escape-hatch Libtooling match
- `1324-1331`: `codegen_backend_label` assembly
- `1342`: forced `ClangParserBackend::Libtooling` backend for legacy path
- `1697-1762`: strict backend / escape-hatch validation tests

### Ordered cutover checkpoints

1. `P0.b.2.b.1.b.1`
   Remove `Libtooling` enum variants at lines `35-44`.
2. `P0.b.2.b.1.b.2`
   Remove the Libtooling adapter arm in
   `strict_parser_backend_from_legacy_backend` at lines `908-923`.
3. `P0.b.2.b.1.b.3`
   Resolve local compile-break sites that still reference deleted variants,
   primarily around lines `1269-1290` and test block `1697-1762`.
4. Keep `P0.b.2.c` boundaries intact while doing this file-local cleanup:
   do not edit parsing/label/help functions at lines `591-643` and `620-624`
   in this leaf.

## Validation Commands

After each cutover checkpoint above:

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
```

After stacking with adjacent date-gated leaves (`P0.b.2.c`/`P0.b.2.e`):

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Operator Notes

- Apply `P0.b.2.b.1.b` as part of a stacked cutover with `P0.b.2.c` and
  `P0.b.2.e`; otherwise expected compile coupling may appear between removed
  variants and still-supported backend strings/labels.
- Keep edits production-path generic and deterministic.
