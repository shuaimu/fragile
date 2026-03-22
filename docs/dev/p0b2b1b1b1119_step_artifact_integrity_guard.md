# P0.b.2.b.1.b.1.1 Step-Artifact Integrity Guard (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.9` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` remains date-gated to on/after **2026-04-18**. This
pre-cutover leaf adds deterministic integrity checks so step artifacts cannot be
mutated silently between capture and verification.

This contract extends:

- `P0.b.2.b.1.b.1.1.6` (per-step log capture)
- `P0.b.2.b.1.b.1.1.7` (anchor-delta assertions)
- `P0.b.2.b.1.b.1.1.8` (freshness + run-id provenance)

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

## Integrity Contract

### 1) Preconditions: fresh, run-id-tagged artifacts exist

Generate fresh logs and anchors using `P0.b.2.b.1.b.1.1.8` / `.7` flow first:

- `/tmp/p0b2b1b1b1116_b111.log`
- `/tmp/p0b2b1b1b1116_b112.log`
- `/tmp/p0b2b1b1b1116_b2.log`
- `/tmp/p0b2b1b1b1116_b3.log`
- `/tmp/p0b2b1b1b1116_c.log`
- `/tmp/p0b2b1b1b1117_b111.anchors`
- `/tmp/p0b2b1b1b1117_b112.anchors`
- `/tmp/p0b2b1b1b1117_b2.anchors`
- `/tmp/p0b2b1b1b1117_b3.anchors`
- `/tmp/p0b2b1b1b1117_c.anchors`

### 2) Capture per-file checksums for logs and anchors

```bash
sha256sum /tmp/p0b2b1b1b1116_b111.log > /tmp/p0b2b1b1b1119_b111.log.sha256
sha256sum /tmp/p0b2b1b1b1116_b112.log > /tmp/p0b2b1b1b1119_b112.log.sha256
sha256sum /tmp/p0b2b1b1b1116_b2.log > /tmp/p0b2b1b1b1119_b2.log.sha256
sha256sum /tmp/p0b2b1b1b1116_b3.log > /tmp/p0b2b1b1b1119_b3.log.sha256
sha256sum /tmp/p0b2b1b1b1116_c.log > /tmp/p0b2b1b1b1119_c.log.sha256

sha256sum /tmp/p0b2b1b1b1117_b111.anchors > /tmp/p0b2b1b1b1119_b111.anchors.sha256
sha256sum /tmp/p0b2b1b1b1117_b112.anchors > /tmp/p0b2b1b1b1119_b112.anchors.sha256
sha256sum /tmp/p0b2b1b1b1117_b2.anchors > /tmp/p0b2b1b1b1119_b2.anchors.sha256
sha256sum /tmp/p0b2b1b1b1117_b3.anchors > /tmp/p0b2b1b1b1119_b3.anchors.sha256
sha256sum /tmp/p0b2b1b1b1117_c.anchors > /tmp/p0b2b1b1b1119_c.anchors.sha256
```

### 3) Create one run-scoped checksum manifest

```bash
RUN_ID="p0b2b1b1b1119_$(date -u +%Y%m%dT%H%M%SZ)"
cat \
  /tmp/p0b2b1b1b1119_b111.log.sha256 \
  /tmp/p0b2b1b1b1119_b112.log.sha256 \
  /tmp/p0b2b1b1b1119_b2.log.sha256 \
  /tmp/p0b2b1b1b1119_b3.log.sha256 \
  /tmp/p0b2b1b1b1119_c.log.sha256 \
  /tmp/p0b2b1b1b1119_b111.anchors.sha256 \
  /tmp/p0b2b1b1b1119_b112.anchors.sha256 \
  /tmp/p0b2b1b1b1119_b2.anchors.sha256 \
  /tmp/p0b2b1b1b1119_b3.anchors.sha256 \
  /tmp/p0b2b1b1b1119_c.anchors.sha256 \
  > /tmp/p0b2b1b1b1119_${RUN_ID}.manifest
```

### 4) Verify checksums before every downstream assertion pass

```bash
sha256sum -c /tmp/p0b2b1b1b1119_b111.log.sha256
sha256sum -c /tmp/p0b2b1b1b1119_b112.log.sha256
sha256sum -c /tmp/p0b2b1b1b1119_b2.log.sha256
sha256sum -c /tmp/p0b2b1b1b1119_b3.log.sha256
sha256sum -c /tmp/p0b2b1b1b1119_c.log.sha256

sha256sum -c /tmp/p0b2b1b1b1119_b111.anchors.sha256
sha256sum -c /tmp/p0b2b1b1b1119_b112.anchors.sha256
sha256sum -c /tmp/p0b2b1b1b1119_b2.anchors.sha256
sha256sum -c /tmp/p0b2b1b1b1119_b3.anchors.sha256
sha256sum -c /tmp/p0b2b1b1b1119_c.anchors.sha256
```

### 5) Reject rerun drift caused by stale checksum sidecars

Before recapturing a new run, remove old checksum sidecars so stale `-c`
verification cannot pass against previous artifacts:

```bash
rm -f /tmp/p0b2b1b1b1119_*.sha256
```

For each run, require a run-scoped manifest and verify expected diagnostic token
in pre-`P0.b.2.c` logs:

```bash
test -s /tmp/p0b2b1b1b1119_${RUN_ID}.manifest
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b111.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b112.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b2.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b3.log
```

Expected pre-`P0.b.2.c` diagnostic token: `error[E0599]`.

## Ownership Boundaries

- `P0.b.2.b.1.b.1.1.9` adds integrity checks only.
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
