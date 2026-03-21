# M9.2.c.iv.e.5.e Post-e.5.d Strict Compile Error Inventory

**Date**: 2026-03-20
**Scope**: `rrr/base/{debugging,misc}.cpp` compiled with release fragilec strict mode
**Previous inventory**: M9.2.c.iv.e.3.f.2 (296/297 errors)
**Current total**: 294 (debugging.cpp) / 295 (misc.cpp)
**Delta**: -2/-2 errors (~0.7% reduction)

## Summary

The e.5.a-d fixes eliminated E0603 (private field access, 4 errors) but the total only dropped by 2 because some of the targeted error classes (E0614, E0599, E0605) still have residual instances from different root causes than the ones fixed.

## Error Distribution (debugging.cpp = 294 errors)

| Error Code | Count | % | Description | Change vs f.2 |
|-----------|-------|---|-------------|---------------|
| E0425 | 194 | 66.0% | Cannot find value/type in scope | unchanged |
| E0308 | 42 | 14.3% | Mismatched types | +2 |
| E0368 | 12 | 4.1% | Binary assignment op on wrong type | -4 |
| E0614 | 10 | 3.4% | Type cannot be dereferenced | unchanged |
| E0605 | 5 | 1.7% | Non-primitive cast | unchanged |
| E0599 | 5 | 1.7% | No method found | -4 |
| E0606 | 2 | 0.7% | Invalid cast | new |
| E0596 | 2 | 0.7% | Cannot borrow as mutable | new |
| E0560 | 2 | 0.7% | Struct has no field | new |
| E0282 | 2 | 0.7% | Type annotations needed | new |
| E0277 | 2 | 0.7% | Trait bound not satisfied | new |
| E0071 | 2 | 0.7% | Expected struct, found primitive | new |
| E0790 | 1 | 0.3% | Ambiguous trait call | new |
| E0658 | 1 | 0.3% | Unsafe fields experimental | new |
| E0618 | 1 | 0.3% | Expected function, found integer | new |
| E0609 | 1 | 0.3% | No field on type | new |
| E0435 | 1 | 0.3% | Non-constant in constant expr | new |
| E0255 | 1 | 0.3% | Name defined multiple times | new |
| E0223 | 1 | 0.3% | Ambiguous associated type | new |
| E0080 | 1 | 0.3% | Constant overflow | new |

misc.cpp has identical distribution plus E0428 (1, duplicate definition).

## Dominant Error Class: E0425 (194 errors, 66%)

### Sub-categories
- **`__fsv___func___x_0` scope bug**: 186 errors (96% of E0425). Function-static variable normalizer produces references to `__fsv___func___x_0` inside every `__func__` expansion, but the variable is defined only once per function scope and cross-references into nested blocks/lambdas fail. This is a single codegen bug that inflates the error count massively.
- **`__c`**: 2 errors. Unresolved variable reference in template expansion.
- **`_Schrage`/`_Part`/`_Full`**: 3 errors. Linear congruential engine template parameters.
- **`__imp` type**: 2 errors. Missing type definition.
- **`__make_unsigned_type_parameter_0_0_` type**: 1 error. Unresolved template parameter type.

### Actionable insight
Fixing the `__fsv___func___x_0` scope bug alone would reduce total errors by 186 (63%), from 294 → 108.

## E0308 Sub-categories (42 errors)

| Sub-category | Count | Pattern |
|-------------|-------|---------|
| numpunct placeholder | 6 | `expected ()`, found `numpunct_type_parameter_0_0` |
| numpunct return type | 4 | `expected std_string`, found `numpunct_type_parameter_0_0` |
| `&c_void` vs `&{integer}` | 8 | `expected &c_void/&numpunct...`, found `&{integer}` |
| chrono duration | 4 | `expected chrono_nanoseconds/chrono_duration_...`, found `i64` |
| iterator types | 4 | `expected _InputIterator/_OutputIterator`, found `()` |
| partial/weak ordering | 3 | ordering type mismatches |
| enum/integer coercion | 4 | `expected u32`, found enum types |
| `u32`/`u64` width | 2 | `expected u64, found u32` and vice versa |
| `*mut c_void` vs `*mut ()` | 1 | pointer void type mismatch |
| tuple/template params | 1 | `expected tuple_DefaultType...`, found `_MArgs___` |
| wrap_iter return | 1 | `expected std___wrap_iter_double`, found `()` |
| misc | 4 | other type mismatches |

## E0368 Sub-categories (12 errors)

All 12: `+=` cannot be applied to type `()`. These are iterator `+=` operations where the iterator type resolves to `()` instead of the actual wrap_iter type (different from the E0368s fixed in e.4, which were for correctly-typed wrap_iters).

## E0614 Sub-categories (10 errors)

| Type | Count | Pattern |
|------|-------|---------|
| `isize` deref | 6 | Pointer arithmetic returning isize instead of pointer |
| `i8` deref | 2 | Byte pointer arithmetic |
| `usize` deref | 1 | Size value instead of pointer |
| `param_type` deref | 1 | Struct type (not pointer) |

## E0599 Sub-categories (5 errors)

| Method | Type | Notes |
|--------|------|-------|
| `op_call` | `std_function_void___` | Still missing despite e.5.a fix (different call path) |
| `do_put` | `&std_time_put_char_` | Reference-qualified call not matching impl |
| `do_put` | `&std_time_put_wchar_t_` | Same as above |
| `do_get` | `&std_time_get_char_` | Same as above |
| `do_get` | `&std_time_get_wchar_t_` | Same as above |

## New Small Error Classes (14 errors total)

These were previously masked by earlier error classes (compiler stops early):

- **E0606** (2): Invalid reference-to-pointer cast (`&mut *mut ()` as `*mut i8`)
- **E0596** (2): Immutable self borrow in mutable context
- **E0560** (2): Struct field not found (`__bitset_0_0_.__first_`, `bernoulli_distribution.__base`)
- **E0282** (2): Type inference failure
- **E0277** (2): `u32 % u64` and `u64 + u32` integer width mismatch in arithmetic
- **E0071** (2): `usize` used as struct constructor
- **E0790** (1): `Default::default()` without impl type specification
- **E0658** (1): `unsafe { __fsv___func___x_0 }` parsed as unsafe field (syntax)
- **E0618** (1): `__gv_min` called as function but resolved as `i64` static
- **E0609** (1): No field `default_error_condition` on `()` (vtable method call on void)
- **E0435** (1): Non-constant value in const expression (byte bitwise-not)
- **E0255** (1): `__gv_swap` defined twice (generic + specific)
- **E0223** (1): Ambiguous associated type on bitset
- **E0080** (1): u64 overflow in numeric power-of-10 table (u128 approximation)

## Priority Assessment for Next Fix Cycle

1. **E0425 `__fsv___func___x_0`** (186 errors): Single highest-impact fix. The function-static variable normalizer needs scope-aware generation.
2. **E0308 numpunct/chrono** (14 errors): Template parameter resolution for locale/time types.
3. **E0368 iterator `()` type** (12 errors): Iterator return type not propagating through template instantiation.
4. **E0614 pointer arithmetic** (10 errors): Remaining deref-on-non-pointer from incomplete offset_from/sub patterns.
5. **Everything else** (72 errors): Diminishing returns; mostly template/trait resolution edge cases.
