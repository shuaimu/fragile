# zlib make-test replay runtime blockers (current baseline)

## Scope
This note tracks the deterministic blocker sequence for `make test` replay under Fragile-linked zlib outputs.

## Previous permissive-link baseline
Before strict-link replay, the first failing `make test` command was:

```bash
TMPST=tmpst_$; if echo hello world | ./minigzip | ./minigzip -d && ./example $TMPST ; then echo ' *** zlib test OK ***'; else echo ' *** zlib test FAILED ***'; false; fi
```

with runtime stderr:

```text
./minigzip: error while loading shared libraries: unexpected PLT reloc type 0x00
./minigzip: error while loading shared libraries: unexpected PLT reloc type 0x00
```

## Strict-link baseline (after removing `--unresolved-symbols=ignore-all`)
Reproducer:

```bash
cargo test -p fragile-clang --test real_world_zlib_tests \
  test_real_world_zlib_fragile_required_link_binaries_replay -- --ignored --nocapture
```

First deterministic failing strict-link step:
- Status file: `/tmp/fragile_real_world_zlib_fragile_link_required_binaries/driver_logs/link_required_example.status`
- Status value: `1`
- Stderr file: `/tmp/fragile_real_world_zlib_fragile_link_required_binaries/driver_logs/link_required_example.stderr`

Observed stderr starts with unresolved zlib symbols from `example.o`:

```text
/usr/bin/ld: .../example.o: in function `main':
example.c:(.text.startup+0x1a): undefined reference to `zlibVersion'
example.c:(.text.startup+0xcb): undefined reference to `compress'
...
collect2: error: ld returned 1 exit status
```

## Current hypothesis
Strict link replay is now behaving faithfully and exposes unresolved-symbol gaps in the replayed link inputs/archives for required zlib test binaries (starting with `example`).

## Immediate next fix direction
1. Close strict link input/symbol/flag gaps until `link_required_example` succeeds.
2. Re-run strict required-link replay and ensure all required outputs link successfully.
3. Resume `make test` command-subset replay and then fix runtime command failures in order.
