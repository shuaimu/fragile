# M9.2.c.iv.e.22 ios_base state-field normalization slice

## Scope
- Leaf: `M9.2.c.iv.e.22`
- Goal: execute one bounded post-e.21 reduction targeting the dominant residual `E0609` field-access cluster while keeping other dominant classes non-increasing.

## Task Sizing
- The selected fix is localized to one existing post-processing normalizer (`normalize_ios_istream_missing_fields`) plus focused unit tests.
- Estimated and actual implementation scope stays well below 1000 LOC.

## Plan Before Execution
1. Re-run strict inventory on `debugging/misc/basetypes/logging` with M9 harness compile args.
2. Identify one dominant repeated `E0609` sub-cluster.
3. Implement one bounded generic normalization fix in `ast_codegen`.
4. Add focused tests for the normalized field access lane.
5. Rebuild release `fragilec`, rerun strict inventory, and publish deterministic deltas.

## Wrong-Approach Check
- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Re-reviewed `docs/dev/wrong.md`.
- No target-specific conditionals, no force-native bypasses, no semantic source stubs, and no rollback-pattern additions were introduced.

## Baseline (Pre-fix)
- Run-root: `/tmp/fragile_e22_before_jJNoYZ`
- Compile profile: `FRAGILEC_MODE=strict`, `-std=gnu++23`, include/define set from `mako_compile_args`.

Per-file counts (`total / E0308 / E0425 / E0599 / E0609 / E0277`):
- `debugging`: `57 / 16 / 17 / 0 / 10 / 2`
- `misc`: `57 / 16 / 17 / 0 / 10 / 2`
- `basetypes`: `47 / 17 / 6 / 0 / 10 / 2`
- `logging`: `75 / 24 / 23 / 0 / 10 / 2`

Aggregate:
- `total=236`, `E0308=73`, `E0425=63`, `E0599=0`, `E0609=40`, `E0277=8`

## Dominant Cluster
`E0609` split in baseline:
- `no field _M_streambuf_state on &ios_base/&mut ios_base`: 36 occurrences
- `no field default_error_condition on ()`: 4 occurrences

Selected bounded slice:
- the 36-count `_M_streambuf_state` lane.

## Root Cause
- `normalize_ios_istream_missing_fields` rewrote `.__rdstate_` accesses to `._M_streambuf_state`.
- Generated `ios_base` lane still exposes `__rdstate_`, so rewritten field accesses fail with `E0609`.

## Implementation
- Updated `normalize_ios_istream_missing_fields` to normalize `._M_streambuf_state` accesses back to `.__rdstate_`.
- Kept existing `_M_gcount` injection and `_M_cache_locale` stubbing behavior unchanged.
- Updated focused unit coverage:
  - `test_normalize_ios_fixes_streambuf_state_to_rdstate`

## Post-fix Results
- Run-root: `/tmp/fragile_e22_after_LyAa8r`

Per-file counts (`total / E0308 / E0425 / E0599 / E0609 / E0277`):
- `debugging`: `48 / 16 / 17 / 0 / 1 / 2`
- `misc`: `48 / 16 / 17 / 0 / 1 / 2`
- `basetypes`: `38 / 17 / 6 / 0 / 1 / 2`
- `logging`: `66 / 24 / 23 / 0 / 1 / 2`

Aggregate delta vs baseline:
- `total: 236 -> 200` (`-36`)
- `E0609: 40 -> 4` (`-36`)
- `E0308: 73 -> 73` (`0`, non-increase)
- `E0425: 63 -> 63` (`0`, non-increase)
- `E0599: 0 -> 0` (`0`, non-increase)
- `E0277: 8 -> 8` (`0`, non-increase)

## Non-Increase Evidence
- Non-target classes remained non-increasing:
  - `E0308: 73 -> 73`
  - `E0425: 63 -> 63`
  - `E0599: 0 -> 0`
  - `E0277: 8 -> 8`
- Target class reduced:
  - `E0609: 40 -> 4`

## Validation
- Targeted tests:
  - `cargo test -p fragile-clang normalize_ios_ -- --nocapture`
- Full regression suites (post-change):
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
