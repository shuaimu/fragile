# M9.2.c.iv.e.30 Residual Error Normalization Inventory

## Scope
- Leaf: `M9.2.c.iv.e.30`
- Goal: execute one bounded post-e.29 reduction via `normalize_e30_residual_errors`.

## Task Sizing
- Implementation stayed below the `<1000 LOC` leaf budget.
- Touched surface:
  - `crates/fragile-clang/src/ast_codegen.rs` (one new normalizer + pipeline wiring + tests)
  - `TODO.md` leaf closure entry

## Plan Before Execution
1. Re-check wrong-approach guidance (`docs/fragile-dev-book.md` section `1.3`, `docs/dev/wrong.md`).
2. Reuse deterministic e29 baseline artifacts to select a bounded residual lane.
3. Implement generic post-processing normalizations in `ast_codegen` (no target-specific branches).
4. Add focused unit tests for each e30 rewrite.
5. Re-run strict compile replay on the same bounded profile and record before/after deltas.

## Wrong-Approach Check
- No target-specific (`mako`/`rpc`) conditionals were added.
- No force-native bypass or fallback compiler delegation was introduced.
- No rollback-pattern expansions were added.
- No semantic type mapping shortcuts were introduced.

## Baseline (Pre-e30)
- Run root: `/tmp/fragile_e29_postchange_n4hSt0`
- Command profile:
  - `FRAGILEC_MODE=strict ./target/release/fragilec -c -std=gnu++23 -I vendor/mako/src -I vendor/mako/src/rrr -o <obj> vendor/mako/src/rrr/base/debugging.cpp`
- Counts (from `debugging.stderr`):
  - Typed rustc errors (`error[E*]`): `4`
  - Breakdown: `E0308=2`, `E0605=1`, `E0596=1`

## Dominant Bounded Lane Selected
- Residual typed blockers in the bounded replay:
  - varargs lane-shape mismatch for `vasprintf_1` (`FragileVaList` vs `[FragileVaList; 1]`)
  - non-primitive bitfield status getter/setter casts (`__status` / `set___status`)
  - missing mutable parameter in wrap-iter `adjacent_difference_*`
  - locale pointer mutability in `__asprintf_1`

## Implementation
Added `normalize_e30_residual_errors` and wired it after `normalize_e29_residual_errors`.

Targeted rewrites:
1. `vasprintf_1` arg shape fix:
   - `std::mem::zeroed::<FragileVaList>()` -> `std::mem::zeroed::<[FragileVaList; 1]>()`
2. Non-primitive `__status` getter rehydration:
   - Replace `(self._bitfield_1 & 0x1) as u8` with zeroed output + first-byte write expression.
3. Non-primitive `set___status` setter rehydration:
   - Replace `((v as u8) & 0x1)` with first-byte lane read from `v` pointer cast.
4. Wrap-iter mutability:
   - `adjacent_difference_*` signatures gain `mut __first` for incrementing loop artifacts.
5. Locale pointer mutability:
   - `__asprintf_1` signature rewrites `__loc:` to `mut __loc:` when `&mut __loc` is passed.

## Post-e30 Replay Results
- Final run root: `/tmp/fragile_e30_postchange4_QJpqTq`
- Same bounded replay profile as baseline.

Delta:
- Typed rustc blockers (`error[E*]`): `4 -> 0`
  - `E0308: 2 -> 0`
  - `E0605: 1 -> 0`
  - `E0596: 1 -> 0`
- Waterfall surfaced new lint-deny blockers (non-`E*`):
  - `invalid_reference_casting`: `2`
  - `deref_nullptr`: `4`

Interpretation:
- e30 fully cleared the targeted typed residual lane.
- Newly surfaced blockers are downstream lint-deny classes and become next-leaf candidates.

## Validation
- Focused unit tests:
  - `cargo test -p fragile-clang test_e30_ -- --nocapture`
- Strict bounded replay:
  - command profile above, with run roots:
    - baseline: `/tmp/fragile_e29_postchange_n4hSt0`
    - post-e30 final: `/tmp/fragile_e30_postchange4_QJpqTq`
