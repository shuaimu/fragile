# Investigation: STL Transpilation Remaining Errors

**Date**: 2026-01-24
**Status**: Blocked on architectural issues

## Summary

After reducing vector errors from 2091 to 25 (98.8% reduction), and testing iostream transpilation (65 errors), the remaining errors across both headers share the same root causes - deep architectural issues that require significant refactoring.

### Error Counts by Header
| Header | Transpilation | Compilation Errors |
|--------|---------------|-------------------|
| `<vector>` | ✅ Success | 25 errors |
| `<iostream>` | ✅ Success | 65 errors |
| `<thread>` | ✅ Success | 40 errors |

All headers share the same root causes. The iostream header has more errors because it includes more STL internals (format, hash, containers for buffering, etc.).

## Error Categories

### 1. Missing Traits (7 errors)
- `bad_allocTrait`, `logic_errorTrait`, `runtime_errorTrait` not defined
- **Root cause**: Traits are only generated for root polymorphic classes (those without bases)
- Classes like `bad_alloc` inherit from `exception` but are themselves base classes
- Their traits aren't generated because they have bases, but derived classes try to implement them

**Attempted fix**: Modified trait impl generation to trace up hierarchy
**Result**: Made things worse (exposed 100+ more errors due to `c_void` base class issue)

### 2. Base Class Type Resolution (contributes to many errors)
- `exception` → `std::ffi::c_void` instead of `exception` type
- All exception hierarchy classes have `__base: std::ffi::c_void`
- This breaks trait implementations and method calls

**Root cause**: In libc++, base classes are being resolved to `c_void` during AST parsing
**Location**: Likely in `parse.rs` when resolving `CXXBaseSpecifier` types

### 3. Template Array Sizes (vector: 3, iostream: 1)
- `_Size`, `_PaddingSize` used as array sizes but not resolved
- Example: `pub __elems_: [std::ffi::c_void; _Size]`

**Root cause**: Template parameters not evaluated during struct generation

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

1. **Fix base class type resolution** (Priority: High)
   - Investigate `parse.rs` CXXBaseSpecifier handling
   - Ensure proper type names are preserved for polymorphic base classes
   - This will fix ~30% of remaining errors

2. **Generate traits for intermediate polymorphic classes** (Priority: Medium)
   - Modify `generate_struct` to generate traits for all polymorphic classes
   - Not just root classes (those without bases)

3. **Add libc++ type stubs** (Priority: Low)
   - Add stubs in preamble for common missing types
   - `std___libcpp_refstring`, `__impl___type_name_t`, etc.

## Files to Modify

- `crates/fragile-clang/src/parse.rs` - Base class type resolution
- `crates/fragile-clang/src/ast_codegen.rs` - Trait generation logic
- `crates/fragile-clang/src/types.rs` - Type name mapping
