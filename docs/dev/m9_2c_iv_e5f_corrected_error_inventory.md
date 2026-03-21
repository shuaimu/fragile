# M9.2.c.iv.e.5.f Corrected Strict Compile Error Inventory

**Date**: 2026-03-20
**Scope**: `vendor/mako/src/rrr/base/{debugging,misc}.cpp` compiled with release fragilec strict mode
**Compile flags**: `-std=gnu++23 -DGTEST_HAS_PTHREAD=1 -w` with full mako include tree (matching `mako_compile_args()` in test harness)
**Previous inventory**: M9.2.c.iv.e.5.e (294/295 errors — **used incorrect `-std=c++17` and partial include paths**)
**Current total**: 410 (debugging.cpp) / ~410 (misc.cpp)

## Critical Finding

The M9.2.c.iv.e.5.e inventory was collected using incorrect compile flags (`-std=c++17` with only `-I vendor/mako/rpc -I vendor/mako`), which didn't match the actual test harness configuration (`-std=gnu++23` with 8 include directories). The correct compile profile reveals:

1. **E0425 `__fsv___func___x_0` is already fixed** — only 15 E0425 errors remain, none involving `__fsv___func_`. The function-scoped normalizer (implemented in prior commits) successfully resolved this class.
2. **Total error count is higher** (410 vs 294) because the correct C++ standard and include paths cause more code to be parsed and compiled, exposing more downstream errors.
3. **E0308 is now dominant** (179 errors, 44%) instead of E0425.

## Error Distribution (debugging.cpp = 410 errors)

| Error Code | Count | % | Description |
|-----------|-------|---|-------------|
| E0308 | 179 | 43.7% | Mismatched types |
| E0277 | 79 | 19.3% | Trait bound not satisfied |
| E0599 | 57 | 13.9% | No method found for type |
| E0609 | 28 | 6.8% | No field on type |
| E0425 | 15 | 3.7% | Cannot find value/function |
| E0428 | 9 | 2.2% | Name defined multiple times |
| E0610 | 7 | 1.7% | Cannot apply unary `*` |
| E0606 | 4 | 1.0% | Invalid cast |
| E0592 | 4 | 1.0% | Conflicting implementations |
| E0117 | 4 | 1.0% | Only traits defined in crate can be implemented |
| E0530 | 3 | 0.7% | Match arm binds enum by name |
| E0433 | 3 | 0.7% | Failed to resolve path |
| E0119 | 3 | 0.7% | Conflicting trait implementation |
| E0071 | 3 | 0.7% | Expected struct, found primitive |
| E0061 | 3 | 0.7% | Incorrect number of arguments |
| E0116 | 2 | 0.5% | Cannot define inherent impl for foreign type |
| E0614 | 1 | 0.2% | Type cannot be dereferenced |
| E0596 | 1 | 0.2% | Cannot borrow as mutable |
| E0560 | 1 | 0.2% | Struct has no field |
| E0424 | 1 | 0.2% | `self` is not available |
| E0423 | 1 | 0.2% | Expected value, found struct |
| E0368 | 1 | 0.2% | Binary op on wrong type |
| E0255 | 1 | 0.2% | Name defined multiple times |

misc.cpp has nearly identical distribution (176 E0308 vs 179).

## E0425 Breakdown (15 errors)

| Sub-category | Count | Pattern |
|-------------|-------|---------|
| `__a` unresolved | 4 | Template parameter in algorithm body |
| `__c` unresolved | 1 | Template parameter in algorithm body |
| `move_ptr_mut_*_u64` | 5 | Missing `char_traits` module functions |
| `__to_xstring_*` | 3 | Long mangled function name unresolved |
| `current_exception` | 1 | STL exception function |
| `__` | 1 | Short identifier unresolved |

**No `__fsv___func___x_0` errors remain** — this class is fully resolved.

## Priority Assessment for Next Fix Cycle

1. **E0308** (179 errors, 44%): Dominant class. Sub-categorization needed.
2. **E0277** (79 errors, 19%): Trait bounds — likely `AddAssign`/`SubAssign`/arithmetic for types that don't impl them.
3. **E0599** (57 errors, 14%): Missing methods on generated types.
4. **E0609** (28 errors, 7%): Field access on types without the field.
5. **E0425** (15 errors, 4%): Remaining scope resolution issues (diverse, no single dominant pattern).
