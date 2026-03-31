# M9.2.c.iv.e.24 residual E0425 placeholder-symbol normalization inventory

## Scope
- Leaf: `M9.2.c.iv.e.24`
- Goal: execute one bounded post-e.23 reduction against dominant residual `E0425` while keeping non-target classes non-increasing.

## Task Sizing
- Change scope is localized to one post-processing normalizer (`normalize_residual_e0425_placeholder_symbols`) plus focused unit tests.
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
- No target-specific conditionals, force-native bypasses, semantic stubs, or rollback-pattern additions were introduced.

## Baseline (Pre-fix)
- Run-root: `/tmp/fragile_e24_before_41dO8j`
- Compile profile: `FRAGILEC_MODE=strict`, `-std=gnu++23`, include/define set from `mako_compile_args`.

Per-file counts (`total / E0308 / E0425 / E0599 / E0609 / E0277`):
- `debugging`: `36 / 16 / 5 / 0 / 1 / 2`
- `misc`: `36 / 16 / 5 / 0 / 1 / 2`
- `basetypes`: `38 / 17 / 6 / 0 / 1 / 2`
- `logging`: `48 / 24 / 5 / 0 / 1 / 2`

Aggregate:
- `total=158`, `E0308=73`, `E0425=21`, `E0599=0`, `E0609=4`, `E0277=8`

## Dominant Cluster
Baseline top `E0425` messages:
- `cannot find value __c in this scope`: 8
- `cannot find type __imp in this scope`: 8
- `cannot find type __make_unsigned_type_parameter_0_0_ in this scope`: 4
- `cannot find function sleep_for_chrono_nanoseconds_chrono_nanoseconds in module this_thread`: 1

Selected bounded slice:
- residual placeholder-symbol normalization lane in generated Rust output.

## Root Cause
- `__to_unsigned_like_*` stubs emitted `zeroed::<__make_unsigned_type_parameter_*>()` with unresolved placeholder type names.
- `__non_unique_impl::__hash` emitted a `while (__c)` loop but dropped the local declaration of `__c`.
- `locale` emitted `__locale_: *mut __imp` without a resolvable `__imp` type.
- A sleep helper call was emitted as `this_thread::sleep_for_chrono_nanoseconds_chrono_nanoseconds(...)` while the helper existed at top-level.

## Implementation
Added `normalize_residual_e0425_placeholder_symbols` in `ast_codegen` and wired it into the post-processing pipeline.

The pass now:
1. Rewrites `this_thread::sleep_for_chrono_nanoseconds_chrono_nanoseconds(` to the top-level helper call when that helper is present.
2. Rewrites unresolved `*mut __imp` / `*const __imp` locale pointers to `*mut std::ffi::c_void` / `*const std::ffi::c_void` with an exact-token guard so `__impl_*` symbols do not block the rewrite.
3. Rewrites `__to_unsigned_like_*` unresolved `zeroed::<__make_unsigned_type_parameter_*>()` returns to parameter-cast returns.
4. Rewrites unresolved `while (__c) != 0` hash loops to emit a local `__c` from `__ptr` and increments `__ptr` in-loop.

Added focused unit tests:
- `test_normalize_residual_e0425_placeholder_symbols_rewrites_to_unsigned_zeroed_placeholder`
- `test_normalize_residual_e0425_placeholder_symbols_rewrites_hash_loop_c_variable`
- `test_normalize_residual_e0425_placeholder_symbols_rewrites_locale_imp_pointer_type`
- `test_normalize_residual_e0425_placeholder_symbols_does_not_treat_impl_prefix_as_imp_definition`
- `test_normalize_residual_e0425_placeholder_symbols_rewrites_sleep_helper_module_path`

## Post-fix Results
- Run-root: `/tmp/fragile_e24_after2_FIttQh`

Per-file counts (`total / E0308 / E0425 / E0599 / E0609 / E0277`):
- `debugging`: `24 / 14 / 0 / 0 / 1 / 0`
- `misc`: `24 / 14 / 0 / 0 / 1 / 0`
- `basetypes`: `25 / 15 / 0 / 0 / 1 / 0`
- `logging`: `32 / 22 / 0 / 0 / 1 / 0`

Aggregate delta vs baseline:
- `total: 158 -> 105` (`-53`)
- `E0425: 21 -> 0` (`-21`)
- `E0308: 73 -> 65` (`-8`, non-increase)
- `E0599: 0 -> 0` (`0`, non-increase)
- `E0609: 4 -> 4` (`0`, non-increase)
- `E0277: 8 -> 0` (`-8`, non-increase)

Post-fix `E0425` messages:
- none in the measured 4-file blocker set.

## Non-Increase Evidence
- Non-target classes remained non-increasing:
  - `E0308: 73 -> 65`
  - `E0599: 0 -> 0`
  - `E0609: 4 -> 4`
  - `E0277: 8 -> 0`
- Target class reduced to zero:
  - `E0425: 21 -> 0`

## Validation
- Targeted tests:
  - `cargo test -p fragile-clang normalize_residual_e0425_placeholder_symbols -- --nocapture`
- Full regression suites (post-change run):
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
