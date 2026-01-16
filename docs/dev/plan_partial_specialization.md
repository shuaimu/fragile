# Plan: Class Template Partial Specialization

## Task
Implement detection and representation of class template partial specializations.

## Background
C++ allows partial specialization of class templates:

```cpp
// Primary template
template<typename T, typename U>
class Pair { ... };

// Partial specialization: both types are the same
template<typename T>
class Pair<T, T> { ... };

// Partial specialization: second type is pointer
template<typename T, typename U>
class Pair<T, U*> { ... };
```

## Scope
This implements detection and representation. Full matching and selection
logic for instantiation would be a follow-up.

## Implementation

### 1. Add CppClassTemplatePartialSpec struct (lib.rs)
```rust
/// A partial specialization of a class template.
#[derive(Debug)]
pub struct CppClassTemplatePartialSpec {
    /// Name of the primary template being specialized
    pub template_name: String,
    /// Specialization pattern (e.g., ["T", "T"] for Pair<T,T>)
    pub specialization_pattern: Vec<CppType>,
    /// Template parameters for this specialization
    pub template_params: Vec<String>,
    /// Fields, methods, etc. (same as CppClassTemplate)
    pub is_class: bool,
    pub namespace: Vec<String>,
    pub fields: Vec<CppField>,
    pub methods: Vec<CppMethod>,
    pub constructors: Vec<CppConstructor>,
    pub destructor: Option<CppDestructor>,
    pub parameter_pack_indices: Vec<usize>,
}
```

### 2. Add ClassTemplatePartialSpecDecl to AST (ast.rs)
```rust
ClassTemplatePartialSpecDecl {
    name: String,
    template_params: Vec<String>,
    specialization_pattern: Vec<CppType>,
    is_class: bool,
    parameter_pack_indices: Vec<usize>,
},
```

### 3. Add partial_specializations to CppModule (lib.rs)
```rust
pub class_partial_specializations: Vec<CppClassTemplatePartialSpec>,
```

### 4. Handle CXCursor_ClassTemplatePartialSpecialization (32) in parser

### 5. Add converter support

## Files to Modify
- `crates/fragile-clang/src/lib.rs` - Add CppClassTemplatePartialSpec
- `crates/fragile-clang/src/ast.rs` - Add ClassTemplatePartialSpecDecl
- `crates/fragile-clang/src/parse.rs` - Handle cursor type 32
- `crates/fragile-clang/src/convert.rs` - Convert partial specs
- `crates/fragile-clang/tests/integration_test.rs` - Add tests

## Test Cases
1. `test_partial_spec_same_type` - Pair<T, T> detected
2. `test_partial_spec_pointer` - Pair<T, U*> detected
3. `test_partial_spec_with_methods` - Methods in partial spec

## Estimated Size
~200 lines of code
