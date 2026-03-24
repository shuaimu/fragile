# M9.2.c.iv.e.17.b — Unit Passthrough Param Return Type Fix (E0308)

## Task Scope (<1000 LOC)

Selected bounded E0308 sub-cluster from the post-e.16 inventory:
- unit-degraded passthrough parameters returned from non-unit functions,
- observed as `expected _InputIterator, found ()` in strict replay output.

Implementation footprint:
- `crates/fragile-clang/src/ast_codegen.rs`
  - add `normalize_unit_passthrough_param_return_types`.
  - wire pass into the normalization pipeline.
  - add focused unit tests.

The implementation is bounded and localized (well under 1000 LOC).

## Design

### Root Cause

Degraded lowering can emit signatures like:

```rust
pub fn get(&self, mut __b: (), ...) -> _InputIterator {
    return __b;
}
```

This is a direct type-flow mismatch: a unit-typed parameter is returned through a non-unit function lane.

### Fix

`normalize_unit_passthrough_param_return_types`:
- scans function signatures and bodies,
- detects `return IDENT;` where `IDENT` is a unit-typed parameter,
- rewrites only that parameter type from `()` to the function return type,
- skips functions with lvalue-rebind artifacts for that parameter (for example `&mut ({ let __v = __s; __v })`) to avoid widening into incompatible store sites.

This keeps the fix generic and avoids target-specific behavior.

## Wrong-Approach Check

Checked against:
- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No forbidden patterns were introduced:
- no target-specific hacks,
- no native bypass,
- no semantic stub injection,
- no rollback-pattern additions.

## Strict Replay Evidence

Compile profile:

```bash
FRAGILEC_MODE=strict ./target/release/fragilec -c vendor/mako/src/rrr/base/debugging.cpp -std=gnu++23 -w \
  -I vendor/mako/src \
  -I vendor/mako/src/rrr \
  -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako \
  -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1
```

Artifacts:
- baseline stderr: `/tmp/fragile_e17b_baseline_aofmau/debugging.stderr`
- post-fix stderr: `/tmp/fragile_e17b_after2_57VSuK/debugging.stderr`

Delta:
- total rustc errors: `46 -> 44`
- `E0308`: `18 -> 16`
- `_InputIterator` mismatches (`expected _InputIterator, found ()`): `2 -> 0`
- `_OutputIterator` mismatches (`expected _OutputIterator, found ()`): `2 -> 2` (unchanged)

## Regression Tests Added

In `crates/fragile-clang/src/ast_codegen.rs`:
- `test_normalize_unit_passthrough_param_return_types_rewrites_returned_unit_param_to_return_type`
- `test_normalize_unit_passthrough_param_return_types_rewrites_only_returned_params`
- `test_normalize_unit_passthrough_param_return_types_skips_lvalue_rebind_artifacts`
- `test_normalize_unit_passthrough_param_return_types_skips_non_identifier_returns`
