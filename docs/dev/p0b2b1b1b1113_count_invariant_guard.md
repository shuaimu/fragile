# P0.b.2.b.1.b.1.1 Declaration/Reference Count Invariant Guard (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.3` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` is date-gated to on/after **2026-04-18**. This pre-cutover
leaf locks measurable count invariants so cutover-day deletion of
`StrictParserBackend::Libtooling` is a single-hunk declaration edit with no
collateral removals in sibling ownership.

## Wrong-Approach Guard Check

Re-reviewed before documenting:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific behavior, no force-native bypass, and no semantic fake/stub
fallback logic is introduced by this pre-cutover contract.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

Expected cutover edit remains:

- remove exactly one declaration entry `Libtooling,` from
  `enum StrictParserBackend` in `crates/fragile-driver/src/lib.rs`

Estimated footprint: **~4-12 LOC** (<1000 LOC target).

## Baseline Count Invariants (Pre-Cutover)

Current declaration anchors in `crates/fragile-driver/src/lib.rs`:

- `36-39`: `enum StrictParserBackend` includes `Libtooling,`
- `42-44`: `enum ParserCoreCodegenEscapeHatch` includes `Libtooling,`

Baseline verification commands and expected counts:

```bash
sed -n '36,39p' crates/fragile-driver/src/lib.rs | rg -n '^\s*Libtooling,$' | wc -l
# expected: 1

sed -n '42,44p' crates/fragile-driver/src/lib.rs | rg -n '^\s*Libtooling,$' | wc -l
# expected: 1

rg -n "StrictParserBackend::Libtooling" crates/fragile-driver/src/lib.rs | wc -l
# expected: 6
```

Reference inventory for the `StrictParserBackend::Libtooling` count (`6`):

- `594`
- `622`
- `912`
- `1270`
- `1701`
- `1707`

## Post-Hunk Invariants for `P0.b.2.b.1.b.1.1`

Immediately after the single-hunk declaration removal (`b.1.1`) and before
running sibling leaves:

- strict-backend declaration count must drop `1 -> 0`
- escape-hatch declaration count must stay `1`
- qualified strict-backend reference count must stay `6`

Post-check commands:

```bash
sed -n '36,39p' crates/fragile-driver/src/lib.rs | rg -n '^\s*Libtooling,$' | wc -l
sed -n '42,44p' crates/fragile-driver/src/lib.rs | rg -n '^\s*Libtooling,$' | wc -l
rg -n "StrictParserBackend::Libtooling" crates/fragile-driver/src/lib.rs | wc -l
```

## Ownership Boundaries (No Collateral Edits)

`P0.b.2.b.1.b.1.1` owns only `StrictParserBackend` declaration entry removal.

Do not modify in this leaf:

- `P0.b.2.b.1.b.1.2`: `ParserCoreCodegenEscapeHatch::Libtooling` declaration
- `P0.b.2.b.1.b.2`: `strict_parser_backend_from_legacy_backend` mapping
- `P0.b.2.b.1.b.3`: strict variant-match compile-break cleanup
- `P0.b.2.c`: backend string/label/help contract removal

## Validation Commands

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```
