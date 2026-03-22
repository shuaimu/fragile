# P0.b.2.b.1.b.1 `fragile-driver` Enum-Declaration Removal Patch Spec (Pre-Cutover)

Date: 2026-03-21
Task: `P0.b.2.b.1.b.1.0` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1` is date-gated to on/after **2026-04-18**. This spec isolates
only the enum declaration edits for
`crates/fragile-driver/src/lib.rs` so cutover-day removal is minimal and
ownership boundaries remain explicit.

## Wrong-Approach Guard Check

Re-checked before drafting:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This pre-cutover leaf adds planning/contracts only. It does not introduce
native bypasses, target-specific paths, or semantic fake bodies.

## LOC Sizing (`P0.b.2.b.1.b.1`)

Expected direct declaration edits:

- `P0.b.2.b.1.b.1.1`: remove `StrictParserBackend::Libtooling` from
  `enum StrictParserBackend` (~4-10 LOC)
- `P0.b.2.b.1.b.1.2`: remove `ParserCoreCodegenEscapeHatch::Libtooling` from
  `enum ParserCoreCodegenEscapeHatch` (~3-8 LOC)

Expected total: **~7-18 LOC** (<1000 LOC target).

## Ownership Boundary (`b.1` vs `b.2` vs `b.3`)

`P0.b.2.b.1.b.1` owns declaration-only edits at the enum definitions.

- Owns:
  - `StrictParserBackend::Libtooling` declaration removal
  - `ParserCoreCodegenEscapeHatch::Libtooling` declaration removal

- Does not own (`P0.b.2.b.1.b.2`):
  - `strict_parser_backend_from_legacy_backend` adapter mapping removal

- Does not own (`P0.b.2.b.1.b.3`):
  - strict-path variant-match compile-break cleanup after declaration removal

## Line-Anchored Edit Targets (`crates/fragile-driver/src/lib.rs`)

Current anchors:

- `35-39`: `enum StrictParserBackend` (remove `Libtooling` entry)
- `41-44`: `enum ParserCoreCodegenEscapeHatch` (remove `Libtooling` entry)

Boundary anchors that remain untouched in this leaf:

- `908-923`: `strict_parser_backend_from_legacy_backend` (`b.2`)
- `1269-1290`: strict-path Libtooling policy/match usage (`b.3`)
- `1697-1762`: strict backend/escape-hatch validation tests (`b.3`)

## Ordered Cutover Steps

1. Apply `b.1.1.1`: remove `StrictParserBackend::Libtooling` declaration line.
2. Apply `b.1.1.2`: remove `ParserCoreCodegenEscapeHatch::Libtooling`
   declaration line.
3. Stop declaration edits; continue follow-up in `b.2` and `b.3` for resulting
   compile coupling.

## Validation Commands

Run after each declaration step:

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
```

Run full gates after stacked cutover (`b.1` + `b.2` + `b.3` + `P0.b.2.c`):

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Operator Notes

- This leaf is intentionally narrow to avoid cross-leaf coupling drift.
- If declaration removal causes compile failures, do not patch adapter/match
  callsites here; proceed to the owning leaves (`b.2`, `b.3`).
