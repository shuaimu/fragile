# P0.b.2.b.1.b.1.1 Step-Artifact Completeness Guard (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.10` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` remains date-gated to on/after **2026-04-18**. This
pre-cutover leaf adds deterministic completeness checks so checksum manifests
cannot pass with missing step artifacts, duplicate rows, or out-of-contract
artifact paths.

This contract extends:

- `P0.b.2.b.1.b.1.1.6` (per-step log capture)
- `P0.b.2.b.1.b.1.1.7` (anchor-delta assertions)
- `P0.b.2.b.1.b.1.1.8` (freshness + run-id provenance)
- `P0.b.2.b.1.b.1.1.9` (checksum integrity)

Protected sequence:

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
fallback behavior is introduced by this guard.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

The date-gated production edit remains declaration-only:

- delete `Libtooling,` from `enum StrictParserBackend` in
  `crates/fragile-driver/src/lib.rs` (`36-39`)

Estimated cutover edit size: **~4-12 LOC** (<1000 LOC target).

## Completeness Contract

### 1) Preconditions: integrity manifest exists

Run `P0.b.2.b.1.b.1.1.9` first to generate a run-scoped checksum manifest:

- `/tmp/p0b2b1b1b1119_${RUN_ID}.manifest`

### 2) Enforce manifest cardinality

The manifest must contain exactly five logs plus five anchor artifacts:

```bash
test "$(wc -l < /tmp/p0b2b1b1b1119_${RUN_ID}.manifest)" -eq 10
```

### 3) Enforce step-pair coverage across all cutover steps

Each step must have exactly one log row and one anchor row in the manifest:

```bash
for STEP in b111 b112 b2 b3 c; do
  rg -n "/tmp/p0b2b1b1b1116_${STEP}.log$" /tmp/p0b2b1b1b1119_${RUN_ID}.manifest
  rg -n "/tmp/p0b2b1b1b1117_${STEP}.anchors$" /tmp/p0b2b1b1b1119_${RUN_ID}.manifest
done
```

### 4) Reject duplicate manifest rows

```bash
sort /tmp/p0b2b1b1b1119_${RUN_ID}.manifest | uniq -d > /tmp/p0b2b1b1b11110_duplicates.txt
test ! -s /tmp/p0b2b1b1b11110_duplicates.txt
```

### 5) Reject out-of-contract artifact paths

Only run-local rows for the five allowed step IDs and expected file kinds are
permitted:

```bash
rg -n -v '^[0-9a-f]{64}  /tmp/p0b2b1b1b111(6|7)_(b111|b112|b2|b3|c)\\.(log|anchors)$' /tmp/p0b2b1b1b1119_${RUN_ID}.manifest
```

Expected result: no matches.

### 6) Keep pre-cutover diagnostic fingerprint invariant

```bash
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b111.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b112.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b2.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b3.log
```

Expected pre-`P0.b.2.c` diagnostic token: `error[E0599]`.

## Ownership Boundaries

- `P0.b.2.b.1.b.1.1.10` adds completeness checks only.
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
