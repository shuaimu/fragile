# Plan: References & Move Semantics (A.5)

**Status**: Partially Complete [26:01:16]

## Task
Implement parsing of const and rvalue references for C++ move semantics.

## Analysis

### Current State
- Lvalue references (T&) are already supported
- Need to distinguish const references (const T&) and rvalue references (T&&)

### libclang Support
- `CXType_LValueReference` - lvalue reference (T&)
- `CXType_RValueReference` - rvalue reference (T&&)
- `clang_isConstQualifiedType()` - check if type is const

## Implementation Summary

### Changes Made

1. **types.rs**:
   - Added `is_rvalue: bool` field to `CppType::Reference` variant
   - Added `rvalue_ref()` helper method
   - Updated `to_rust_type_str()` to handle rvalue references (both lvalue and rvalue map to raw pointers for FFI)

2. **parse.rs**:
   - Updated `convert_type()` to set `is_rvalue` based on whether type is `CXType_RValueReference`

### Tests Added
- `test_const_reference` - tests const and mutable lvalue references
- `test_rvalue_reference` - tests rvalue references (T&&)

All 53 tests pass (7 unit + 46 integration for fragile-clang, 6 for fragile-rustc-driver).

## Remaining Work
- [ ] std::move - requires understanding of template instantiation
- [ ] std::forward - requires perfect forwarding support
