# Plan: Single Inheritance (A.3)

**Status**: ✅ Completed [26:01:16]

## Task
Implement parsing of single inheritance in C++ classes.

## Analysis

### Current State
- Class definitions are parsed with fields, methods, constructors, destructors, friends
- No support for inheritance (base classes)

### libclang Support
- `CXCursor_CXXBaseSpecifier` (44) - cursor kind for base class specifiers
- `clang_isVirtualBase()` - check if base is virtual inheritance
- `clang_getCXXAccessSpecifier()` - get access specifier (public/private/protected)
- `clang_getCursorType()` - get the type of the base class

### Inheritance Types
1. **Public inheritance**: `class Derived : public Base`
2. **Protected inheritance**: `class Derived : protected Base`
3. **Private inheritance**: `class Derived : private Base`
4. **Virtual inheritance**: `class Derived : virtual public Base`

## Required Changes

### 1. AST Layer (ast.rs)
- Add `CXXBaseSpecifier` variant with:
  - `base_type: CppType` - the base class type
  - `access: AccessSpecifier` - inheritance access
  - `is_virtual: bool` - virtual inheritance flag

### 2. Parser Layer (parse.rs)
- Handle `CXCursor_CXXBaseSpecifier`
- Extract base class type, access, and virtual flag

### 3. Data Structures (lib.rs)
- Add `CppBaseClass` struct with base class info
- Add `bases: Vec<CppBaseClass>` to `CppStruct`

### 4. Converter Layer (convert.rs)
- Pass bases through conversion

## Estimated Changes
- ast.rs: ~10 lines
- parse.rs: ~20 lines
- lib.rs: ~15 lines
- convert.rs: ~10 lines
- tests: ~60 lines
Total: ~115 lines (well under 500 LOC)

## Test Cases
1. Public single inheritance
2. Protected single inheritance
3. Private single inheritance
4. Virtual inheritance
5. Derived class accessing base members

## Implementation Summary

### Changes Made

1. **ast.rs**:
   - Added `CXXBaseSpecifier` variant with `base_type: CppType`, `access: AccessSpecifier`, `is_virtual: bool`

2. **parse.rs**:
   - Added `CXCursor_CXXBaseSpecifier` handler
   - Uses `clang_getCursorType()` for base type
   - Uses `clang_isVirtualBase()` for virtual inheritance detection

3. **lib.rs**:
   - Added `CppBaseClass` struct with `base_type`, `access`, `is_virtual`
   - Added `bases: Vec<CppBaseClass>` to `CppStruct`

4. **convert.rs**:
   - Updated `convert_struct()` to handle `CXXBaseSpecifier` nodes
   - Passes bases through to the returned `CppStruct`

5. **stubs.rs**:
   - Updated test structs to include `bases: vec![]`

### Tests Added
- `test_public_inheritance` - tests `class Derived : public Base`
- `test_protected_inheritance` - tests `class Derived : protected Base`
- `test_private_inheritance` - tests `class Derived : private Base`
- `test_virtual_inheritance` - tests `class Derived : virtual public Base`

All 39 tests pass (33 fragile-clang + 6 fragile-rustc-driver).
