# M9.2.c.iv.e.32 Post-e31 Full-Flag Inventory

## Scope
- Leaf: `M9.2.c.iv.e.32`
- Goal: refresh strict compile inventory with harness-equivalent include/define profile and close the `logging.cpp` inventory gap.

## Wrong-Approach Check
- No target-specific (`mako`/`rpc`) conditionals introduced.
- No force-native bypasses.
- No parser backend rollback.
- No semantic type-mapping shortcuts.

## Command Profile
- Compiler: `./target/release/fragilec`
- Mode: `FRAGILEC_MODE=strict`
- Common flags:
  - `-I vendor/mako/src`
  - `-I vendor/mako/src/rrr`
  - `-I vendor/mako/src/memdb`
  - `-I vendor/mako/src/mako`
  - `-I vendor/mako/test`
  - `-I vendor/mako/third-party/rusty-cpp/include`
  - `-I vendor/mako/third-party/googletest/googletest/include`
  - `-I vendor/mako/third-party/googletest/googletest`
  - `-DGTEST_HAS_PTHREAD=1`
  - `-std=gnu++23`
  - `-w`

## Replay Artifacts
- Run root: `/tmp/fragile_e32_inventory_full_NRzZrX`
- Files compiled:
  - `vendor/mako/src/rrr/base/debugging.cpp`
  - `vendor/mako/src/rrr/base/misc.cpp`
  - `vendor/mako/src/rrr/base/basetypes.cpp`
  - `vendor/mako/src/rrr/base/logging.cpp`

## Results
- `debugging.cpp`: success, typed rustc errors `0`
- `misc.cpp`: success, typed rustc errors `0`
- `basetypes.cpp`: success, typed rustc errors `0`
- `logging.cpp`: failed with exactly `1` typed rustc error

Residual blocker (`logging.cpp`):
- `E0308` at `/tmp/fragilec_transpiled/logging.cpp_262dfa90071b81e7_logging.rs:4521`
- Expression: `data_: data,`
- Expected: `*mut std::cell::UnsafeCell<T>`
- Found: `*mut std::cell::UnsafeCell<()>`

## Dominant Next Slice
- Remaining strict-lane blockers for this profile collapse to one cluster:
  - `SpinLockResult` data pointer lane mismatch in `logging.cpp` (`UnsafeCell<T>` vs `UnsafeCell<()>`).
- Next leaf (`e.33`) should target this single mismatch with a bounded, generic normalization.
