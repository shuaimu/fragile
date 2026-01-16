# Plan: Basic Class Templates

## Task
Implement basic class template parsing and representation.

## Background
C++ class templates allow parameterized class definitions:

```cpp
template<typename T>
class Box {
public:
    T value;
    Box(T v) : value(v) {}
    T get() const { return value; }
};

// Usage
Box<int> intBox(42);
```

## Scope
This implements parsing and representation only, not instantiation.
Full template instantiation would be a follow-up feature.

## Implementation

### 1. Add CppClassTemplate struct (lib.rs)
```rust
/// A C++ class template declaration.
#[derive(Debug)]
pub struct CppClassTemplate {
    /// Template name (e.g., "Box")
    pub name: String,
    /// Namespace path
    pub namespace: Vec<String>,
    /// Template type parameters (e.g., ["T", "U"])
    pub template_params: Vec<String>,
    /// Whether this is a class (vs struct)
    pub is_class: bool,
    /// Fields (may reference template params)
    pub fields: Vec<CppField>,
    /// Constructors (may reference template params)
    pub constructors: Vec<CppConstructor>,
    /// Destructor (at most one)
    pub destructor: Option<CppDestructor>,
    /// Methods (may reference template params)
    pub methods: Vec<CppMethod>,
    /// Indices of parameter packs (variadic)
    pub parameter_pack_indices: Vec<usize>,
}
```

### 2. Add ClassTemplateDecl to ClangNodeKind (ast.rs)
```rust
/// Class template declaration
ClassTemplateDecl {
    name: String,
    /// Template type parameters
    template_params: Vec<String>,
    is_class: bool,
    /// Indices of parameter packs
    parameter_pack_indices: Vec<usize>,
},
```

### 3. Add class_templates field to CppModule (lib.rs)
```rust
pub struct CppModule {
    // ... existing fields ...
    pub class_templates: Vec<CppClassTemplate>,
}
```

### 4. Handle CXCursor_ClassTemplate in parser (parse.rs)
- CXCursor_ClassTemplate = 31
- Extract template parameters using get_template_type_params_with_packs()
- Parse class body (fields, methods, etc.)

### 5. Handle ClassTemplateDecl in converter (convert.rs)
- Convert AST node to CppClassTemplate
- Store in module.class_templates

## Files to Modify
- `crates/fragile-clang/src/lib.rs` - Add CppClassTemplate, update CppModule
- `crates/fragile-clang/src/ast.rs` - Add ClassTemplateDecl variant
- `crates/fragile-clang/src/parse.rs` - Handle CXCursor_ClassTemplate
- `crates/fragile-clang/src/convert.rs` - Convert ClassTemplateDecl
- `crates/fragile-clang/tests/integration_test.rs` - Add tests

## Test Cases
1. `test_class_template_basic` - Simple class template is parsed
2. `test_class_template_multiple_params` - Multiple template params work
3. `test_class_template_with_methods` - Methods referencing template params
4. `test_class_template_variadic` - Variadic class template detected

## Estimated Size
~200 lines of code
