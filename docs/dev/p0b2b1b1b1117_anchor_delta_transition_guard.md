# P0.b.2.b.1.b.1.1 Anchor-Delta Transition Guard (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.7` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` remains date-gated to on/after **2026-04-18**. This
pre-cutover leaf adds deterministic **step-to-step delta assertions** over
per-step diagnostic logs so the expected anchor progression can be verified by
set difference, not only by per-step spot checks.

This contract builds on `P0.b.2.b.1.b.1.1.6` (unique per-step log capture) and
locks expected anchor additions/removals across:

- `P0.b.2.b.1.b.1.1`
- `P0.b.2.b.1.b.1.2`
- `P0.b.2.b.1.b.2`
- `P0.b.2.b.1.b.3`
- `P0.b.2.c`

## Wrong-Approach Guard Check

Re-reviewed before documenting:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific behavior, no force-native bypass, and no semantic fake/stub
fallback behavior is introduced by this planning contract.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

The date-gated production edit remains declaration-only:

- delete `Libtooling,` from `enum StrictParserBackend` in
  `crates/fragile-driver/src/lib.rs` (`36-39`)

Estimated cutover edit size: **~4-12 LOC** (<1000 LOC target).

## Step Artifact Inputs

Reuse per-step logs from `P0.b.2.b.1.b.1.1.6`:

- `/tmp/p0b2b1b1b1116_b111.log`
- `/tmp/p0b2b1b1b1116_b112.log`
- `/tmp/p0b2b1b1b1116_b2.log`
- `/tmp/p0b2b1b1b1116_b3.log`
- `/tmp/p0b2b1b1b1116_c.log`

Normalize each log to a sorted anchor set file:

```bash
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_b111.log | sort -u > /tmp/p0b2b1b1b1117_b111.anchors
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_b112.log | sort -u > /tmp/p0b2b1b1b1117_b112.anchors
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_b2.log | sort -u > /tmp/p0b2b1b1b1117_b2.anchors
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_b3.log | sort -u > /tmp/p0b2b1b1b1117_b3.anchors
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_c.log | sort -u > /tmp/p0b2b1b1b1117_c.anchors
```

## Expected Delta Assertions

1. `b.1.1 -> b.1.2`
- Added anchors: `src/lib.rs:636:`, `src/lib.rs:1289:`, `src/lib.rs:1752:`
- Removed anchors: none

2. `b.1.2 -> b.2`
- Removed anchors: `src/lib.rs:912:`
- Added anchors: none

3. `b.2 -> b.3`
- Removed anchors: `src/lib.rs:594:`, `src/lib.rs:1270:`, `src/lib.rs:1701:`, `src/lib.rs:1707:`
- Added anchors: none

4. `b.3 -> P0.b.2.c`
- Removed anchors: `src/lib.rs:622:`, `src/lib.rs:636:`, `src/lib.rs:1289:`, `src/lib.rs:1752:`
- Added anchors: none

## Delta Verification Commands

Compute added (`new - old`) and removed (`old - new`) sets:

```bash
comm -13 /tmp/p0b2b1b1b1117_b111.anchors /tmp/p0b2b1b1b1117_b112.anchors
comm -23 /tmp/p0b2b1b1b1117_b111.anchors /tmp/p0b2b1b1b1117_b112.anchors

comm -13 /tmp/p0b2b1b1b1117_b112.anchors /tmp/p0b2b1b1b1117_b2.anchors
comm -23 /tmp/p0b2b1b1b1117_b112.anchors /tmp/p0b2b1b1b1117_b2.anchors

comm -13 /tmp/p0b2b1b1b1117_b2.anchors /tmp/p0b2b1b1b1117_b3.anchors
comm -23 /tmp/p0b2b1b1b1117_b2.anchors /tmp/p0b2b1b1b1117_b3.anchors

comm -13 /tmp/p0b2b1b1b1117_b3.anchors /tmp/p0b2b1b1b1117_c.anchors
comm -23 /tmp/p0b2b1b1b1117_b3.anchors /tmp/p0b2b1b1b1117_c.anchors
```

Guard command for still-expected `E0599` during pre-`P0.b.2.c` steps:

```bash
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b111.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b112.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b2.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b3.log
```

Expected diagnostic token at these pre-`P0.b.2.c` steps: `error[E0599]`.

## Ownership Boundaries

- `P0.b.2.b.1.b.1.1.7` adds evidence-delta assertions only.
- It does not edit production code and does not reassign cutover ownership.
- Edit ownership remains with `P0.b.2.b.1.b.1.1`, `P0.b.2.b.1.b.1.2`,
  `P0.b.2.b.1.b.2`, `P0.b.2.b.1.b.3`, and `P0.b.2.c` on/after 2026-04-18.

## Regression Gates

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```
