# M9.2.c.iv.e.23 namespace-alias dedup bounded inventory

## Scope
- Leaf: `M9.2.c.iv.e.23`
- Goal: execute one bounded post-e.22 reduction against dominant residual `E0425` while keeping non-target classes non-increasing.

## Task Sizing
- Change scope is localized to one post-processing normalizer (`normalize_duplicate_type_alias_struct_definitions`) plus focused tests.
- Estimated and actual implementation footprint is well under 1000 LOC.

## Plan Before Execution
1. Re-run strict inventory on `debugging/misc/basetypes/logging` with M9 harness compile args.
2. Select one dominant repeated residual class slice.
3. Apply one bounded generic fix in `ast_codegen`.
4. Add focused unit coverage for the repaired lane.
5. Rebuild release `fragilec`, rerun strict inventory, and publish deterministic deltas.

## Wrong-Approach Check
- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Re-reviewed `docs/dev/wrong.md`.
- No target-specific conditionals, no force-native bypasses, no semantic stubs, and no rollback-pattern additions were introduced.

## Baseline (Pre-fix)
- Run-root: `/tmp/fragile_e23_before_DWjhRe`
- Compile profile: `FRAGILEC_MODE=strict`, `-std=gnu++23`, include/define set from `mako_compile_args`.

Per-file counts (`total / E0308 / E0425 / E0599 / E0609 / E0277`):
- `debugging`: `48 / 16 / 17 / 0 / 1 / 2`
- `misc`: `48 / 16 / 17 / 0 / 1 / 2`
- `basetypes`: `38 / 17 / 6 / 0 / 1 / 2`
- `logging`: `66 / 24 / 23 / 0 / 1 / 2`

Aggregate:
- `total=200`, `E0308=73`, `E0425=63`, `E0599=0`, `E0609=4`, `E0277=8`

## Dominant Cluster
Baseline top `E0425` messages:
- `cannot find type Job in this scope`: 24
- `cannot find type OneTimeJob in this scope`: 12
- `cannot find type NoCopy in this scope`: 3
- `cannot find type SpinLock in this scope`: 3

Selected bounded slice:
- namespace alias-loss lane caused by duplicate type-alias normalization.

## Root Cause
- `normalize_duplicate_type_alias_struct_definitions` collected struct names from all scopes based on trimmed lines.
- Indented module-local struct definitions (for example inside `pub mod rrr`) were treated as conflicts with top-level export aliases (for example `pub type Job = rrr::Job`), and those aliases were removed.
- Downstream generated vtable/function signatures still referenced top-level alias names (`Job`, `OneTimeJob`, etc.), producing `E0425` unresolved type errors.

## Implementation
- Updated `normalize_duplicate_type_alias_struct_definitions` to only collect conflicting struct names from true top-level (column-zero) definitions.
- Preserved existing behavior for real top-level alias/struct conflicts.
- Added focused unit test coverage:
  - `test_normalize_duplicate_type_alias_struct_preserves_namespaced_export_alias`

## Post-fix Results
- Run-root: `/tmp/fragile_e23_after_nwHCvl`

Per-file counts (`total / E0308 / E0425 / E0599 / E0609 / E0277`):
- `debugging`: `36 / 16 / 5 / 0 / 1 / 2`
- `misc`: `36 / 16 / 5 / 0 / 1 / 2`
- `basetypes`: `38 / 17 / 6 / 0 / 1 / 2`
- `logging`: `48 / 24 / 5 / 0 / 1 / 2`

Aggregate delta vs baseline:
- `total: 200 -> 158` (`-42`)
- `E0425: 63 -> 21` (`-42`)
- `E0308: 73 -> 73` (`0`, non-increase)
- `E0599: 0 -> 0` (`0`, non-increase)
- `E0609: 4 -> 4` (`0`, non-increase)
- `E0277: 8 -> 8` (`0`, non-increase)

Post-fix top `E0425` messages:
- `cannot find value __c in this scope`: 8
- `cannot find type __imp in this scope`: 8
- `cannot find type __make_unsigned_type_parameter_0_0_ in this scope`: 4
- `cannot find function sleep_for_chrono_nanoseconds_chrono_nanoseconds in module this_thread`: 1

## Non-Increase Evidence
- Non-target classes remained non-increasing:
  - `E0308: 73 -> 73`
  - `E0599: 0 -> 0`
  - `E0609: 4 -> 4`
  - `E0277: 8 -> 8`
- Target class reduced:
  - `E0425: 63 -> 21`

## Validation
- Targeted tests:
  - `cargo test -p fragile-clang normalize_duplicate_type_alias_struct -- --nocapture`
- Full regression suites:
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
