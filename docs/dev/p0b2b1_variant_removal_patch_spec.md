# P0.b.2.b.1 Variant-Removal Patch Spec (Pre-Cutover)

Date: 2026-03-21
Task: `P0.b.2.b.1.a` (pre-cutover)

## Purpose

`P0.b.2.b.1` is date-gated to on/after **2026-04-18**. This document is the
line-level patch spec for the first code-removal slice so cutover-day edits are
bounded, reproducible, and compile-checked.

## Wrong-Approach Guard Check

Re-checked before drafting this patch spec:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This leaf only adds planning/contract coverage. No target-specific special
cases, no force-native bypasses, and no fake semantic stubs were introduced.

## LOC Sizing (for `P0.b.2.b.1`)

Estimated touched LOC by cutover sub-leaf:

- `P0.b.2.b.1.b` (`crates/fragile-driver/src/lib.rs`): ~35-70 LOC
- `P0.b.2.b.1.c` (`crates/fragile-cli/src/bin/fragilec.rs`): ~40-90 LOC
- `P0.b.2.b.1.d` (residual strict-path variant matches/tests in both files):
  ~60-140 LOC

Expected total for `P0.b.2.b.1` stack: **~135-300 LOC**, below the <1000 LOC
leaf target.

## Ownership Boundary: `P0.b.2.b.1` vs `P0.b.2.c`

`P0.b.2.b.1` owns enum-variant deletion and direct strict-path variant match
cleanup only.

- Remove:
  - `StrictParserBackend::Libtooling`
  - `ParserCoreCodegenEscapeHatch::Libtooling`
  - `strict_parser_backend_from_legacy_backend` arms that construct removed
    variants
  - strict-path `matches!(..., StrictParserBackend::Libtooling)` and
    `Some(ParserCoreCodegenEscapeHatch::Libtooling)` branches that no longer
    compile after variant deletion

`P0.b.2.c` owns all backend-string/help contract removals:

- `parse_parser_backend_value` (`"libtooling"` acceptance)
- `strict_parser_backend_label` (`"libtooling"` label)
- `parse_codegen_escape_hatch_value` (`"libtooling"` acceptance)

## Ordered Edit Checkpoints (Cutover Day)

### `crates/fragile-driver/src/lib.rs` (`P0.b.2.b.1.b`)

Apply edits in this order:

1. Remove `Libtooling` from `enum StrictParserBackend`.
2. Remove `Libtooling` from `enum ParserCoreCodegenEscapeHatch`.
3. Update `strict_parser_backend_from_legacy_backend` so it no longer returns
   `StrictParserBackend::Libtooling`.
4. Do **not** edit `parse_parser_backend_value`, `strict_parser_backend_label`,
   or `parse_codegen_escape_hatch_value` in this step (reserved for `P0.b.2.c`).

### `crates/fragile-cli/src/bin/fragilec.rs` (`P0.b.2.b.1.c`)

Apply edits in this order:

1. Remove `Libtooling` from `enum StrictParserBackend`.
2. Remove `Libtooling` from `enum ParserCoreCodegenEscapeHatch`.
3. Update `strict_parser_backend_from_legacy_backend` so it no longer returns
   `StrictParserBackend::Libtooling`.
4. Do **not** edit `parse_parser_backend_value`, `strict_parser_backend_label`,
   or `parse_codegen_escape_hatch_value` in this step (reserved for `P0.b.2.c`).

### Residual variant-match cleanup (`P0.b.2.b.1.d`)

After the declarations are removed, clean up strict-path branches that still
match deleted variants in both production drivers.

## Validation Checkpoints

Run after each sub-leaf (`b.1.b`, `b.1.c`, `b.1.d`):

```bash
cargo test -p fragile-driver
cargo test -p fragile-cli
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
```

Run full gates after stacking with `P0.b.2.c`:

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Cutover-Day Operator Notes

- Apply this patch spec as a single stacked change together with `P0.b.2.c`.
- If compilation fails between slices due to parser-backend string/label matches,
  continue with the `P0.b.2.c` edits in the same stack (expected coupling).
- Keep all edits generic and production-path-wide; do not add target-specific
  bypasses.
