# P0.b.2.b.1.b.1.1 Single-Hunk Delete Contract (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.2` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` is date-gated to on/after **2026-04-18**. This pre-cutover
leaf locks an exact delete-only hunk contract for removing
`StrictParserBackend::Libtooling` so cutover-day edits stay deterministic and do
not spill into sibling ownership.

## Wrong-Approach Guard Check

Re-reviewed before documenting:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific behavior, no force-native bypass, and no semantic fake/stub
fallback behavior is introduced by this leaf.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

Expected cutover edit remains declaration-only:

- delete one line `Libtooling,` from `enum StrictParserBackend` in
  `crates/fragile-driver/src/lib.rs`

Estimated footprint: **~4-12 LOC** (<1000 LOC target).

## Delete-Only Hunk Contract

Allowed hunk shape for `P0.b.2.b.1.b.1.1`:

```diff
enum StrictParserBackend {
-    Libtooling,
     ParserCore { backend_id: String },
}
```

Pre-check command (must return one declaration entry before cutover removal):

```bash
rg -n '^\s*Libtooling,$' crates/fragile-driver/src/lib.rs
```

Line-window verification command:

```bash
nl -ba crates/fragile-driver/src/lib.rs | sed -n '34,40p'
```

Post-check command for this leaf (must return no declaration entry):

```bash
rg -n '^\s*Libtooling,$' crates/fragile-driver/src/lib.rs
```

ParserCore declaration must remain in place:

```bash
rg -n 'ParserCore \{ backend_id: String \},' crates/fragile-driver/src/lib.rs
```

## Collateral-Edit Guard

`P0.b.2.b.1.b.1.1` must not remove downstream references in this step; those
are intentionally left for sibling leaves:

- `P0.b.2.b.1.b.2`: `strict_parser_backend_from_legacy_backend` mapping (`912`)
- `P0.b.2.b.1.b.3`: strict-path variant-match compile cleanup (`594`, `1270`, `1701`, `1707`)
- `P0.b.2.c`: backend value/label/help contract cleanup (`622`)
- `P0.b.2.b.1.b.1.2`: `ParserCoreCodegenEscapeHatch::Libtooling` declaration

Guard command (expected to show remaining references until sibling leaves run):

```bash
rg -n "StrictParserBackend::Libtooling" crates/fragile-driver/src/lib.rs
```

## Validation Commands

After `P0.b.2.b.1.b.1.1` delete-only hunk on cutover day:

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
```

After stacked execution with `b.1.2` + `b.2` + `b.3` + `P0.b.2.c`:

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```
