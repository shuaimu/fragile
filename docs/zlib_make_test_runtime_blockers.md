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

## Strict-link baseline after C-ABI export fix
Reproducer:

```bash
cargo test -p fragile-clang --test real_world_zlib_tests \
  test_real_world_zlib_make_test_command_subset_replay -- --ignored --nocapture --test-threads=1
```

Current deterministic first failing strict-link step:
- Status file: `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs/link_required_example.status`
- Status value: `1`
- Stderr file: `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs/link_required_example.stderr`

Observed stderr now starts with unresolved Rust runtime symbols from replayed zlib archive members:

```text
/usr/bin/ld: .../libz.a(deflate.o): undefined reference to `<std::sys::sync::once::futex::Once>::call'
/usr/bin/ld: .../libz.a(deflate.o): undefined reference to `core::panicking::panic'
...
collect2: error: ld returned 1 exit status
```

## Strict-link baseline after runtime support link-input fix
Reproducer:

```bash
cargo test -p fragile-clang --test real_world_zlib_tests \
  test_real_world_zlib_make_test_command_subset_replay -- --ignored --nocapture
```

Current deterministic first failing strict-link step:
- Status file: `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs/link_required_example.status`
- Status value: `1`
- Stderr file: `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs/link_required_example.stderr`

Observed stderr now starts with unresolved C global symbol gaps:

```text
/usr/bin/ld: .../libz.a(deflate.o): ... undefined reference to `_dist_code'
/usr/bin/ld: .../libz.a(deflate.o): ... undefined reference to `_length_code'
...
collect2: error: ld returned 1 exit status
```

Runtime support artifacts now present and successful:
- `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs/rustc_link_runtime_support.status` = `0`
- `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs/libfragile_runtime_support.a` exists and is non-empty.

## Current hypothesis
Strict link replay is now behaving faithfully and exposes missing C-global symbol export parity for non-static zlib data objects (e.g., `_dist_code`, `_length_code`) after runtime-link inputs were added.

## Immediate next fix direction
1. Export required non-static C global data symbols with unmangled C ABI names in generated Rust.
2. Re-run strict required-link replay and ensure all required outputs link successfully.
3. Resume `make test` command-subset replay and then fix runtime command failures in order.
