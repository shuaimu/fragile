# M9.2.c.iv.e.3.f.2: Post-f.1 Strict Compile Error Inventory

Date: 2026-03-20
Files: `vendor/mako/src/rrr/base/debugging.cpp`, `vendor/mako/src/rrr/base/misc.cpp`
Compiler: `fragilec` (release build, post M9.2.c.iv.e.3.f.1 char_traits i8 fix)

## Summary

| File | Total Errors | Unique Error Codes |
|------|-------------|-------------------|
| debugging.cpp | 296 | 20 |
| misc.cpp | 297 | 21 |

Both files have nearly identical error profiles (misc.cpp has one extra E0428).

## Error Code Distribution (debugging.cpp)

| Error Code | Count | Category |
|-----------|-------|----------|
| E0425 | 194 | Unresolved name/value |
| E0308 | 40 | Type mismatch |
| E0368 | 16 | Binary assign op on wrong type |
| E0614 | 10 | Deref non-pointer type |
| E0599 | 9 | No method found |
| E0605 | 5 | Non-primitive cast |
| E0603 | 4 | Private field access |
| E0606 | 2 | Invalid cast (numeric) |
| E0596 | 2 | Cannot borrow immutable as mutable |
| E0560 | 2 | Struct has no field |
| E0282 | 2 | Type annotations needed |
| E0071 | 2 | Expected struct, found enum |
| E0790 | 1 | Cannot call associated fn without type |
| E0658 | 1 | Unstable feature |
| E0618 | 1 | Expected function, found value |
| E0609 | 1 | No field on type |
| E0435 | 1 | Non-constant in constant position |
| E0255 | 1 | Import conflicts with value |
| E0223 | 1 | Ambiguous associated type |
| E0080 | 1 | Const eval error |

## Dominant Error Classes

### 1. E0425: Unresolved Names (194 errors, 65%)

**Root cause**: `__fsv___func___x_0` — a function-static variable pattern where the normalizer
generates a unique name per function, but the generated name is not resolvable from the scope
where it's referenced. 186 of 194 E0425 errors are this single pattern.

Remaining E0425 (8 errors):
- `__c` (2) — unresolved char variable
- `__imp` type (2) — Windows-specific import symbol
- `_Schrage`, `_Part`, `_Full` (3) — linear congruential engine template constants
- `__make_unsigned_type_parameter_0_0_` (1) — unresolved template type param

### 2. E0308: Type Mismatches (40 errors, 13.5%)

Sub-categories:
- **numpunct placeholder** (6): `numpunct_type_parameter_0_0` expected `()` or `&{integer}`
- **chrono duration** (4): `chrono_duration_long_long__ratio_1__1000000000` vs `i64`, `chrono_nanoseconds` return
- **iterator types** (6): `_InputIterator`/`_OutputIterator` expected `()` or wrong return type
- **ordering types** (4): `partial_ordering` vs `weak_ordering`/`strong_ordering` in conversion fns
- **`&c_void` vs `&{integer}`** (4): void pointer dereference mismatch
- **enum-int coercion** (4): `u32` expected, found `memory_order`/`__rule`/`__GB9c_*`/`__GB11_*`
- **misc** (12): `tuple_DefaultType` vs `_MArgs`, `std_mt19937` init, `std_atomic_i64` init,
  `*mut c_void` vs `usize`, `FragileVaList` vs `VaList`, `f64` vs integer, `std_string` return, `()` vs iterator

### 3. E0368: Binary Assign Op (16 errors, 5.4%)

- `+= on ()` (12): iterator arithmetic on unresolved `()` type (iterator type not resolved)
- `+= on std___wrap_iter_double` (3): iterator types lack `AddAssign` impl
- `+= on std___wrap_iter_const_double` (1): same

### 4. E0614: Deref Non-Pointer (10 errors, 3.4%)

- `isize` cannot be dereferenced (6): pointer arithmetic returns isize instead of pointer
- `i8` cannot be dereferenced (2): char pointer dereference
- `usize` cannot be dereferenced (1)
- `param_type` cannot be dereferenced (1)

### 5. E0599: No Method (9 errors, 3%)

- `std_mt19937::op_call` (3): missing operator() on Mersenne Twister
- `std_function_void___::op_call` (1): missing operator() on std::function
- `std_time_put_*::do_put` (2): missing locale facet methods
- `std_time_get_*::do_get` (2): missing locale facet methods
- `param_type::p` (1): missing method

## Comparison with Pre-f.1 Inventory

| Metric | Pre-f.1 (est.) | Post-f.1 | Delta |
|--------|---------------|----------|-------|
| Total errors | ~383 | 296 | -87 (-23%) |
| E0308 | ~52 | 40 | -12 |
| E0425 | ~194 | 194 | 0 |
| E0368 | ~16 | 16 | 0 |

The f.1 char_traits fix eliminated 12 E0308 errors (the `Self::lt`/`Self::eq` i8 mismatch family).
The total dropped from ~383 to 296. The dominant class is now E0425 (`__fsv___func___x_0`).

## Recommended Next Leaf Closures

Priority order for maximum error reduction:

1. **Fix `__fsv___func___x_0` scope resolution** (186 E0425 errors = 63% of all errors).
   This is a single codegen bug where function-static variable names leak scope.

2. **Iterator arithmetic** (16 E0368 + 6 E0308 + 10 E0614 = 32 errors).
   Iterator types lack `AddAssign` and pointer arithmetic resolves to `isize` not pointer.

3. **Ordering type conversions** (4 E0308 errors).
   `weak_ordering`/`strong_ordering` → `partial_ordering` conversion returns wrong type.

4. **Enum-int coercion** (4 E0308 errors).
   `memory_order`, `__rule`, etc. used where `u32` expected.

5. **chrono duration** (4 E0308 errors).
   `__gv_max`/`__gv_min` globals typed as `i64` instead of `chrono_duration`.
