# M9.2.c.iv.e.29 exception_ptr signature rehydration inventory

## Scope
- Leaf: `M9.2.c.iv.e.29`
- Goal: execute one bounded post-e.28 reduction via `normalize_e29_residual_errors`.

## Task Sizing
- Change scope is one post-processing normalizer, one pipeline wiring edit, and focused unit tests.
- Estimated and actual implementation footprint is below 1000 LOC.

## Plan Before Execution
1. Capture strict replay baseline for `vendor/mako/src/rrr/base/debugging.cpp` using the harness-equivalent compile profile.
2. Select one bounded residual lane without semantic stubs.
3. Implement a generic post-processing fix in `ast_codegen`.
4. Add focused unit tests.
5. Rebuild release `fragilec`, rerun strict replay, and record deterministic before/after counts.

## Wrong-Approach Check
- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Re-reviewed `docs/dev/wrong.md`.
- No target-specific conditionals, no force-native bypasses, no rollback-pattern additions, and no semantic fake bodies were introduced.

## Baseline (Pre-fix)
- Run-root: `/tmp/fragile_e29_after_gwujnD`
- Compile profile: `FRAGILEC_MODE=strict`, `gnu++23`, full mako include paths used by `mako_compile_args`.

Counts:
- `total=6`
- `E0308=2`
- `E0512=2`
- `E0599=1`
- `E0596=1`

Top messages:
- `E0308`: mismatched iterator and hash-code return lane types.
- `E0512`: transmute size mismatch in unicode consume-result bitfield accessor/mutator.
- `E0599`: `FragileVaList::new()` unresolved constructor call.
- `E0596`: missing `mut` on iterator parameter.

## Dominant Bounded Lane Selected
- Selected lane: degraded `exception_ptr` signatures (`&u128`/`&mut u128`) and a recurring signedness mismatch in decimal conversion helper local init.
- Why bounded: deterministic token-level patterns in post-processed Rust output; no source-target special casing needed.

## Implementation
Added `normalize_e29_residual_errors` and wired it after `normalize_e28_residual_errors`.

Targeted rewrites:
1. `exception_ptr` signature rehydration:
- `op_assign(&mut self, __o: &mut u128) -> &mut u128`
  -> `op_assign(&mut self, __o: &mut exception_ptr) -> &mut exception_ptr`
- `new_1_2(__other: &u128) -> Self`
  -> `new_1_2(__other: &exception_ptr) -> Self`
- `op_assign_1(&mut self, __other: &u128) -> &mut u128`
  -> `op_assign_1(&mut self, __other: &exception_ptr) -> &mut exception_ptr`
- `swap(&mut self, mut __other: &mut u128)`
  -> `swap(&mut self, mut __other: &mut exception_ptr)`

2. Signedness mismatch repair:
- `let mut __uval: u64 = if __neg { !__val as u64 + 1 } else { __val };`
  -> `let mut __uval: u64 = if __neg { !__val as u64 + 1 } else { __val as u64 };`

3. Parse-label artifact cleanup:
- `"{ _opaque: {"` -> `"{ {"`.

Focused unit tests added in `ast_codegen.rs`:
- `test_e29_rehydrates_exception_ptr_signatures`
- `test_e29_fixes_uval_signedness_mismatch`
- `test_e29_removes_malformed_opaque_block_label`

## Post-fix Results
- Run-root: `/tmp/fragile_e29_postchange_n4hSt0`

Counts:
- `total=4`
- `E0308=2`
- `E0605=1`
- `E0596=1`
- `E0599=0`
- `E0512=0`

Aggregate delta vs baseline:
- `total: 6 -> 4` (`-2`)
- `E0599: 1 -> 0` (`-1`)
- `E0512: 2 -> 0` (`-2`)
- `E0308: 2 -> 2` (`0`, non-increase)
- `E0596: 1 -> 1` (`0`, non-increase)
- New surfaced `E0605: 0 -> 1` (expected waterfall after removing transmute-lane blocker).

## Non-Increase Evidence
- Pre-existing residual classes under this slice did not increase:
  - `E0308: 2 -> 2`
  - `E0596: 1 -> 1`
- Targeted blocker classes reduced:
  - `E0599: 1 -> 0`
  - `E0512: 2 -> 0`

## Validation
- Focused unit tests:
  - `cargo test -p fragile-clang test_e29_ -- --nocapture`
- Full regression suites:
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
