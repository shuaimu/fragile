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

## Strict-link baseline after C-global export fix
Reproducer:

```bash
cargo test -p fragile-clang --test real_world_zlib_tests \
  test_real_world_zlib_fragile_required_link_binaries_replay -- --ignored --nocapture --test-threads=1
```

Current strict-link status:
- `link_required_{example,minigzip,examplesh,minigzipsh,example64,minigzip64}.status` are all `0`.
- `/tmp/fragile_real_world_zlib_fragile_link_required_binaries/driver_logs/fragile_link_manifest.txt` is emitted with `relinked_output_count=6`.
- Manifest output sizes are non-empty for all required binaries.

## Make-test replay baseline after strict-link success
Reproducer (guarded by timeout because command #1 currently does not complete):

```bash
timeout 900 cargo test -p fragile-clang --test real_world_zlib_tests \
  test_real_world_zlib_make_test_command_subset_replay -- --ignored --nocapture --test-threads=1
```

Current runtime status for command #1 (`TMPST=tmpst_$; if echo hello world | ./minigzip | ./minigzip -d && ./example $TMPST ; then ... fi`):

- pipeline stage `echo hello world | ./minigzip | ./minigzip -d` no longer crashes;
- replay status remains `124` because `./example tmpst_$` does not terminate.

Observed artifact state:
- strict-link logs/manifests are present and successful;
- `make_test_dryrun.status` and `make_test_commands_manifest.txt` are produced;
- `make_test_replay_01.status` is `124`;
- `make_test_replay_01.stderr` is empty (previous `Segmentation fault` signature is cleared);
- `link_gzwrite_o_transpiled.rs` now lowers nested stream-field writes as `((*state).strm).next_in` / `((*state).strm).avail_in`.

## Make-test replay baseline after switch-fallthrough fix (current)
Reproducer:

```bash
cargo test -p fragile-clang --test real_world_zlib_tests \
  test_real_world_zlib_make_test_command_subset_replay -- --ignored --nocapture --test-threads=1
```

Current runtime status for command #1:
- Status file: `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs/make_test_replay_01.status`
- Status value: `1`
- Stderr file: `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs/make_test_replay_01.stderr`

Observed stderr:

```text
./minigzip: <fd:0>: incorrect data check
```

Observed behavior shift:
- `./example tmpst_$` no longer hangs/times out; it now exits quickly with `uncompress error: -3`.
- replay command #1 failure is now data-integrity/checksum validation, not non-termination.

## Current hypothesis
Strict-link input/symbol gaps and the command-#1 non-termination are resolved; the current first blocker is checksum/data-integrity mismatch in the gzip round-trip path (`minigzip`/`inflate`).

## Immediate next fix direction
1. Capture native-vs-fragile gzip stream/trailer deltas for command #1 round-trip.
2. Root-cause and fix checksum/state accounting mismatch in transpiled gzip/inflate runtime path.
3. Re-run command-subset replay and advance command #1 status from `1` to `0`.
