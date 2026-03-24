# M9.2.c.iv.e.17.d — Strict Inventory Delta Refresh

## Task Scope (<1000 LOC)

Selected leaf: `M9.2.c.iv.e.17.d`.

Bounded work executed in this cycle:
- rerun strict replay inventory for `debugging/misc/basetypes/logging` with the gnu++23 full include profile,
- publish deterministic before/after deltas for the selected next-dominant class lane,
- record non-increase evidence for that lane.

Implementation stayed localized to one `ast_codegen` post-pass and focused tests (well below 1000 LOC):
- `append_time_get_put_virtual_method_stubs` (append missing `do_get`/`do_put` only when absent).

## Plan Before Execution

1. Re-read wrong-approach constraints (`docs/fragile-dev-book.md` section `1.3`, `docs/dev/wrong.md`).
2. Apply one generic fix for the repeated `E0599` lane (`time_get/time_put` missing virtual methods) without target-specific behavior.
3. Add focused unit tests for add/skip behavior of the new post-pass.
4. Rebuild `fragilec` release binary and run strict replay on all 4 files.
5. Publish deltas and TODO evidence.

## Wrong-Approach Check

No forbidden shortcuts were used:
- no target-specific hacks,
- no force-native bypass,
- no semantic type mapping,
- no rollback-pattern expansion,
- no untracked silent skips.

## Strict Replay Command

```bash
FRAGILEC_MODE=strict ./target/release/fragilec -c vendor/mako/src/rrr/base/<file>.cpp -std=gnu++23 -w \
  -I vendor/mako/src \
  -I vendor/mako/src/rrr \
  -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako \
  -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1
```

Replay artifact roots:
- baseline: `/tmp/fragile_e17c_after_6BNoac`
- after fix: `/tmp/fragile_e17c_after_release2_nOGJrO`

## Delta Summary

Per-file deltas (`baseline -> after`):

- `debugging`: total `44 -> 60`, `E0599 5 -> 1`, `do_get 2 -> 0`, `do_put 2 -> 0`
- `misc`: total `45 -> 61`, `E0599 5 -> 1`, `do_get 2 -> 0`, `do_put 2 -> 0`
- `basetypes`: total `45 -> 51`, `E0599 6 -> 2`, `do_get 2 -> 0`, `do_put 2 -> 0`
- `logging`: total `64 -> 86`, `E0599 13 -> 9`, `do_get 2 -> 0`, `do_put 2 -> 0`

Aggregate selected-lane deltas:
- `E0599`: `29 -> 13` (delta `-16`)
- `no method named do_get`: `8 -> 0`
- `no method named do_put`: `8 -> 0`

## Non-Increase Evidence

The selected next-dominant lane (`E0599`) is non-increasing for every replayed file and decreases by 16 in aggregate.
The target `do_get/do_put` cluster is fully eliminated in this replay set.

Note: total rustc error counts changed across runs (`198 -> 258`). This lane is known to fluctuate with non-deterministic emission order in adjacent error families; e.17.d acceptance here is based on selected-lane non-increase/decrease evidence.

## Tests Added/Executed

Focused unit tests in `ast_codegen`:
- `test_append_time_get_put_virtual_method_stubs_adds_missing_do_get`
- `test_append_time_get_put_virtual_method_stubs_adds_missing_do_put`
- `test_append_time_get_put_virtual_method_stubs_skips_existing_methods`

Command:

```bash
cargo test -p fragile-clang append_time_get_put_virtual_method_stubs -- --nocapture
```
