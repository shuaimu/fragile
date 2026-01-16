# Plan: Access Specifiers (A.2.1)

**Status:** Completed [26:01:15, 23:46]

## Design Rationale

### Problem
C++ access specifiers (public, private, protected) control member visibility.
This is essential for:
- Generating correct Rust struct visibility modifiers
- Enforcing encapsulation in cross-language calls
- Accurate representation of C++ classes

### Solution
We implement access specifier support by:
1. Adding `AccessSpecifier` enum to AST
2. Using `clang_getCXXAccessSpecifier()` to query field access
3. Storing access with each field in CppStruct
4. Generating appropriate visibility in Rust stubs

## Implementation Summary

### Changes Made

1. **ast.rs**: Added `AccessSpecifier` enum with `Public`, `Private`, `Protected` variants
   - Added `access: AccessSpecifier` field to `FieldDecl`
2. **parse.rs**: Added `get_access_specifier()` to query `clang_getCXXAccessSpecifier()`
3. **lib.rs**:
   - Added `is_class: bool` to `CppStruct`
   - Changed fields to `Vec<(String, CppType, AccessSpecifier)>`
   - Exported `AccessSpecifier`
4. **convert.rs**: Updated to pass `is_class` and `access` through conversion
5. **stubs.rs**: Updated to generate `pub` only for public fields

### Test Coverage
- `test_class_access_specifiers` - Class with public/private/protected fields
- `test_struct_default_access` - Struct with default public access
- `test_generate_struct_stub_with_private_fields` - Rust stub visibility generation

## User Guide

### Usage
After parsing C++ code, the `CppStruct` will have access specifiers:

```rust
let module = compile_cpp_file("path/to/file.cpp")?;
for s in &module.structs {
    for (name, ty, access) in &s.fields {
        match access {
            AccessSpecifier::Public => println!("pub {}: {}", name, ty),
            AccessSpecifier::Private => println!("{}: {} (private)", name, ty),
            AccessSpecifier::Protected => println!("{}: {} (protected)", name, ty),
        }
    }
}
```

### Rust Stub Generation
Generated Rust structs use `pub` only for public fields:

```cpp
class MyClass {
public:
    int x;
private:
    int y;
};
```

Generates:
```rust
#[repr(C)]
pub struct MyClass {
    pub x: i32,
    y: i32,
}
```
