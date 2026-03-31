# M9.2.c.iv.e.20 iterator-return passthrough normalization slice

## Scope
- Leaf: `M9.2.c.iv.e.20`
- Goal: execute one bounded post-e.19 reduction focused on dominant `E0308` without increasing `E0425`.

## Task Sizing
- Candidate change was scoped to one existing normalizer (`normalize_unresolved_iterator_return_types`) plus focused tests.
- Estimated impact and implementation size stayed well under 1000 LOC.

## Plan Before Execution
1. Re-run strict inventory on `debugging/misc/basetypes/logging` with the M9 harness compile profile.
2. Identify one repeated dominant `E0308` sub-cluster.
3. Implement one bounded generic fix in `ast_codegen`.
4. Add focused unit tests for the fixed normalization behavior.
5. Rebuild release `fragilec`, rerun the strict inventory, and publish deterministic deltas.

## Wrong-Approach Check
- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Re-reviewed `docs/dev/wrong.md`.
- No target-specific conditionals, native-fallback bypasses, rollback-pattern additions, or semantic source stubs were introduced.

## Baseline (Pre-fix)
- Run-root: `/tmp/fragile_e20_before_clean_wtkmbR`
- Compile profile: `FRAGILEC_MODE=strict`, `-std=gnu++23`, M9 include/define set from `mako_compile_args`.

Per-file counts (`total / E0425 / E0308 / E0599`):
- `debugging`: `48 / 17 / 16 / 0`
- `misc`: `48 / 17 / 16 / 0`
- `basetypes`: `38 / 6 / 17 / 0`
- `logging`: `66 / 23 / 24 / 0`

Aggregate:
- `total=200`, `E0425=63`, `E0308=73`, `E0599=0`

## Root Cause
- `normalize_unresolved_iterator_return_types` rewrote all `-> _InputIterator` / `-> _OutputIterator` signatures to `-> ()`.
- This was correct only for degraded signatures (iterator return with unit-degraded params), but incorrect for concrete passthrough signatures that still had iterator-typed parameters.
- The over-broad rewrite produced repeated `E0308` mismatches in iterator passthrough flows.

## Implementation
- Refined `normalize_unresolved_iterator_return_types`:
  - preserve `-> _InputIterator` when function header contains `: _InputIterator` parameter lane.
  - preserve `-> _OutputIterator` when function header contains `: _OutputIterator` parameter lane.
  - keep existing rewrite to `()` for truly degraded iterator-return signatures.
- Added focused unit tests:
  - `test_normalize_unresolved_iterator_return_preserves_input_iterator_passthrough`
  - `test_normalize_unresolved_iterator_return_preserves_output_iterator_passthrough`
  - existing degraded-shape tests retained.

## Post-fix Results
- Run-root: `/tmp/fragile_e20_after_clean_B0JrDk`

Per-file counts (`total / E0425 / E0308 / E0599`):
- `debugging`: `44 / 17 / 12 / 0`
- `misc`: `44 / 17 / 12 / 0`
- `basetypes`: `34 / 6 / 13 / 0`
- `logging`: `62 / 23 / 20 / 0`

Aggregate delta vs baseline:
- `total: 200 -> 184` (`-16`)
- `E0308: 73 -> 57` (`-16`)
- `E0425: 63 -> 63` (`0`, non-increase)
- `E0599: 0 -> 0` (`0`, non-increase)

## Non-Increase Evidence
- Aggregate non-increase constraints were satisfied for non-target classes:
  - `E0425: 63 -> 63`
  - `E0599: 0 -> 0`
- Target-class reduction was satisfied:
  - `E0308: 73 -> 57`

## Validation
- Targeted tests:
  - `cargo test -p fragile-clang normalize_unresolved_iterator_return -- --nocapture`
- Full regression suites rerun before commit:
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
