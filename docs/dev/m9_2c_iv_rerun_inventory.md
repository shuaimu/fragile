# M9.2.c.iv Rerun Live Strict Compile Inventory

**Date**: 2026-03-21 (post e.5.h + rerun fixes)
**Compile profile**: `FRAGILEC_MODE=strict ./target/release/fragilec -c -std=gnu++23 -w -I vendor/mako/src -I vendor/mako/src/rrr -I vendor/mako/src/memdb -I vendor/mako/src/mako -I vendor/mako/test -I vendor/mako/third-party/rusty-cpp/include -I vendor/mako/third-party/googletest/googletest/include -I vendor/mako/third-party/googletest/googletest -DGTEST_HAS_PTHREAD=1`
**Parser backend**: `fragile-parser-clang` (default since M8.1)

## Per-File Error Counts

| File | Current | Previous (e.5 inventory) | Delta |
|------|---------|--------------------------|-------|
| debugging.cpp | 320 | 348 | -28 (-8.0%) |
| misc.cpp | 317 | 344 | -27 (-7.8%) |
| basetypes.cpp | 297 | 325 | -28 (-8.6%) |
| logging.cpp | 360 | 389 | -29 (-7.5%) |
| **TOTAL** | **1294** | **1406** | **-112 (-8.0%)** |

## Fixes Applied in This Rerun

### 1. basic_string `_M_set_length` / `_M_init_local_buf` method stubs (-40 E0599)
- **Root cause**: libc++ `basic_string` constructors and assignment operators call `_M_set_length()` and `_M_init_local_buf()`, but these methods were not defined on the transpiled `basic_string_*` struct types.
- **Fix**: `append_basic_string_internal_method_stubs()` appends impl blocks with method stubs for all 5 basic_string specializations (char, wchar_t, char8_t, char16_t, char32_t).
- **Impact**: 40 E0599 errors eliminated (2 methods x 5 types x 4 files).

### 2. ios_base fmtflags `_S_*` constant resolution (-32 E0599)
- **Root cause**: `ios_base::fmtflags` is type-aliased to `u32`, and the transpiler generated `std__Ios_Fmtflags::_S_boolalpha` etc. Since `std__Ios_Fmtflags` resolves to `u32`, these become `u32::_S_boolalpha` which is invalid (cannot add associated items to primitive types).
- **Fix**: `normalize_ios_base_fmtflags_primitive_associated_constants()` replaces `std__Ios_Fmtflags::_S_*` and `u32::_S_*` with the actual constant values from libc++ (e.g., `_S_boolalpha` -> `0x0001_u32`).
- **Impact**: 32 E0599 errors eliminated (8 constants x 4 files).

### 3. Secondary error reduction (-40)
- When E0599 errors are fixed, some downstream errors that depended on them (cascading type mismatches, etc.) are also eliminated.

## Error Category Breakdown (Aggregated Across 4 Files)

| Error Code | Count | Previous | Delta | Description |
|-----------|-------|----------|-------|-------------|
| E0308 | 701 | 701 | 0 | Type mismatches |
| E0599 | 111 | 223 | -112 (-50%) | No method found |
| E0609 | 116 | 116 | 0 | No field on type |
| E0277 | 75 | 75 | 0 | Trait bound not satisfied |
| E0425 | 62 | 62 | 0 | Unresolved name |
| E0428 | 36 | 36 | 0 | Duplicate definition |
| E0610 | 34 | 34 | 0 | Primitive has no fields |
| E0606 | 20 | 20 | 0 | Invalid cast |
| E0592 | 16 | 16 | 0 | Duplicate impl method |
| E0117 | 16 | 16 | 0 | Orphan rule violation |
| E0061 | 14 | 14 | 0 | Wrong arg count |
| E0614 | 12 | 12 | 0 | Deref non-pointer |
| E0530 | 12 | 12 | 0 | Match binding shadows |
| E0433 | 12 | 12 | 0 | Unresolved path |
| E0119 | 12 | 12 | 0 | Conflicting trait impl |
| E0071 | 12 | 12 | 0 | Expected struct, found enum |
| E0116 | 8 | 8 | 0 | Impl outside crate |
| Other | 19 | 19 | 0 | (E0596, E0560, E0424, E0368, E0255, E0423, E0605, E0515) |

## Remaining E0599 Breakdown (111 total)

| Pattern | Count | Category |
|---------|-------|----------|
| Methods on `u128` type (what, value, category, swap, test_and_set, notify_*, assume_init_read) | 46 | u128 opaque type |
| `_M_compare` on collate types | 8 | Missing locale method |
| `op_call` on `std_function_void___` | 3 | Callable stub gap |
| `unlock` on `__mutex_type` | 3 | Missing mutex method |
| `swap` on `thread` | 2 | Missing thread method |
| `unlock` on `()` | 1 | Unit type method |
| `store` on `std_atomic_bool` | 1 | Missing atomic method |
| `lock` on `unique_lock_mutex` | 1 | Missing lock method |

## Next Steps

1. **E0308 (701)**: Dominant remaining class. Diverse sub-patterns including type coercion mismatches, iterator types, and template parameter resolution failures. Root-cause fixes needed in the transpiler's type mapping.
2. **E0609 (116)**: Field access on opaque/incomplete struct types. Requires specialization field data for more STL types.
3. **E0277 (75)**: Mixed-signedness arithmetic (`u32 |= i32`, `u32 * i32`), large array `Default` bounds, and function pointer arithmetic.
4. **E0425 (62)**: Legitimate missing functions (`__to_xstring_*`, `current_exception`, `char_traits::move_ptr_mut_*`, `__a`, `__c` allocator params).
5. **E0599 (111)**: Remaining methods on `u128` opaque types dominate. These require resolving the underlying concrete types that are being collapsed to `u128` fallback.
