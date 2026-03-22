# P0.b.2.b.1.b.1.1 Stepwise Diagnostic Log Contract (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.6` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` remains date-gated to on/after **2026-04-18**. This
pre-cutover leaf defines deterministic evidence capture for each stacked cutover
step so diagnostics are not overwritten and anchor transitions can be validated
step-by-step.

## Wrong-Approach Guard Check

Re-reviewed before documenting:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific logic, no force-native bypass, and no semantic fake/stub
fallback behavior is introduced by this planning contract.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

The date-gated code edit remains small:

- declaration-only deletion of `Libtooling,` from `enum StrictParserBackend`
  in `crates/fragile-driver/src/lib.rs` (`36-39`)

Estimated cutover edit size: **~4-12 LOC** (<1000 LOC target).

## Unique Per-Step Log Paths

Use a unique capture log per leaf in sequence:

1. `P0.b.2.b.1.b.1.1` -> `/tmp/p0b2b1b1b1116_b111.log`
2. `P0.b.2.b.1.b.1.2` -> `/tmp/p0b2b1b1b1116_b112.log`
3. `P0.b.2.b.1.b.2` -> `/tmp/p0b2b1b1b1116_b2.log`
4. `P0.b.2.b.1.b.3` -> `/tmp/p0b2b1b1b1116_b3.log`
5. `P0.b.2.c` -> `/tmp/p0b2b1b1b1116_c.log`

Capture command template:

```bash
cargo test -p fragile-driver 2>&1 | tee /tmp/p0b2b1b1b1116_<step>.log
```

## Expected Anchor Progression

For each step, verify `error[E0599]` and source anchors:

- after `P0.b.2.b.1.b.1.1`: `594`, `622`, `912`, `1270`, `1701`, `1707`
- after `P0.b.2.b.1.b.1.2`: `594`, `622`, `636`, `912`, `1270`, `1289`, `1701`, `1707`, `1752`
- after `P0.b.2.b.1.b.2`: `594`, `622`, `636`, `1270`, `1289`, `1701`, `1707`, `1752` (anchor `912` removed)
- after `P0.b.2.b.1.b.3`: `622`, `636`, `1289`, `1752` (anchors `594`, `1270`, `1701`, `1707` removed)
- after `P0.b.2.c`: no remaining `libtooling` strict-path anchor from this sequence

## Verification Commands

```bash
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b111.log
rg -n 'src/lib.rs:(594|622|912|1270|1701|1707):' /tmp/p0b2b1b1b1116_b111.log

rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b112.log
rg -n 'src/lib.rs:(594|622|636|912|1270|1289|1701|1707|1752):' /tmp/p0b2b1b1b1116_b112.log

rg -n 'src/lib.rs:(594|622|636|1270|1289|1701|1707|1752):' /tmp/p0b2b1b1b1116_b2.log
rg -n 'src/lib.rs:(622|636|1289|1752):' /tmp/p0b2b1b1b1116_b3.log
rg -n 'src/lib.rs:(622|636|1289|1752):' /tmp/p0b2b1b1b1116_c.log
```

Cross-step drift check (must preserve separate artifacts):

```bash
ls -1 /tmp/p0b2b1b1b1116_b111.log /tmp/p0b2b1b1b1116_b112.log /tmp/p0b2b1b1b1116_b2.log /tmp/p0b2b1b1b1116_b3.log /tmp/p0b2b1b1b1116_c.log
```

Final validation gates after stacked sequence:

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Ownership Boundaries

- This contract does not change production code.
- `P0.b.2.b.1.b.1.1.6` is evidence-capture guidance only.
- Removal/edit ownership remains with `P0.b.2.b.1.b.1.1`, `P0.b.2.b.1.b.1.2`,
  `P0.b.2.b.1.b.2`, `P0.b.2.b.1.b.3`, and `P0.b.2.c` on/after 2026-04-18.
