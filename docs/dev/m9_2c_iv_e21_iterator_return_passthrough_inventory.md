# M9.2.c.iv.e.21 iterator-return passthrough normalization slice

## Scope
- Leaf: `M9.2.c.iv.e.21`
- Goal: execute one bounded post-e.20 reduction focused on dominant `E0308` without increasing `E0425`/`E0599`/`E0609`.

## Task Sizing
- The selected change is localized to one existing normalizer (`normalize_unresolved_iterator_return_types`) plus focused unit tests.
- Estimated and actual code footprint stays well under 1000 LOC.

## Plan Before Execution
1. Re-run strict inventory on `debugging/misc/basetypes/logging` with the M9 harness compile profile.
2. Isolate one dominant repeated `E0308` sub-cluster.
3. Implement one bounded generic fix in `ast_codegen`.
4. Add focused tests for preserved passthrough and degraded rewrite behavior.
5. Rebuild release `fragilec`, rerun strict inventory, and publish deterministic deltas.

## Wrong-Approach Check
- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Re-reviewed `docs/dev/wrong.md`.
- No target-specific conditionals, no native fallback bypasses, no semantic source stubs, and no rollback-pattern additions were introduced.

## Baseline (Pre-fix)
- Run-root: `/tmp/fragile_e21_before_5rawle`
- Compile profile: `FRAGILEC_MODE=strict`, `-std=gnu++23`, M9 include/define set from `mako_compile_args`.

Per-file counts (`total / E0308 / E0425 / E0599 / E0609`):
- `debugging`: `48 / 16 / 17 / 0 / 1`
- `misc`: `48 / 16 / 17 / 0 / 1`
- `basetypes`: `38 / 17 / 6 / 0 / 1`
- `logging`: `66 / 24 / 23 / 0 / 1`

Aggregate:
- `total=200`, `E0308=73`, `E0425=63`, `E0599=0`, `E0609=4`

## Root Cause
- `normalize_unresolved_iterator_return_types` rewrote all `-> _InputIterator` / `-> _OutputIterator` function returns to `-> ()`.
- This is valid only for truly degraded signatures, but invalid for concrete passthrough signatures where the matching iterator-typed parameter lane is present.
- The over-broad rewrite generated repeated iterator-return `E0308` mismatches.

## Implementation
- Refined `normalize_unresolved_iterator_return_types` so it:
  - preserves `-> _InputIterator` when the signature has an `_InputIterator` parameter lane;
  - preserves `-> _OutputIterator` when the signature has an `_OutputIterator` parameter lane;
  - rewrites to `()` only when the matching iterator parameter lane is absent (true degraded case).
- Added focused unit tests:
  - `test_normalize_unresolved_iterator_return_preserves_input_iterator_passthrough`
  - `test_normalize_unresolved_iterator_return_preserves_output_iterator_passthrough`
- Existing degraded-shape tests remain in place.

## Post-fix Results
- Run-root: `/tmp/fragile_e21_after_fix_V7hyar`

Per-file counts (`total / E0308 / E0425 / E0599 / E0609`):
- `debugging`: `44 / 12 / 17 / 0 / 1`
- `misc`: `44 / 12 / 17 / 0 / 1`
- `basetypes`: `34 / 13 / 6 / 0 / 1`
- `logging`: `62 / 20 / 23 / 0 / 1`

Aggregate delta vs baseline:
- `total: 200 -> 184` (`-16`)
- `E0308: 73 -> 57` (`-16`)
- `E0425: 63 -> 63` (`0`, non-increase)
- `E0599: 0 -> 0` (`0`, non-increase)
- `E0609: 4 -> 4` (`0`, non-increase)

## Non-Increase Evidence
- Non-target classes did not increase:
  - `E0425: 63 -> 63`
  - `E0599: 0 -> 0`
  - `E0609: 4 -> 4`
- Target class reduced:
  - `E0308: 73 -> 57`

## Validation
- Targeted tests:
  - `cargo test -p fragile-clang normalize_unresolved_iterator_return -- --nocapture`
- Full regression suites (post-change):
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
