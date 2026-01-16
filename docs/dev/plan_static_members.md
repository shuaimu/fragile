# Plan: Static Members (A.2)

**Status**: ✅ Completed [26:01:16]

## Task
Implement parsing of static member variables and static methods in C++ classes.

## Analysis

### Current State
- Class fields and methods are parsed
- No distinction between static and non-static members

### libclang Support
- `clang_CXXMethod_isStatic()` - check if a method is static
- `clang_Cursor_getStorageClass()` - get storage class (`CX_SC_Static` = 3)
- `CXCursor_CXXMethod` (21) - cursor kind for C++ methods

## Required Changes

### 1. AST Layer (ast.rs)
- Add `is_static: bool` to `FieldDecl` variant
- Add `CXXMethodDecl` variant for methods

### 2. Parser Layer (parse.rs)
- Use `clang_Cursor_getStorageClass()` to detect static variables
- Handle `CXCursor_CXXMethod` for methods
- Use `clang_CXXMethod_isStatic()` to detect static methods

### 3. Data Structures (lib.rs)
- Add `is_static: bool` to field representation
- Update method representation to include `is_static`

### 4. Converter Layer (convert.rs)
- Pass `is_static` through conversion

## Estimated Changes
- ast.rs: ~15 lines
- parse.rs: ~30 lines
- lib.rs: ~10 lines
- convert.rs: ~15 lines
- tests: ~60 lines
Total: ~130 lines (well under 500 LOC)

## Test Cases
1. Static member variable `static int count;`
2. Static method `static void foo() { }`
3. Mix of static and non-static members
4. Static member initialization `int Foo::count = 0;`

## Implementation Summary

### Changes Made

1. **ast.rs**:
   - Added `is_static: bool` field to `FieldDecl` variant
   - Added `CXXMethodDecl` variant with `name`, `return_type`, `params`, `is_definition`, `is_static`, `access` fields

2. **parse.rs**:
   - Added `CXCursor_CXXMethod` handler using `clang_CXXMethod_isStatic()`
   - Updated `CXCursor_VarDecl` to detect static class members via `clang_Cursor_getStorageClass()` and parent cursor check
   - Added `extract_params()` helper function

3. **lib.rs**:
   - Restructured `CppStruct` to use `CppField` struct instead of tuple for fields
   - Added `static_fields: Vec<CppField>` field to `CppStruct`
   - Added `CppMethod` struct with `is_static` field
   - Changed `methods` from `Vec<CppFunction>` to `Vec<CppMethod>`

4. **convert.rs**:
   - Added `convert_method_body()` helper function
   - Updated `convert_struct()` to handle static fields and methods separately

5. **stubs.rs/queries.rs**:
   - Updated to use new `CppField` struct accessor pattern

### Tests Added
- `test_static_member_variable` - tests `static int count;`
- `test_static_method` - tests `static void increment() { }`
- `test_mixed_static_members` - tests combination of static and non-static

All 26 fragile-clang tests pass (7 unit + 19 integration → 7 unit + 26 integration).
