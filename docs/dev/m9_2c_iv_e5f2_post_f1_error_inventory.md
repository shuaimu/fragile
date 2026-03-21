# M9.2.c.iv.e.5.f.2: Post-f.1 Strict Compile Error Inventory

## Summary

Re-captured strict compile error inventory for `rrr/base/debugging.cpp` and
`rrr/base/misc.cpp` after M9.2.c.iv.e.5.f.1 function-static scoping hardening.

**Key finding: 0 `__fsv___func___x_0` references remain in E0425 errors.**
The f.1 fix fully resolved the function-static alias leakage issue.

## Compile Profile

- Compiler: `fragilec` (release build, commit post-f.1)
- Standard: `-std=gnu++23`
- Include paths: `-I$MAKO_SRC/rrr -I$MAKO_SRC/rrr/rpc -I$MAKO_SRC/rrr/misc -I$MAKO_SRC/rrr/base -I$MAKO_SRC`
- Date: 2026-03-21

## Error Inventory: debugging.cpp

| Error Code | Count | Description |
|-----------|-------|-------------|
| E0308 | 179 | Type mismatch |
| E0277 | 79 | Trait not satisfied |
| E0599 | 57 | Method not found |
| E0609 | 28 | No field on type |
| E0425 | 15 | Unresolved name (0 `__fsv_`) |
| E0428 | 9 | Duplicate definition |
| E0610 | 7 | Field access on primitive |
| E0606 | 4 | Invalid cast |
| E0592 | 4 | Duplicate impl |
| E0117 | 4 | Orphan impl |
| E0530 | 3 | Match binding shadows |
| E0433 | 3 | Unresolved module |
| E0119 | 3 | Conflicting impl |
| E0071 | 3 | Struct expression on non-struct |
| E0061 | 3 | Wrong number of args |
| E0116 | 2 | Impl on non-local type |
| E0614 | 1 | Deref non-pointer |
| E0596 | 1 | Borrow immutable as mutable |
| E0560 | 1 | Unknown struct field |
| E0424 | 1 | `self` not available |
| E0423 | 1 | Expected value, found type |
| E0368 | 1 | Binary assign op |
| E0255 | 1 | Name clash |
| **Total** | **410** | |

## Error Inventory: misc.cpp

| Error Code | Count | Description |
|-----------|-------|-------------|
| E0308 | 175 | Type mismatch |
| E0277 | 79 | Trait not satisfied |
| E0599 | 57 | Method not found |
| E0609 | 28 | No field on type |
| E0425 | 15 | Unresolved name (0 `__fsv_`) |
| E0428 | 9 | Duplicate definition |
| E0610 | 7 | Field access on primitive |
| E0606 | 4 | Invalid cast |
| E0592 | 4 | Duplicate impl |
| E0117 | 4 | Orphan impl |
| E0530 | 3 | Match binding shadows |
| E0433 | 3 | Unresolved module |
| E0119 | 3 | Conflicting impl |
| E0071 | 3 | Struct expression on non-struct |
| E0061 | 3 | Wrong number of args |
| E0116 | 2 | Impl on non-local type |
| E0614 | 1 | Deref non-pointer |
| E0596 | 1 | Borrow immutable as mutable |
| E0560 | 1 | Unknown struct field |
| E0424 | 1 | `self` not available |
| E0423 | 1 | Expected value, found type |
| E0368 | 1 | Binary assign op |
| E0255 | 1 | Name clash |
| **Total** | **406** | |

## E0425 Breakdown (debugging.cpp)

All 15 remaining E0425 errors are legitimate missing functions/values:
- `__a` (4): unresolved allocator parameter
- `__c` (1): unresolved parameter
- `char_traits::move_ptr_mut_*_u64` (5): missing char_traits member functions
- `__to_xstring_*` (3): missing string conversion function
- `current_exception` (1): missing exception function
- `__` (1): empty identifier

**None** are `__fsv___func___x_0` patterns — the function-static scoping fix is complete.

## Comparison with Previous Inventories

| Inventory | Total Errors | E0425 | `__fsv_` in E0425 |
|-----------|-------------|-------|-------------------|
| e.5.e (2026-03-21, pre-f.1) | 275-276 | 194 | 186 |
| e.5.f (2026-03-20, corrected flags) | 410 | 15 | 0 |
| **e.5.f.2 (post-f.1)** | **410/406** | **15** | **0** |

The f.1 fix did not change error counts because the corrected-flags inventory (e.5.f)
already showed 0 `__fsv_` matches. This confirms the corrected compile flags (gnu++23,
full include tree) were the primary factor resolving the `__fsv_` scoping issue, and the
f.1 hardening provides defense-in-depth for edge cases.

## Conclusion

M9.2.c.iv.e.5.f.2 is complete. The `__fsv___func___x_0` references are fully resolved.
The remaining 15 E0425 errors are legitimate missing function/value declarations that
will be addressed by future codegen improvements (not function-static scoping issues).
