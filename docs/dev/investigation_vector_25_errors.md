# Investigation: STL Transpilation Remaining Errors

**Date**: 2026-01-24
**Status**: Significant progress, 16 compilation errors remaining

## Summary

After reducing vector errors from 2091 to 8 (99.6% reduction), then exposing and fixing underlying errors, we're now at 16 compilation errors.

### Error Counts by Header
| Header | Transpilation | Errors (start) | Current |
|--------|---------------|----------------|---------|
| `<vector>` | ✅ Success | 8 → 69 (exposed) | 16 errors |
| `<iostream>` | ✅ Success | ~60 errors | TBD |
| `<thread>` | ✅ Success | ~35 errors | TBD |

### Recent Fixes ✅ 2026-01-24/25
- Added type stubs: `value_type`, `std___libcpp_refstring`, `__impl___type_name_t`
- Added union stub for glibc mbstate_t type
- Added function stubs: `__hash`, `__string_to_type_name`
- Added `_LIBCPP_ABI_NAMESPACE` module with `__libcpp_is_constant_evaluated`, `swap`, `move`
- Fixed template array size resolution: `_Size`, `_PaddingSize` now substituted correctly (3 errors fixed)
- Fixed `_unnamed` placeholder handling: use zeroed() for Named types, skip in statements (2 errors fixed)
- Fixed while loop with VarDecl condition: generate proper loop with break check (1 error fixed)
- **Fixed trait generation for polymorphic class hierarchies** ✅ 2026-01-25:
  - Added `find_root_polymorphic_ancestor()` to trace up inheritance hierarchy
  - Derived classes now implement ROOT class's trait, not immediate parent's trait
  - Fixed 8 missing trait errors (bad_allocTrait, logic_errorTrait x4, runtime_errorTrait x3)
- **Fixed duplicate anonymous bit field accessors** ✅ 2026-01-25:
  - Added counter to generate unique names `_unnamed_1`, `_unnamed_2`, etc.
  - Fixed 20 duplicate definition errors
- **Added exception class stub constructors** ✅ 2026-01-25:
  - `logic_error::new_1`, `runtime_error::new_1` for string/const char* arguments
  - Fixed 7 missing constructor errors
- **Fixed placeholder `_` return types** ✅ 2026-01-25:
  - Extended `sanitize_return_type()` to all code generation paths
  - Template methods, coroutines, operators, trait implementations
  - Fixed 9 placeholder errors
- **Fixed duplicate value_type type alias** ✅ 2026-01-25:
  - Register stub types in `generated_aliases` to prevent duplicates
  - Fixed 1 error

## Trait Fix Details ✅ 2026-01-25

### Problem
- `bad_allocTrait`, `logic_errorTrait`, `runtime_errorTrait` not defined
- Root cause: Traits only generated for ROOT polymorphic classes (no polymorphic base)
- Classes like `bad_alloc` inherit from `exception`, so `bad_allocTrait` wasn't generated
- But `bad_array_new_length` tried to `impl bad_allocTrait` → failed

### Solution
Instead of generating traits for intermediate classes (which causes method duplication),
we make ALL derived classes implement the ROOT class's trait:

```
Exception hierarchy:
exception          ← ROOT, generates exceptionTrait
├── bad_alloc      ← impl exceptionTrait (not bad_allocTrait)
│   └── bad_array_new_length ← impl exceptionTrait (through __base)
├── logic_error    ← impl exceptionTrait
│   ├── domain_error ← impl exceptionTrait
│   └── out_of_range ← impl exceptionTrait
└── runtime_error  ← impl exceptionTrait
    └── range_error ← impl exceptionTrait
```

### Implementation
1. Added `find_root_polymorphic_ancestor()` function that traces up class hierarchy
2. Modified trait impl generation: instead of `impl {base}Trait for {derived}`,
   now uses `impl {root}Trait for {derived}`

### Result
- 8 trait errors → 0 trait errors ✅
- Exposed 69 pre-existing errors that were blocked by trait errors

## Remaining Error Categories (16 errors)

### 1. Mismatched Types (11 errors)
- Template type parameters resolved to `c_void` placeholder incorrectly
- `numeric_limits::min()`, `numeric_limits::max()` returning wrong types
- Iterator `operator[]` returning value instead of reference
- `select_on_container_copy_construction` clone return type issue

### 2. Argument Count Errors (2 errors)
- `_Hash_impl::hash()` called with 1 argument instead of 3
- float/double hash specializations need different call patterns

### 3. Non-primitive Cast Errors (2 errors)
- `0 as _Sp___rep` - integer cast to struct type
- numeric_limits max() implementation issues

### 4. Missing Method (1 error)
- `is_equal` method not found for `&c_void`
- Type info comparison needs method stub

## Analysis

The remaining 16 errors are fundamentally about template type resolution:
- Template type parameters being substituted with `c_void` placeholder
- Specialized template functions not generating correct argument patterns
- Type coercion between templates and concrete types

These require deeper changes to the template instantiation system.

## Next Steps

1. **Template Type Resolution** (Priority: High)
   - Improve type parameter substitution in template specializations
   - Track original template types through specialization process

2. **Hash Function Specializations** (Priority: Medium)
   - Add special handling for float/double hash implementations
   - These need different calling patterns than pointer-based hashing

3. **Type Info Method Stubs** (Priority: Low)
   - Add `is_equal` method stub to type info classes

## Files Modified

- `crates/fragile-clang/src/ast_codegen.rs`:
  - Added `find_root_polymorphic_ancestor()` function
  - Modified trait impl generation to use root class
