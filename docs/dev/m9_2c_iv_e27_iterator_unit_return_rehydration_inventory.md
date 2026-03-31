# M9.2.c.iv.e.27 iterator unit-return rehydration inventory

## Scope
- Leaf: `M9.2.c.iv.e.27`
- Goal: execute one bounded post-e.26 reduction against the dominant newly-visible `E0308` mismatch class.

## Task Sizing
- Change scope is one post-processing normalizer plus focused unit tests.
- Estimated and actual implementation footprint is below 1000 LOC.

## Plan Before Execution
1. Re-run strict replay on `debugging/misc/basetypes/logging` with harness-equivalent `fragilec` compile commands.
2. Isolate one dominant repeated `E0308` pattern family.
3. Apply one generic post-processing fix in `ast_codegen`.
4. Add focused unit tests.
5. Rebuild release `fragilec`, rerun strict replay, and publish deterministic deltas.

## Wrong-Approach Check
- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Re-reviewed `docs/dev/wrong.md`.
- No rollback-pattern additions, no target-specific bypasses, no semantic stubs, and no force-native escape hatch usage were introduced.

## Baseline (Pre-fix)
- Run-root: `/tmp/fragile_e27_before_r_7r6ghn`
- Compile profile: `FRAGILEC_MODE=strict`, `FRAGILEC_PARSER_BACKEND=fragile-parser-clang`, `gnu++23` args from `vendor/mako/build_rpc_fragilec_make_20260311/compile_commands.json`.

Per-file counts (`total / E0308 / E0425 / E0599 / E0609 / E0277 / E0614 / E0618`):
- `debugging`: `10 / 6 / 0 / 1 / 0 / 0 / 0 / 0`
- `misc`: `10 / 6 / 0 / 1 / 0 / 0 / 0 / 0`
- `basetypes`: `11 / 7 / 0 / 1 / 0 / 0 / 0 / 0`
- `logging`: `18 / 14 / 0 / 1 / 0 / 0 / 0 / 0`

Aggregate:
- `total=49`, `E0308=33`, `E0425=0`, `E0599=4`, `E0609=0`, `E0277=0`, `E0614=0`, `E0618=0`

## Dominant Cluster
Top repeated `E0308` pair markers:
- `expected _InputIterator, found ()`: `8`
- `expected (), found _InputIterator`: `8`

Observed shape:
- method signatures degraded to `-> ()` while bodies still returned iterator params (`return __b;` / `return __s;`) and assigned `self.get(...)`/`self.put(...)` into iterator vars.

## Root Cause
- Earlier iterator-return degradation converted `_InputIterator`/`_OutputIterator` return types to `()`.
- In `time_get`/`time_put` lanes, function bodies still explicitly returned iterator params.
- This created paired type mismatches in call assignment and return statements.

## Implementation
- Added `normalize_iterator_unit_return_types_from_body_returns` in `ast_codegen`.
- The pass:
1. Targets only function signatures with `-> ()` and iterator params (`__b: _InputIterator` or `__s: _OutputIterator`).
2. Scans the function body and rewrites the signature to iterator return type only if it finds `return __b;` or `return __s;`.
3. Leaves unrelated unit-return functions unchanged.

Pipeline wiring:
- Invoked immediately after `normalize_e26_residual_errors` so it repairs final emitted shape.

Focused unit tests added:
- `test_normalize_iterator_unit_return_rehydrates_input_iterator`
- `test_normalize_iterator_unit_return_rehydrates_output_iterator`
- `test_normalize_iterator_unit_return_preserves_non_returning_body`
- `test_normalize_iterator_unit_return_noop_when_already_iterator`

## Post-fix Results
- Run-root: `/tmp/fragile_e27_after_kf4i8f86`

Per-file counts (`total / E0308 / E0425 / E0599 / E0609 / E0277 / E0614 / E0618`):
- `debugging`: `6 / 2 / 0 / 1 / 0 / 0 / 0 / 0`
- `misc`: `6 / 2 / 0 / 1 / 0 / 0 / 0 / 0`
- `basetypes`: `7 / 3 / 0 / 1 / 0 / 0 / 0 / 0`
- `logging`: `14 / 10 / 0 / 1 / 0 / 0 / 0 / 0`

Aggregate delta vs baseline:
- `total: 49 -> 33` (`-16`)
- `E0308: 33 -> 17` (`-16`)
- `E0599: 4 -> 4` (`0`, non-increase)
- `E0425/E0609/E0277/E0614/E0618`: unchanged at `0`

Pair-marker deltas:
- `expected _InputIterator, found ()`: `8 -> 0`
- `expected (), found _InputIterator`: `8 -> 0`

## Non-Increase Evidence
- Non-target classes remained non-increasing:
  - `E0599: 4 -> 4`
  - `E0425: 0 -> 0`
  - `E0609: 0 -> 0`
  - `E0277: 0 -> 0`
  - `E0614: 0 -> 0`
  - `E0618: 0 -> 0`
- Target class reduced:
  - `E0308: 33 -> 17`

## Validation
- Focused test slice:
  - `cargo test -p fragile-clang normalize_iterator_unit_return_ -- --nocapture`
- Full regression suites (executed after implementation):
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
