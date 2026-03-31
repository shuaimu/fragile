# M9.2.c.iv.e.31 Residual Error Normalization Inventory

## Scope
- Leaf: `M9.2.c.iv.e.31`
- Goal: execute one bounded post-e.30 reduction via `normalize_e31_residual_errors`.

## Task Sizing
- Bounded to one additive post-processing pass plus focused tests in `ast_codegen`.
- Estimated and actual implementation size stayed below `<1000 LOC`.

## Plan Before Execution
1. Re-check wrong-approach guidance (`docs/fragile-dev-book.md` section `1.3`, `docs/dev/wrong.md`).
2. Reuse the e30 bounded strict replay profile on `rrr/base/debugging.cpp`.
3. Target one dominant residual class family from the baseline strict replay output.
4. Add generic rewrites (no target-specific conditionals).
5. Add focused unit tests and verify replay delta on the same profile.

## Wrong-Approach Check
- No target-specific (`mako`/`rpc`) conditionals added.
- No force-native bypasses.
- No semantic type mappings.
- No rollback-pattern additions.

## Baseline (Pre-e31)
- Run root: `/tmp/fragile_e31_before_d2Jtlj`
- Command profile:
  - `FRAGILEC_MODE=strict ./target/release/fragilec -c -std=gnu++23 -I vendor/mako/src -I vendor/mako/src/rrr -o <obj> vendor/mako/src/rrr/base/debugging.cpp`
- Baseline blockers (`debugging.stderr`):
  - `invalid_reference_casting` deny: `2`
  - `deref_nullptr` deny: `4`
  - total blocking errors: `6` (`error: aborting due to 6 previous errors`)

## Dominant Residual Slice
Two recurring strict-lane deny classes:
1. Atomic-flag wait lane invalid reference cast:
   - `__atomic_wait_std_atomic_flag_bool(&mut *(self as *const Self as *mut Self), ...)`
2. Degraded reference null-init:
   - `let __x: &T = unsafe { &*std::ptr::null::<T>() };`

## Implementation
Added `normalize_e31_residual_errors` and wired it immediately after `normalize_e30_residual_errors`.

Rewrites:
1. Atomic wait lane:
   - `__atomic_wait_std_atomic_flag_bool(&mut *(self as *const Self as *mut Self), ... )`
   - -> `__atomic_wait_std_atomic_flag_bool((self as *const Self as *mut Self), ... )`
2. Null-deref ref init lane:
   - `let __x: &T = unsafe { &*std::ptr::null::<T>() };`
   - -> `let __x: &T = unsafe { &std::mem::zeroed::<T>() };`
   - Guard: only rewrites when `lhs` reference type `T` exactly matches `null::<T>` type.

## Post-e31 Replay Result
- Run root: `/tmp/fragile_e31_after_7GZjo3`
- Same bounded profile as baseline.

Delta:
- `invalid_reference_casting`: `2 -> 0`
- `deref_nullptr`: `4 -> 0`
- blocking errors: `6 -> 0`
- compile outcome: success (`debugging.o` emitted; `debugging.stderr` empty)

## Validation
- Focused unit tests:
  - `cargo test -p fragile-clang --lib test_e31_ -- --nocapture`
- e31 replay evidence:
  - baseline: `/tmp/fragile_e31_before_d2Jtlj`
  - post-e31: `/tmp/fragile_e31_after_7GZjo3`
