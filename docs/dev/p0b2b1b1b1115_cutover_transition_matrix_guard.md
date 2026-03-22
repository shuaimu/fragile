# P0.b.2.b.1.b.1.1 Cutover Transition Matrix Guard (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.5` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` remains date-gated to on/after **2026-04-18**. This
pre-cutover leaf publishes a deterministic transition matrix for cutover-day
execution so each stacked leaf (`b.1.1 -> b.1.2 -> b.2 -> b.3 -> P0.b.2.c`)
has a clear expected compile-error fingerprint and ownership boundary.

## Wrong-Approach Guard Check

Re-reviewed before documenting:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific behavior, no force-native bypass, and no semantic fake/stub
fallback behavior is introduced by this planning leaf.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

The date-gated code edit remains small:

- declaration-only deletion of `Libtooling,` from `enum StrictParserBackend`
  in `crates/fragile-driver/src/lib.rs` (`36-39`)

Estimated cutover edit size: **~4-12 LOC** (<1000 LOC target).

## Transition Matrix (Cutover Day)

1. `P0.b.2.b.1.b.1.1` (remove `StrictParserBackend::Libtooling` declaration)
Expected compile fingerprint after this isolated step:
  - `error[E0599]`
  - `no variant or associated item named \`Libtooling\` found for enum \`StrictParserBackend\``
  - source anchors: `594`, `622`, `912`, `1270`, `1701`, `1707`

2. `P0.b.2.b.1.b.1.2` (remove `ParserCoreCodegenEscapeHatch::Libtooling` declaration)
Expected additional compile fingerprint while `b.2`/`b.3`/`P0.b.2.c` remain pending:
  - `error[E0599]`
  - `no variant or associated item named \`Libtooling\` found for enum \`ParserCoreCodegenEscapeHatch\``
  - source anchors: `636`, `1289`, `1752`

3. `P0.b.2.b.1.b.2` (remove `strict_parser_backend_from_legacy_backend` mapping)
Expected transition:
  - source anchor `912` no longer appears in compile diagnostics
  - remaining strict-backend variant reference cleanup still belongs to `b.3` and `P0.b.2.c`

4. `P0.b.2.b.1.b.3` (resolve strict-path variant-match compile breaks)
Expected transition:
  - source anchors `594`, `1270`, `1701`, `1707` are removed by owned code updates
  - backend label/help string contract (`622`) remains owned by `P0.b.2.c`

5. `P0.b.2.c` (remove backend value/label/help `libtooling` contract)
Expected transition:
  - source anchor `622` removed
  - strict production-driver variant-removal sequence for this slice is compile-clean

## Verification Commands

Capture command per step:

```bash
cargo test -p fragile-driver 2>&1 | tee /tmp/p0b2b1b1b1115_step.log
```

Diagnostic checks:

```bash
rg -n 'error\\[E0599\\]' /tmp/p0b2b1b1b1115_step.log
rg -n 'StrictParserBackend|ParserCoreCodegenEscapeHatch' /tmp/p0b2b1b1b1115_step.log
rg -n 'src/lib.rs:(594|622|636|912|1270|1289|1701|1707|1752):' /tmp/p0b2b1b1b1115_step.log
```

Final regression gates after stacked sequence:

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```
