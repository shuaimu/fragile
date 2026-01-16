# Plan: Namespace Declarations (A.1.1)

**Status:** Completed [26:01:15, 23:35]

## Design Rationale

### Problem
C++ namespaces are essential for organizing code and avoiding name conflicts. The Mako
database uses `namespace rrr` and `namespace mako` extensively. Without namespace support,
all declarations would be flattened to global scope, causing name conflicts and losing
semantic information needed for correct code generation.

### Solution
We implement namespace support by:
1. Adding a `NamespaceDecl` AST node to represent namespace blocks
2. Tracking namespace context during AST traversal
3. Propagating namespace path to all items (functions, structs, externs)

### Architecture
The namespace is represented as a `Vec<String>` path, allowing nested namespaces.
For example, `namespace outer { namespace inner { } }` produces `["outer", "inner"]`.

Anonymous namespaces (used for internal linkage) result in an empty path, which
will need special handling in name mangling (future task).

## Implementation Summary

### Changes Made

1. **ast.rs**: Added `NamespaceDecl { name: Option<String> }` variant
2. **parse.rs**: Added `CXCursor_Namespace` handling to extract namespace name
3. **lib.rs**: Added `namespace: Vec<String>` field to `CppFunction`, `CppStruct`, `CppExtern`
4. **convert.rs**: Modified conversion to track and propagate namespace context

### Test Coverage
- `test_parse_namespace` - Basic namespace parsing
- `test_parse_anonymous_namespace` - Anonymous namespace parsing
- `test_namespace_function` - Function in namespace
- `test_nested_namespace` - Nested namespaces
- `test_namespace_struct` - Struct in namespace
- `test_anonymous_namespace` - Anonymous namespace conversion

## User Guide

### Usage
After parsing C++ code with namespaces, the `CppFunction`, `CppStruct`, and `CppExtern`
structures will have their `namespace` field populated:

```rust
let module = compile_cpp_file("path/to/file.cpp")?;
for func in &module.functions {
    println!("Function {} in namespace {:?}", func.display_name, func.namespace);
}
```

### Example
```cpp
namespace rrr {
    int compute(int x) { return x * 2; }
}
```
Results in a `CppFunction` with:
- `display_name`: "compute"
- `namespace`: ["rrr"]

### Limitations
- `using namespace` is not yet supported (task A.1.3)
- Namespace-qualified name resolution is not yet supported (task A.1.4)
