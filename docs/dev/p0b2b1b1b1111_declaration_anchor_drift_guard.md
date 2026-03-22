# P0.b.2.b.1.b.1.1 Declaration-Anchor Drift Guard (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.1` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` remains date-gated to on/after **2026-04-18**. This
pre-cutover guard locks the declaration-anchor and downstream-reference checks
so cutover-day deletion of `StrictParserBackend::Libtooling` stays deterministic
and does not silently drift into adjacent ownership.

## Wrong-Approach Guard Check

Re-reviewed before documenting:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific bypasses, no force-native shortcuts, and no semantic fake
stubs are introduced by this planning leaf.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

Expected cutover-day edit size remains:

- remove one declaration entry `Libtooling,` from `enum StrictParserBackend`
  in `crates/fragile-driver/src/lib.rs`

Estimated footprint: **~4-10 LOC** (<1000 LOC target).

## Declaration Anchor Fingerprint

Expected declaration region in `crates/fragile-driver/src/lib.rs`:

```rust
enum StrictParserBackend {
    Libtooling,
    ParserCore { backend_id: String },
}
```

Verification command:

```bash
nl -ba crates/fragile-driver/src/lib.rs | sed -n '34,40p'
```

## Downstream Reference Verification Checklist

Qualified references expected pre-cutover:

- `594`: `parse_parser_backend_value` returns `StrictParserBackend::Libtooling`
- `622`: `strict_parser_backend_label` maps variant to `"libtooling"`
- `912`: `strict_parser_backend_from_legacy_backend` adapter mapping
- `1270`: strict policy gate `matches!(..., StrictParserBackend::Libtooling)`
- `1701` and `1707`: driver tests asserting `StrictParserBackend::Libtooling`

Verification command:

```bash
rg -n "StrictParserBackend::Libtooling" crates/fragile-driver/src/lib.rs
```

## Ownership Boundary Reminder

`P0.b.2.b.1.b.1.1` owns declaration removal only.

- Do not modify adapter mapping here (`P0.b.2.b.1.b.2`).
- Do not modify strict-path variant-match cleanup here (`P0.b.2.b.1.b.3`).
- Do not modify backend string/help contract here (`P0.b.2.c`).
- Do not modify `ParserCoreCodegenEscapeHatch::Libtooling` declaration here
  (`P0.b.2.b.1.b.1.2`).

## Validation Commands

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```
