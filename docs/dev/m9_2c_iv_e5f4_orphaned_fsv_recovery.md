# M9.2.c.iv.e.5.f.4: Orphaned `__fsv___func___x_0` Recovery

## Problem

After f.3's owner-stable function-static alias mapping, 40 `__fsv___func___x_0`
unresolved references remained per file. These were "orphaned" references —
`__fsv___func_` symbols appearing in function bodies that had no matching static
declaration in scope.

## Root Cause

Cross-function alias collision in the transpiler's codegen layer:

1. Function A (e.g. `__seed()`) declares `static mut __fsv___func___x_0: i8`
2. Function B (e.g. `op_eq(__x: &error_condition, ...)`) has parameter `__x`
3. The codegen layer incorrectly produces `__fsv___func___x_0` references in B's
   body instead of using the `__x` parameter directly
4. The owner-stable normalizer correctly refuses to rewrite these (B has no
   matching static), leaving them as unresolved E0425 errors

## Fix

Added a **fourth pass** to `normalize_unprefixed_function_static_symbol_refs`
that scans the output for any remaining bare (unwrapped) `__fsv___func_` references
and replaces them back to the bare alias name (e.g. `__fsv___func___x_0` → `__x`).

The pass carefully preserves:
- Static declarations (`static mut __fsv___func_...`)
- Correctly-rewritten references inside `unsafe { __fsv___func_... }` wrappers

Only **bare** references — those NOT inside `unsafe { ... }` blocks — are recovered.

## Evidence

- 3 new unit tests: `recovers_orphaned_cross_function_refs`,
  `recovers_multiple_orphaned_refs`, `no_recovery_needed_when_owned`
- 3 M9 closure tests: `m9_2c_iv_e5f4_task_documented_in_todo`,
  `m9_2c_iv_e5f4_orphaned_recovery_documented`,
  `m9_2c_iv_e5f4_unit_tests_cover_orphaned_recovery`
- All 1070+ lib tests pass with 0 failures
