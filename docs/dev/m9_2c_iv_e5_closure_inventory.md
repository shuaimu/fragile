# M9.2.c.iv.e.5 Closure Live Strict Compile Inventory

**Date**: 2026-03-21
**Compile profile**: `FRAGILEC_MODE=strict ./target/release/fragilec -c -std=gnu++23 -I vendor/mako/third-party/rusty-cpp/include -I vendor/mako/src/rrr -I vendor/mako/src`
**Parser backend**: `fragile-parser-clang` (default since M8.1)

## Per-File Error Counts

| File | Total Errors | Previous (e.5.f) | Delta |
|------|-------------|-------------------|-------|
| debugging.cpp | 348 | 410 | -62 (-15%) |
| misc.cpp | 344 | ~410 | -66 (-16%) |
| basetypes.cpp | 325 | (new) | N/A |
| logging.cpp | 389 | (preflight fail) | N/A |

## Error Category Breakdown

### debugging.cpp (348 total)

| Error Code | Count | Description |
|-----------|-------|-------------|
| E0308 | 179 | Type mismatches |
| E0599 | 57 | No method found |
| E0609 | 28 | No field on type |
| E0277 | 17 | Trait bound not satisfied |
| E0425 | 15 | Unresolved name |
| E0428 | 9 | Duplicate definition |
| E0610 | 7 | Applied unary `*` to non-pointer |
| E0606 | 4 | Invalid cast |
| E0592 | 4 | Duplicate impl method |
| E0117 | 4 | Orphan rule violation |
| E0530 | 3 | Match binding shadows |
| E0433 | 3 | Unresolved path |
| E0119 | 3 | Conflicting trait impl |
| E0071 | 3 | Expected struct, found enum |
| E0061 | 3 | Wrong number of args |
| E0116 | 2 | Impl for type outside crate |
| E0614 | 1 | Deref non-pointer |
| E0596 | 1 | Borrow mutability |
| E0560 | 1 | Unknown field in init |
| E0424 | 1 | `self` usage |
| E0423 | 1 | Expected value, found struct |
| E0368 | 1 | Binary op not implemented |
| E0255 | 1 | Import conflict |

### misc.cpp (344 total)

| Error Code | Count | Description |
|-----------|-------|-------------|
| E0308 | 175 | Type mismatches |
| E0599 | 57 | No method found |
| E0609 | 28 | No field on type |
| E0277 | 17 | Trait bound not satisfied |
| E0425 | 15 | Unresolved name |
| E0428 | 9 | Duplicate definition |
| E0610 | 7 | Applied unary `*` to non-pointer |
| E0606 | 4 | Invalid cast |
| E0592 | 4 | Duplicate impl method |
| E0117 | 4 | Orphan rule violation |
| E0530 | 3 | Match binding shadows |
| E0433 | 3 | Unresolved path |
| E0119 | 3 | Conflicting trait impl |
| E0071 | 3 | Expected struct, found enum |
| E0061 | 3 | Wrong number of args |
| E0116 | 2 | Impl for type outside crate |
| E0614 | 1 | Deref non-pointer |
| E0596 | 1 | Borrow mutability |
| E0560 | 1 | Unknown field in init |
| E0424 | 1 | `self` usage |
| E0423 | 1 | Expected value, found struct |
| E0368 | 1 | Binary op not implemented |
| E0255 | 1 | Import conflict |

### basetypes.cpp (325 total)

| Error Code | Count | Description |
|-----------|-------|-------------|
| E0308 | 160 | Type mismatches |
| E0599 | 48 | No method found |
| E0609 | 25 | No field on type |
| E0277 | 20 | Trait bound not satisfied |
| E0425 | 16 | Unresolved name |
| E0610 | 11 | Applied unary `*` to non-pointer |
| E0428 | 9 | Duplicate definition |
| E0606 | 4 | Invalid cast |
| E0592 | 4 | Duplicate impl method |
| E0117 | 4 | Orphan rule violation |
| E0530 | 3 | Match binding shadows |
| E0433 | 3 | Unresolved path |
| E0119 | 3 | Conflicting trait impl |
| E0071 | 3 | Expected struct, found enum |
| E0061 | 3 | Wrong number of args |
| E0116 | 2 | Impl for type outside crate |
| E0614 | 1 | Deref non-pointer |
| E0596 | 1 | Borrow mutability |
| E0560 | 1 | Unknown field in init |
| E0515 | 1 | Cannot return ref to local |
| E0424 | 1 | `self` usage |
| E0368 | 1 | Binary op not implemented |
| E0255 | 1 | Import conflict |

### logging.cpp (389 total)

| Error Code | Count | Description |
|-----------|-------|-------------|
| E0308 | 187 | Type mismatches |
| E0599 | 61 | No method found |
| E0609 | 35 | No field on type |
| E0277 | 21 | Trait bound not satisfied |
| E0425 | 16 | Unresolved name |
| E0614 | 9 | Deref non-pointer |
| E0610 | 9 | Applied unary `*` to non-pointer |
| E0428 | 9 | Duplicate definition |
| E0606 | 8 | Invalid cast |
| E0061 | 5 | Wrong number of args |
| E0592 | 4 | Duplicate impl method |
| E0117 | 4 | Orphan rule violation |
| E0530 | 3 | Match binding shadows |
| E0433 | 3 | Unresolved path |
| E0119 | 3 | Conflicting trait impl |
| E0071 | 3 | Expected struct, found enum |
| E0116 | 2 | Impl for type outside crate |
| E0605 | 1 | Non-primitive cast |
| E0596 | 1 | Borrow mutability |
| E0560 | 1 | Unknown field in init |
| E0424 | 1 | `self` usage |
| E0423 | 1 | Expected value, found struct |
| E0368 | 1 | Binary op not implemented |
| E0255 | 1 | Import conflict |

## Comparison with Previous Inventory

### vs e.5.h (incorrect c++17 flags, 2 files only)
- e.5.h reported debugging.cpp total=100 (with `-std=c++17`, partial includes)
- Current (correct gnu++23 flags): 348 — higher because more code is parsed

### vs e.5.f corrected inventory (correct gnu++23 flags, 2 files only)
- e.5.f debugging.cpp: 410 -> 348 (-62, -15.1%)
- e.5.f misc.cpp: ~410 -> 344 (-66, -16.1%)

### What was eliminated by e.3 + e.5 sub-tasks
1. **E0425 `__fsv___func___x_0` scope leak** (186/file): fully eliminated by e.5.h brace-literal fix + e.5.f.5 pub(crate) fn fix
2. **E0308 `runtime_error::new_1` borrow** (8/file): eliminated by e.3.b
3. **E0308 `__lce_alg_type` enum mismatch** (4/file): eliminated by e.3.c
4. **E0308 `Self::lt`/`Self::eq` i8-lane** (12/file): eliminated by e.3.f.1
5. **E0368 iterator `AddAssign`** (16/file): eliminated by e.4
6. **E0614 double-deref ptr arithmetic** (10/file): eliminated by e.5.b
7. **E0605 non-primitive cast** (7/file): eliminated by e.5.c (1 residual in logging.cpp)
8. **E0603 private field access** (4/file): fully eliminated by e.5.d
9. **E0277 `TryInto<i64>` trait bound** (62/file): eliminated by e.5.g `CharTraitsArg` trait
10. **E0599 callable STL `op_call`** (3-4/file): eliminated by e.5.a

### What remains (dominant classes)
1. **E0308 type mismatches** (160-187/file, ~50%): dominant remaining class, diverse sub-patterns
2. **E0599 no method found** (48-61/file, ~16%): missing method stubs on STL types
3. **E0609 no field on type** (25-35/file, ~8%): struct field access on opaque types
4. **E0277 trait bound failures** (17-21/file, ~5%): mixed signedness arithmetic, large array Default
5. **E0425 unresolved names** (15-16/file, ~4%): legitimate missing functions

## Summary

All M9.2.c.iv.e.3 sub-tasks (a through f.2) and M9.2.c.iv.e.5 sub-tasks (a through h) are complete.
Total strict compile errors across 4 blocker files: 348 + 344 + 325 + 389 = **1406**.
The dominant remaining class is E0308 type mismatches (701 total, 50%), followed by E0599 missing methods (223, 16%) and E0609 field access (116, 8%).
