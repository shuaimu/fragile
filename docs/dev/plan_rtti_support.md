# Plan: E.2 RTTI Support

## Overview

Add support for C++ Run-Time Type Information (RTTI): typeid and dynamic_cast.

## Clang Cursor Types

- `CXCursor_CXXTypeidExpr` (129) - typeid expression
- `CXCursor_CXXDynamicCastExpr` (125) - dynamic_cast expression

## Design

### 1. AST Node Types (ast.rs)

Add two new variants to `ClangNodeKind`:

```rust
/// typeid expression
TypeidExpr {
    /// Type of the result (std::type_info const&)
    result_ty: CppType,
},

/// dynamic_cast expression
DynamicCastExpr {
    /// Target type of the cast
    target_ty: CppType,
    /// Result type (may be pointer or reference)
    result_ty: CppType,
},
```

### 2. Parse Support (parse.rs)

Handle the cursor kinds in `convert_cursor_kind()`:

```rust
clang_sys::CXCursor_CXXTypeidExpr => {
    let result_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
    ClangNodeKind::TypeidExpr { result_ty }
}

clang_sys::CXCursor_CXXDynamicCastExpr => {
    let result_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
    // Target type is the type in the angle brackets
    ClangNodeKind::DynamicCastExpr { target_ty: result_ty.clone(), result_ty }
}
```

### 3. MIR Conversion

For now, convert these as:
- typeid: call to runtime type_info lookup
- dynamic_cast: call to runtime dynamic cast

## Test Plan

```cpp
#include <typeinfo>

void test_typeid() {
    int x = 42;
    const std::type_info& ti = typeid(x);
}

class Base { virtual void foo() {} };
class Derived : public Base {};

void test_dynamic_cast(Base* b) {
    Derived* d = dynamic_cast<Derived*>(b);
}
```

## Estimated LOC

- ast.rs: ~15 lines
- parse.rs: ~20 lines
- convert.rs: ~20 lines
- tests: ~60 lines
- Total: ~115 lines
