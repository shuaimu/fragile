# P0.b.2.b.1.b.1.1 Step-Artifact Freshness Guard (Pre-Cutover)

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1.8` (pre-cutover)

## Purpose

`P0.b.2.b.1.b.1.1` remains date-gated to on/after **2026-04-18**. This
pre-cutover leaf adds a deterministic freshness contract so the step artifacts
used by `P0.b.2.b.1.b.1.1.6` and `P0.b.2.b.1.b.1.1.7` cannot be accidentally
reused from a stale prior run.

The guard makes run provenance explicit for:

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
fallback behavior is introduced by this contract.

## LOC Sizing (`P0.b.2.b.1.b.1.1`)

The date-gated production edit remains declaration-only:

- delete `Libtooling,` from `enum StrictParserBackend` in
  `crates/fragile-driver/src/lib.rs` (`36-39`)

Estimated cutover edit size: **~4-12 LOC** (<1000 LOC target).

## Freshness Contract

### 1) Remove stale step artifacts before capture

```bash
rm -f \
  /tmp/p0b2b1b1b1116_b111.log \
  /tmp/p0b2b1b1b1116_b112.log \
  /tmp/p0b2b1b1b1116_b2.log \
  /tmp/p0b2b1b1b1116_b3.log \
  /tmp/p0b2b1b1b1116_c.log

rm -f \
  /tmp/p0b2b1b1b1117_b111.anchors \
  /tmp/p0b2b1b1b1117_b112.anchors \
  /tmp/p0b2b1b1b1117_b2.anchors \
  /tmp/p0b2b1b1b1117_b3.anchors \
  /tmp/p0b2b1b1b1117_c.anchors
```

### 2) Stamp this capture run with explicit provenance

```bash
START_EPOCH="$(date -u +%s)"
RUN_ID="p0b2b1b1b1118_$(date -u +%Y%m%dT%H%M%SZ)"
```

### 3) Capture per-step logs with run-id marker

```bash
{ echo "RUN_ID=${RUN_ID} STEP=b111"; cargo test -p fragile-driver; } 2>&1 | tee /tmp/p0b2b1b1b1116_b111.log
{ echo "RUN_ID=${RUN_ID} STEP=b112"; cargo test -p fragile-driver; } 2>&1 | tee /tmp/p0b2b1b1b1116_b112.log
{ echo "RUN_ID=${RUN_ID} STEP=b2"; cargo test -p fragile-driver; } 2>&1 | tee /tmp/p0b2b1b1b1116_b2.log
{ echo "RUN_ID=${RUN_ID} STEP=b3"; cargo test -p fragile-driver; } 2>&1 | tee /tmp/p0b2b1b1b1116_b3.log
{ echo "RUN_ID=${RUN_ID} STEP=c"; cargo test -p fragile-driver; } 2>&1 | tee /tmp/p0b2b1b1b1116_c.log
```

### 4) Verify run marker + fresh modification time in each log

```bash
rg -n "RUN_ID=${RUN_ID} STEP=b111" /tmp/p0b2b1b1b1116_b111.log
rg -n "RUN_ID=${RUN_ID} STEP=b112" /tmp/p0b2b1b1b1116_b112.log
rg -n "RUN_ID=${RUN_ID} STEP=b2" /tmp/p0b2b1b1b1116_b2.log
rg -n "RUN_ID=${RUN_ID} STEP=b3" /tmp/p0b2b1b1b1116_b3.log
rg -n "RUN_ID=${RUN_ID} STEP=c" /tmp/p0b2b1b1b1116_c.log

test "$(stat -c %Y /tmp/p0b2b1b1b1116_b111.log)" -ge "${START_EPOCH}"
test "$(stat -c %Y /tmp/p0b2b1b1b1116_b112.log)" -ge "${START_EPOCH}"
test "$(stat -c %Y /tmp/p0b2b1b1b1116_b2.log)" -ge "${START_EPOCH}"
test "$(stat -c %Y /tmp/p0b2b1b1b1116_b3.log)" -ge "${START_EPOCH}"
test "$(stat -c %Y /tmp/p0b2b1b1b1116_c.log)" -ge "${START_EPOCH}"
```

### 5) Rebuild anchor sets only from fresh logs

```bash
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_b111.log | sort -u > /tmp/p0b2b1b1b1117_b111.anchors
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_b112.log | sort -u > /tmp/p0b2b1b1b1117_b112.anchors
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_b2.log | sort -u > /tmp/p0b2b1b1b1117_b2.anchors
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_b3.log | sort -u > /tmp/p0b2b1b1b1117_b3.anchors
rg -o 'src/lib.rs:[0-9]+:' /tmp/p0b2b1b1b1116_c.log | sort -u > /tmp/p0b2b1b1b1117_c.anchors

for file in \
  /tmp/p0b2b1b1b1117_b111.anchors \
  /tmp/p0b2b1b1b1117_b112.anchors \
  /tmp/p0b2b1b1b1117_b2.anchors \
  /tmp/p0b2b1b1b1117_b3.anchors \
  /tmp/p0b2b1b1b1117_c.anchors
do
  test -s "${file}"
done
```

### 6) Preserve expected diagnostic token before `P0.b.2.c`

```bash
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b111.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b112.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b2.log
rg -n 'error\[E0599\]' /tmp/p0b2b1b1b1116_b3.log
```

Expected pre-`P0.b.2.c` diagnostic token: `error[E0599]`.

## Ownership Boundaries

- `P0.b.2.b.1.b.1.1.8` adds artifact freshness and provenance checks only.
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
