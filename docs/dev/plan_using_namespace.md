# Plan: Using Namespace Directive (A.1.3)

**Status:** Completed [26:01:15, 23:41]

## Design Rationale

### Problem
C++ `using namespace` directives import names from one namespace into another scope,
allowing unqualified access to those names. This is essential for common C++ patterns
like `using namespace std;`.

### Solution
We implement using directive support by:
1. Adding AST nodes for `UsingDirective` and `UsingDeclaration`
2. Parsing these nodes via libclang's `CXCursor_UsingDirective` and `CXCursor_UsingDeclaration`
3. Extracting namespace paths by visiting `NamespaceRef` children
4. Storing directives with their scope context for later name resolution

### Implementation Details
- libclang represents using directives with `NamespaceRef` children pointing to the namespaces
- We traverse children to collect the full namespace path
- The scope where a directive appears is tracked for proper name lookup in Phase A.1.4

## Implementation Summary

### Changes Made

1. **ast.rs**: Added `UsingDirective { namespace: Vec<String> }` and `UsingDeclaration { qualified_name: Vec<String> }` variants
2. **parse.rs**:
   - Added `CXCursor_UsingDirective` and `CXCursor_UsingDeclaration` handling
   - Added `get_using_directive_namespace()` to extract namespace path from NamespaceRef children
3. **lib.rs**:
   - Added `UsingDirective` and `UsingDeclaration` structs
   - Added `using_directives` and `using_declarations` fields to `CppModule`
4. **convert.rs**: Added conversion for using directives/declarations with scope tracking

### Test Coverage
- `test_parse_using_namespace` - Basic using directive parsing
- `test_parse_using_nested_namespace` - Nested namespace using directive
- `test_using_namespace_conversion` - Module conversion with using directive
- `test_using_namespace_in_scope` - Using directive inside a namespace scope
- `test_using_nested_namespace_conversion` - Nested namespace module conversion

## User Guide

### Usage
After parsing C++ code with using directives, the `CppModule` will contain:

```rust
let module = compile_cpp_file("path/to/file.cpp")?;
for using_dir in &module.using_directives {
    println!("Using namespace {:?} in scope {:?}",
             using_dir.namespace, using_dir.scope);
}
```

### Example
```cpp
namespace foo {
    int x;
}
namespace bar {
    using namespace foo;
}
```
Results in a `UsingDirective` with:
- `namespace`: ["foo"]
- `scope`: ["bar"]

### Limitations
- Name resolution using these directives is not yet implemented (task A.1.4)
- `using namespace` inside functions is parsed but stored at module level
