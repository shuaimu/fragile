# Plan: Override/Final Specifiers (A.3)

**Status**: ✅ Completed [26:01:16]

## Task
Implement parsing of `override` and `final` specifiers on C++ virtual methods.

## Analysis

### Current State
- Virtual and pure virtual methods are parsed with `is_virtual` and `is_pure_virtual` flags
- Override and final specifiers are not detected

### libclang Support
- `CXCursor_CXXOverrideAttr` (404) - child cursor when method has `override`
- `CXCursor_CXXFinalAttr` (405) - child cursor when method has `final`

### Specifier Types
1. **override**: `void foo() override { }` - indicates method overrides a virtual base method
2. **final**: `void bar() final { }` - prevents further overriding in derived classes
3. **both**: `void baz() override final { }` - overrides and prevents further overriding

## Required Changes

### 1. AST Layer (ast.rs)
- Add `is_override: bool` to `CXXMethodDecl`
- Add `is_final: bool` to `CXXMethodDecl`

### 2. Parser Layer (parse.rs)
- Add `get_override_final_attrs()` helper that visits method children
- Detect `CXCursor_CXXOverrideAttr` (405) and `CXCursor_CXXFinalAttr` (404)

### 3. Data Structures (lib.rs)
- Add `is_override: bool` to `CppMethod`
- Add `is_final: bool` to `CppMethod`

### 4. Converter Layer (convert.rs)
- Pass is_override and is_final through conversion

## Implementation Summary

### Changes Made

1. **ast.rs**:
   - Added `is_override: bool` and `is_final: bool` to `CXXMethodDecl`

2. **parse.rs**:
   - Added `get_override_final_attrs()` helper using cursor child visitor
   - Detects CXCursor_CXXOverrideAttr (405) and CXCursor_CXXFinalAttr (404)

3. **lib.rs**:
   - Added `is_override` and `is_final` fields to `CppMethod`

4. **convert.rs**:
   - Updated `convert_struct()` to pass override/final flags through

### Tests Added
- `test_override_specifier` - tests `void foo() override { }`
- `test_final_specifier` - tests `void foo() final { }`
- `test_override_and_final` - tests `void foo() override final { }`

All 46 tests pass (7 unit + 39 integration for fragile-clang, 6 for fragile-rustc-driver).
