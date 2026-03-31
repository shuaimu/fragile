# M9.2.c.iv.e.26 time_get/time_put virtual lane and error_category vtable typing inventory

## Scope
- Leaf: `M9.2.c.iv.e.26`
- Goal: execute one bounded post-e.25 reduction on dominant residual `E0308`/`E0609` classes.

## Task Sizing
- Implementation is localized to two generic post-processing normalizations in `ast_codegen` plus focused unit coverage.
- Estimated and actual implementation size is below 1000 LOC.

## Plan Before Execution
1. Re-run strict inventory on `debugging/misc/basetypes/logging` with M9 harness-equivalent compile args.
2. Confirm dominant residual classes and select one bounded, generic slice.
3. Apply minimal, generic normalizer changes.
4. Add focused unit tests and TODO/doc contract regression checks.
5. Re-run strict inventory and publish deterministic deltas.

## Wrong-Approach Check
- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Re-reviewed `docs/dev/wrong.md`.
- No target-specific conditionals, no force-native bypasses, and no semantic fake-body hacks were introduced.

## Baseline (Pre-fix)
- Run-root: `/tmp/fragile_e26_before_97zU1I`
- Compile profile: `FRAGILEC_MODE=strict`, `-std=gnu++23`, include/define set matching `mako_compile_args`.

Per-file counts (`total / E0308 / E0609 / E0614 / E0277`):
- `debugging`: `24 / 14 / 1 / 2 / 0`
- `misc`: `24 / 14 / 1 / 2 / 0`
- `basetypes`: `25 / 15 / 1 / 2 / 0`
- `logging`: `32 / 22 / 1 / 2 / 0`

Aggregate:
- `total=105`, `E0308=65`, `E0609=4`, `E0614=8`, `E0277=0`

## Dominant Cluster
- `E0308` included repeated time facet virtual-lane mismatches from `time_get::get` / `time_put::put` call rewriting (`arguments to this method are incorrect`, wrong lane argument shapes, and unit-return mismatch cascades).
- `E0609` was a repeated `error_category` vtable field-lane failure (`default_error_condition` lookup on `*const ()`).

## Root Cause
1. `normalize_do_method_to_base_method` rewrote `self.do_get(...)`/`self.do_put(...)` to `self.get(...)`/`self.put(...)` purely by token, even when the owning impl type already had virtual `do_get`/`do_put` methods (often in a separate impl block).
2. `error_category` could be emitted with `__vtable: *const ()`, so downstream vtable member access in `default_error_condition` degraded to field access on unit.

## Implementation
1. Hardened `normalize_do_method_to_base_method`:
- Parse inherent impl blocks by type.
- Collect whether each type defines `do_get`/`do_put` in any impl block.
- Rewrite `self.do_get`/`self.do_put` only for types that truly lack those virtual methods.
- Preserve legacy snippet behavior for micro-inputs without impl blocks.

2. Added `normalize_error_category_vtable_pointer_type`:
- Rewrites `error_category` field lane `__vtable: *const ()` to `__vtable: *const error_category_vtable` when the vtable type is present.
- Scoped to `error_category` only (does not rewrite unrelated `*const ()` vtable fields).

Focused unit tests added:
- `test_normalize_do_method_to_base_method_preserves_do_get_when_type_has_virtual_stub`
- `test_normalize_do_method_to_base_method_preserves_do_put_when_type_has_virtual_stub`
- `test_normalize_error_category_vtable_pointer_type_rewrites_single_line_struct`
- `test_normalize_error_category_vtable_pointer_type_preserves_other_unit_vtable_fields`

## Post-fix Results
- Run-root: `/tmp/fragile_e26_after_No7p3v`

Per-file counts (`total / E0308 / E0609 / E0614 / E0277`):
- `debugging`: `19 / 10 / 0 / 2 / 0`
- `misc`: `19 / 10 / 0 / 2 / 0`
- `basetypes`: `20 / 11 / 0 / 2 / 0`
- `logging`: `27 / 18 / 0 / 2 / 0`

Aggregate delta vs baseline:
- `total: 105 -> 85` (`-20`)
- `E0308: 65 -> 49` (`-16`)
- `E0609: 4 -> 0` (`-4`)
- `E0614: 8 -> 8` (`0`, non-increase)
- `E0277: 0 -> 0` (`0`, non-increase)

Observed marker deltas:
- `arguments to this method are incorrect`: `16 -> 0`
- `default_error_condition` marker occurrences in stderr: `8 -> 0`

## Non-Increase Evidence
- Non-target classes were non-increasing:
  - `E0614: 8 -> 8`
  - `E0277: 0 -> 0`
- Target classes reduced:
  - `E0308: 65 -> 49`
  - `E0609: 4 -> 0`

## Validation
- Focused test slices:
  - `cargo test -p fragile-clang normalize_do_method_to_base_method -- --nocapture`
  - `cargo test -p fragile-clang normalize_error_category_vtable_pointer_type -- --nocapture`
- Full regression gates executed after implementation:
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
