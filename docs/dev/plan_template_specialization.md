# Plan: Template Specialization

## Task
Implement explicit template specialization for function templates.

## Background
C++ allows explicit specialization of function templates:
```cpp
// Primary template
template<typename T> T identity(T x) { return x; }

// Explicit specialization for int
template<> int identity<int>(int x) { return x + 1; }
```

When calling `identity<int>(42)`, the specialized version should be used.

## Implementation

### 1. Add CppTemplateSpecialization struct
```rust
pub struct CppTemplateSpecialization {
    /// Template arguments for this specialization
    pub args: Vec<CppType>,
    /// The specialized function
    pub function: CppFunction,
}
```

### 2. Add specializations field to CppFunctionTemplate
```rust
pub struct CppFunctionTemplate {
    // ... existing fields ...
    pub specializations: Vec<CppTemplateSpecialization>,
}
```

### 3. Detect specializations in parser/converter
In convert.rs, when processing a FunctionDecl:
- Check if name matches a template name with explicit type args
- Link the function as a specialization of the template

### 4. Add selection logic
In instantiation:
- First check for matching explicit specialization
- Fall back to primary template if none found

## Files to Modify
- `crates/fragile-clang/src/lib.rs` - Add CppTemplateSpecialization struct
- `crates/fragile-clang/src/convert.rs` - Link specializations to templates
- `crates/fragile-clang/tests/integration_test.rs` - Add tests

## Test Cases
1. `test_explicit_specialization_parsed` - Specialization is linked to template
2. `test_specialization_selected` - Correct specialization used during instantiation
3. `test_fallback_to_primary` - Primary template used when no specialization matches

## Estimated Size
~150 lines of code
