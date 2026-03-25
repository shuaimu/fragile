# M9.2.c.iv.e.33 — SpinMutexGuard Constructor Data-Lane Rehydration

Date: 2026-03-25

## Task sizing analysis
- Parent task: `M9.2.c.iv.e` strict replay closure.
- Selected first open leaf: `M9.2.c.iv.e.33`.
- Scope remained bounded (<1000 LOC): one generic normalization pass in `ast_codegen` + focused unit tests + strict replay verification.

## Plan before execution
1. Reproduce the residual strict blocker from `e.32` (`logging.cpp` single `E0308`).
2. Implement one generic pass that rehydrates `SpinMutexGuard_*::new_2` degraded `data` param type from `UnsafeCell<()>` to struct-declared `data_` lane type.
3. Add focused tests for rewrite, unit-lane no-op behavior, and scope isolation.
4. Re-run strict replay inventory with full harness-equivalent compile flags.
5. Run full regression suites.

## Wrong-approach check
- Re-reviewed `docs/dev/wrong.md` and the dev-book wrong-approach guidance.
- Avoided target-specific or path-specific rewrites.
- Avoided force-native fallback, rollback-pattern edits, and one-off output patching.
- Kept normalization keyed by generated Rust shape (`SpinMutexGuard_*` struct/impl correspondence), not source-file names.

## Implementation summary
- Added `normalize_spinmutex_guard_constructor_data_param_types(code: &str) -> String` in `crates/fragile-clang/src/ast_codegen.rs`.
- Pass behavior:
  - Scans `pub struct SpinMutexGuard_*` definitions and records each `data_` field type.
  - Scans matching `impl SpinMutexGuard_*` blocks.
  - Rewrites only `pub fn new_2(..., data: *mut UnsafeCell<()>)` parameter lanes when struct `data_` is `UnsafeCell<...>` and non-unit.
  - Leaves unit-lane constructors unchanged.
  - Uses brace-depth scope guards so later unrelated structs do not pollute lane mapping.
- Integrated the pass at pipeline tail after `normalize_e31_residual_errors`.

## Focused test coverage
- `test_spinmutex_guard_constructor_param_rehydration_rewrites_data_type`
- `test_spinmutex_guard_constructor_param_rehydration_skips_unit_field_lane`
- `test_spinmutex_guard_constructor_param_rehydration_preserves_struct_scope`

Targeted run:
- `cargo test -p fragile-clang spinmutex_guard_constructor_param_rehydration -- --nocapture`
- Result: all focused tests passed.

## Strict replay evidence
Baseline from `e.32`:
- Run-root: `/tmp/fragile_e32_inventory_full_NRzZrX`
- Summary: `debugging=0`, `misc=0`, `basetypes=0`, `logging=1` (`E0308` residual in `SpinLockResult` lane).

Post-fix (`e.33`) inventory profile:
- `FRAGILEC_MODE=strict ./target/release/fragilec -c -I vendor/mako/src -I vendor/mako/src/rrr -I vendor/mako/src/memdb -I vendor/mako/src/mako -I vendor/mako/test -I vendor/mako/third-party/rusty-cpp/include -I vendor/mako/third-party/googletest/googletest/include -I vendor/mako/third-party/googletest/googletest -DGTEST_HAS_PTHREAD=1 -std=gnu++23 -w vendor/mako/src/rrr/base/{debugging,misc,basetypes,logging}.cpp`
- Run-root: `/tmp/fragile_e33_inventory_full_final_ZZJQHn`
- Summary file: `/tmp/fragile_e33_inventory_full_final_ZZJQHn/summary.tsv`
- Result:
  - `debugging ok 0`
  - `misc ok 0`
  - `basetypes ok 0`
  - `logging ok 0`

## Regression gate runs
- `cargo test --workspace --all-targets` passed.
- `python3 -m unittest discover -s tests/python -p 'test_*.py'` passed (`84` tests, `0` failed, `1` skipped).
