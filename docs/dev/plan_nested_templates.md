# Plan: Nested Templates

## Task
Implement support for nested templates in C++ class and function templates.

## Background
C++ allows templates to be nested in various ways:

1. **Member Templates**: Method templates inside classes
```cpp
class Container {
public:
    template<typename U>
    void add(U value);
};
```

2. **Member Templates in Class Templates**:
```cpp
template<typename T>
class Container {
public:
    template<typename U>
    void convert(U value);
};
```

3. **Nested Class Templates**: Class templates inside other templates
```cpp
template<typename T>
class Outer {
    template<typename U>
    class Inner { };
};
```

## Scope
This implementation focuses on **member templates** (method templates inside classes),
which is the most common pattern used in the Mako codebase.

Key examples from Mako:
- `small_unordered_map.h`: bucket::construct() variadic member template
- `marshal.hpp`: MarshallDeputy template constructor

## Implementation

### 1. Add CppMemberTemplate struct (lib.rs)
```rust
/// A member template (template method inside a class).
#[derive(Debug)]
pub struct CppMemberTemplate {
    /// Method name
    pub name: String,
    /// Template type parameters
    pub template_params: Vec<String>,
    /// Return type
    pub return_type: CppType,
    /// Parameters
    pub params: Vec<(String, CppType)>,
    /// Access specifier
    pub access: AccessSpecifier,
    /// Whether this is static
    pub is_static: bool,
    /// Indices of parameter packs
    pub parameter_pack_indices: Vec<usize>,
    /// Whether this is a definition (has body)
    pub is_definition: bool,
}
```

### 2. Add member_templates to CppStruct and CppClassTemplate
```rust
// In CppStruct
pub member_templates: Vec<CppMemberTemplate>,

// In CppClassTemplate
pub member_templates: Vec<CppMemberTemplate>,
```

### 3. Handle member templates in parser (parse.rs)
- Detect CXCursor_FunctionTemplate (30) when parent is a class
- Extract template parameters with `get_template_type_params_with_packs`
- Parse method signature with template-aware type conversion

### 4. Add converter support (convert.rs)
- Convert member templates in `convert_struct` and `convert_class_template`

## Files to Modify
- `crates/fragile-clang/src/lib.rs` - Add CppMemberTemplate, update CppStruct/CppClassTemplate
- `crates/fragile-clang/src/parse.rs` - Handle member templates in class context
- `crates/fragile-clang/src/convert.rs` - Convert member templates
- `crates/fragile-clang/tests/integration_test.rs` - Add tests

## Test Cases
1. `test_member_template_basic` - Simple method template in non-template class
2. `test_member_template_in_class_template` - Method template in class template
3. `test_member_template_variadic` - Variadic member template (like Mako's construct())
4. `test_member_template_with_class_params` - Uses both class and method template params

## Estimated Size
~250-300 lines of code

