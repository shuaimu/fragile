# P0.b.2.b.1.b.1.1 Compile-Error Fingerprint Guard (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.4` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` is date-gated to on/after **2026-04-18**. This pre-cutover
leaf locks a deterministic compile-error fingerprint for the isolated
declaration removal of `StrictParserBackend::Libtooling` in
`crates/fragile-driver/src/lib.rs`.

The guard ensures cutover-day execution can verify that only the expected
variant-reference compile breaks occur, while sibling ownership remains intact.

## Wrong-Approach Guard Check

Re-reviewed before documenting:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific behavior, no force-native bypass, and no semantic fake/stub
fallback behavior is introduced by this planning leaf.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

Expected cutover edit remains declaration-only:

- delete one line `Libtooling,` from `enum StrictParserBackend` in
  `crates/fragile-driver/src/lib.rs` (`36-39`)

Estimated footprint: **~4-12 LOC** (<1000 LOC target).

## Expected Post-Delete Compile Fingerprint

After only `P0.b.2.b.1.b.1.1` declaration deletion (before running
`b.1.2`/`b.2`/`b.3`/`P0.b.2.c`), `cargo test -p fragile-driver` is expected to
fail with `error[E0599]` at `StrictParserBackend::Libtooling` callsites.

Capture command:

```bash
cargo test -p fragile-driver 2>&1 | tee /tmp/p0b2b1b1b1114_after_b111.log
```

Required diagnostic fingerprint checks:

```bash
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1114_after_b111.log
rg -n 'no variant or associated item named `Libtooling` found for enum `StrictParserBackend`' /tmp/p0b2b1b1b1114_after_b111.log
rg -n 'src/lib.rs:(594|622|912|1270|1701|1707):' /tmp/p0b2b1b1b1114_after_b111.log
```

Expected source anchors covered by the fingerprint:

- `594`
- `622`
- `912`
- `1270`
- `1701`
- `1707`

## Ownership Boundary Guard

`P0.b.2.b.1.b.1.1` owns only `StrictParserBackend` declaration removal.

Do not resolve fingerprinted downstream compile breaks in this leaf; they are
owned by:

- `P0.b.2.b.1.b.1.2`: `ParserCoreCodegenEscapeHatch::Libtooling` declaration
- `P0.b.2.b.1.b.2`: `strict_parser_backend_from_legacy_backend` mapping
- `P0.b.2.b.1.b.3`: strict-path variant-match cleanup
- `P0.b.2.c`: backend string/label/help contract removal

## Validation Commands

```bash
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```
