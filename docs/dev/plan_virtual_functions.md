# Plan: Virtual Functions (A.3)

**Status**: ✅ Completed [26:01:16]

## Task
Implement parsing of virtual and pure virtual functions in C++ classes.

## Analysis

### Current State
- Methods are parsed with `is_static` flag
- No support for virtual functions

### libclang Support
- `clang_CXXMethod_isVirtual()` - check if method is virtual
- `clang_CXXMethod_isPureVirtual()` - check if method is pure virtual (= 0)

### Virtual Function Types
1. **Virtual function**: `virtual void foo() { }`
2. **Pure virtual function**: `virtual void bar() = 0;`

## Required Changes

### 1. AST Layer (ast.rs)
- Add `is_virtual: bool` to `CXXMethodDecl`
- Add `is_pure_virtual: bool` to `CXXMethodDecl`

### 2. Parser Layer (parse.rs)
- Use `clang_CXXMethod_isVirtual()` and `clang_CXXMethod_isPureVirtual()`

### 3. Data Structures (lib.rs)
- Add `is_virtual: bool` to `CppMethod`
- Add `is_pure_virtual: bool` to `CppMethod`

### 4. Converter Layer (convert.rs)
- Pass is_virtual and is_pure_virtual through conversion

## Estimated Changes
- ast.rs: ~5 lines
- parse.rs: ~5 lines
- lib.rs: ~5 lines
- convert.rs: ~5 lines
- tests: ~40 lines
Total: ~60 lines (well under 500 LOC)

## Test Cases
1. Virtual function `virtual void foo() { }`
2. Pure virtual function `virtual void bar() = 0;`
3. Override function `void foo() override { }`

## Implementation Summary

### Changes Made

1. **ast.rs**:
   - Added `is_virtual: bool` and `is_pure_virtual: bool` to `CXXMethodDecl`

2. **parse.rs**:
   - Added `clang_CXXMethod_isVirtual()` and `clang_CXXMethod_isPureVirtual()` calls

3. **lib.rs**:
   - Added `is_virtual` and `is_pure_virtual` fields to `CppMethod`

4. **convert.rs**:
   - Updated `convert_struct()` to pass virtual flags through

### Tests Added
- `test_virtual_function` - tests `virtual void foo() { }` and non-virtual methods
- `test_pure_virtual_function` - tests `virtual void pure() = 0;`

All 42 tests pass (36 fragile-clang + 6 fragile-rustc-driver).
