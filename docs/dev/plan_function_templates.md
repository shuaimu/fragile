# Plan: Function Templates (B.1)

**Status**: Partially Complete [26:01:16]

## Task
Implement parsing of basic C++ function templates.

## Analysis

### Current State
- Functions are parsed but templates are not recognized
- Need to handle `FunctionTemplateDecl` cursor type

### libclang Support
- `CXCursor_FunctionTemplate` (30) - function template declaration
- `CXCursor_TemplateTypeParameter` (27) - template type parameter (typename T)
- Children of FunctionTemplateDecl contain template params and the templated function

## Implementation Summary

### Changes Made

1. **ast.rs**:
   - Added `FunctionTemplateDecl` variant with template_params, return_type, params
   - Added `TemplateTypeParmDecl` variant for template type parameters

2. **lib.rs**:
   - Added `CppFunctionTemplate` struct with name, namespace, template_params, return_type, params, is_definition
   - Added `function_templates` field to `CppModule`

3. **parse.rs**:
   - Added handler for `CXCursor_FunctionTemplate` (30)
   - Added handler for `CXCursor_TemplateTypeParameter` (27)
   - Added `get_template_type_params()` helper to extract template parameter names

4. **convert.rs**:
   - Added conversion for `FunctionTemplateDecl` to `CppFunctionTemplate`

### Tests Added
- `test_function_template_basic` - single template parameter `template<typename T>`
- `test_function_template_multiple_params` - multiple params `template<typename T, typename U>`
- `test_function_template_declaration` - declaration without definition

All 56 tests pass (7 unit + 49 integration for fragile-clang, 6 for fragile-rustc-driver).

## Remaining Work
- [ ] Argument deduction
- [ ] Template specialization (`template<> void foo<int>(...)`)
- [ ] Variadic templates (`template<typename... Args>`)
