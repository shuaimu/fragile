# P0.b.2.b.1.b.1.1 StrictParserBackend Declaration Removal Rehearsal (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.0` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` is explicitly date-gated to on/after **2026-04-18**. This
pre-cutover rehearsal narrows the single-entry declaration removal for
`StrictParserBackend::Libtooling` in `crates/fragile-driver/src/lib.rs` so the
cutover-day edit stays bounded and ownership remains explicit.

## Wrong-Approach Guard Check

Re-reviewed before preparing this rehearsal:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific bypasses, no force-native shortcuts, and no semantic fake
stubs are introduced by this pre-cutover leaf.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

Expected declaration-only edit:

- remove `Libtooling,` from `enum StrictParserBackend` in
  `crates/fragile-driver/src/lib.rs` (`35-39`)  

Estimated edit footprint: **~4-10 LOC** (<1000 LOC target).

## Line-Anchored Cutover Target

Current declaration anchor:

- `35-39`: `enum StrictParserBackend { Libtooling, ParserCore { backend_id: String }, }`

Cutover-day step for `P0.b.2.b.1.b.1.1`:

1. Delete the `Libtooling,` enum entry from this declaration block only.

## Compile-Break Inventory After Deletion (Owned by Other Leaves)

Removing only the declaration entry is expected to break downstream references.
Those references are intentionally handled by sibling/adjacent leaves:

- `P0.b.2.b.1.b.3` ownership (variant-match compile cleanup):
  - `parse_parser_backend_value`: `594`
  - strict policy gate: `1270`, `1272`
  - strict-mode tests constructing/asserting `StrictParserBackend::Libtooling`:
    `1699-1707`
- `P0.b.2.b.1.b.2` ownership (legacy backend adapter mapping):
  - `strict_parser_backend_from_legacy_backend`: `908-912`
- `P0.b.2.c` ownership (backend string/label/help contract removal):
  - `strict_parser_backend_label`: `620-622`
  - parser backend text contracts using `"libtooling"` labels
- `P0.b.2.b.1.b.1.2` ownership (separate enum declaration leaf):
  - `ParserCoreCodegenEscapeHatch::Libtooling` declaration at `41-44`

## Ownership Boundary (Do / Do Not)

`P0.b.2.b.1.b.1.1` owns only:

- declaration deletion of `StrictParserBackend::Libtooling` in
  `enum StrictParserBackend`.

`P0.b.2.b.1.b.1.1` does not own:

- `ParserCoreCodegenEscapeHatch::Libtooling` declaration removal (`b.1.2`)
- adapter mapping changes in `strict_parser_backend_from_legacy_backend` (`b.2`)
- compile-break cleanup for remaining variant references (`b.3`)
- backend string/help parsing/label contract removal (`P0.b.2.c`)

## Validation Commands

After declaration removal on cutover day:

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
```

After stacked execution with `b.1.2` + `b.2` + `b.3` + `P0.b.2.c`:

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```
