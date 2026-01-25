# Investigation: STL Transpilation Remaining Errors

**Date**: 2026-01-24
**Status**: Blocked on architectural issues

## Summary

After reducing vector errors from 2091 to 9 (99.6% reduction), the remaining errors are primarily architectural issues (trait generation for intermediate polymorphic classes) and complex while loop patterns.

### Error Counts by Header
| Header | Transpilation | Compilation Errors |
|--------|---------------|-------------------|
| `<vector>` | ✅ Success | 9 errors |
| `<iostream>` | ✅ Success | ~60 errors |
| `<thread>` | ✅ Success | ~35 errors |

### Recent Fixes ✅ 2026-01-24
- Added type stubs: `value_type`, `std___libcpp_refstring`, `__impl___type_name_t`
- Added union stub for glibc mbstate_t type
- Added function stubs: `__hash`, `__string_to_type_name`
- Added `_LIBCPP_ABI_NAMESPACE` module with `__libcpp_is_constant_evaluated`, `swap`, `move`
- Fixed template array size resolution: `_Size`, `_PaddingSize` now substituted correctly (3 errors fixed)
- Fixed `_unnamed` placeholder handling: use zeroed() for Named types, skip in statements (2 errors fixed)

All headers share the same root causes. The iostream header has more errors because it includes more STL internals (format, hash, containers for buffering, etc.).

## Error Categories

### 1. Missing Traits (7 errors)
- `bad_allocTrait`, `logic_errorTrait`, `runtime_errorTrait` not defined
- **Root cause**: Traits are only generated for root polymorphic classes (those without bases)
- Classes like `bad_alloc` inherit from `exception` but are themselves base classes
- Their traits aren't generated because they have bases, but derived classes try to implement them

**Attempted fix**: Modified trait impl generation to trace up hierarchy
**Result**: Made things worse (exposed 100+ more errors due to `c_void` base class issue)

### 2. Base Class Type Resolution ✅ FIXED
- `exception` → `std::ffi::c_void` instead of `exception` type
- All exception hierarchy classes had `__base: std::ffi::c_void`

**Root cause**: Was in `types.rs` - exception types were explicitly mapped to c_void
**Fix**: Changed mappings in `to_rust_type_str()` to preserve exception type names:
  - `exception` | `std::exception` → `"exception"` (was `"std::ffi::c_void"`)
  - `bad_alloc` | `std::bad_alloc` → `"bad_alloc"` (was `"std::ffi::c_void"`)
  - etc.

**Status**: Fixed ✅ - `bad_alloc` now correctly has `__base: exception`

### 3. Template Array Sizes ✅ FIXED
- `_Size`, `_PaddingSize` used as array sizes but not resolved
- Example: `pub __elems_: [std::ffi::c_void; _Size]`

**Root cause**: Template parameters not being substituted in array-like type names
**Fix**: Modified `substitute_template_type()` in ast_codegen.rs to handle `_Tp[_Size]` patterns
  - Parse array-like type names with bracket notation
  - Substitute both element type and size from substitution map
  - Use `0` as fallback for unknown size parameters

**Status**: Fixed ✅ - Arrays now correctly use numeric sizes (e.g., `[std::ffi::c_void; 0]`)

### 4. Missing Types (vector: 5, iostream: ~20)
- `value_type`, `__impl___type_name_t`, `std___libcpp_refstring`, etc.
- iostream adds: `_HashIterator`, `container_type`, `value_compare`, `_Indexing`, `__iterator`, `mutex_type`, `__max_output_size`, `__format___arg_t`, `__next_pointer`, `_OutIt`, `_Fp`, `_Hash`, `_Cp`, `_Key`
- Some are libc++ internal types, others are template type aliases

### 5. Missing Functions (4 errors)
- `__hash`, `__string_to_type_name`, `swap`, `__libcpp_is_constant_evaluated`
- These are libc++ internal functions not being generated

### 6. While Loop Syntax (1 error - shared)
- Complex post-increment expression in while condition generates invalid Rust
- `while { { let __v = __ptr; __ptr += 1; __v } += 1; ... }`
- This pattern appears in `__non_unique_impl::__hash()` in libc++ typeinfo
- **Original C++**: `while (unsigned char __c = static_cast<unsigned char>(*__ptr++)) { ... }`
- The AST structure is complex: DeclStmt with VarDecl containing ImplicitCastExpr→UnaryOp→UnaryOp
- Added basic DeclStmt handling in generate_while_stmt, but the libc++ case has nested casts
- **Status**: Partially fixed for simple cases; complex cases still produce invalid code

### 7. ASAN Annotation Types (iostream: 3)
- `__asan_annotation_type`, `__asan_annotation_place`
- Debug/sanitizer types from libc++ not being generated

### 8. Other (5 errors)
- Module resolution: `_LIBCPP_ABI_NAMESPACE::swap`, `_LIBCPP_ABI_NAMESPACE::r#move`
- Identifier issues: `__c`, union with long path name

## Recommended Next Steps

1. ~~**Fix base class type resolution** (Priority: High)~~ ✅ DONE
   - Fixed in types.rs by changing exception type mappings

2. **Generate traits for intermediate polymorphic classes** (Priority: High)
   - Modify `generate_struct` to generate traits for all polymorphic classes
   - Not just root classes (those without bases)
   - This will fix 6 errors (bad_allocTrait, logic_errorTrait x4, runtime_errorTrait)
   - **Attempted**: Removed `is_base_class` check in trait generation
   - **Result**: Increased errors from 23 to 83 due to:
     - Duplicate method definitions in trait impls
     - Complex inheritance hierarchies generating conflicting methods
   - **Needs**: More sophisticated approach that tracks which methods are already implemented

3. **Add libc++ type stubs** (Priority: Medium)
   - Add stubs in preamble for common missing types
   - `std___libcpp_refstring`, `__impl___type_name_t`, `value_type`, etc.

4. **Fix complex while loop conditions** (Priority: Low)
   - The `while (unsigned char c = *ptr++)` pattern produces invalid code
   - Affects only 1 function (`__non_unique_impl::__hash`)

## Files to Modify

- `crates/fragile-clang/src/ast_codegen.rs` - Trait generation logic
- `crates/fragile-clang/src/types.rs` - Type stubs for missing libc++ types
