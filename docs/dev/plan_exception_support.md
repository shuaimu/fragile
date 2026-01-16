# Plan: E.1 Exception Support

## Overview

Add support for C++ exception handling: try/catch/throw statements.

## Clang Cursor Types

Clang provides these cursor kinds for exceptions:
- `CXCursor_CXXThrowExpr` (133) - throw expression
- `CXCursor_CXXCatchStmt` (223) - catch handler
- `CXCursor_CXXTryStmt` (224) - try block

## Design

### 1. AST Node Types (ast.rs)

Add three new variants to `ClangNodeKind`:

```rust
/// try statement with catch handlers
TryStmt,

/// catch handler (exception type is a child)
CatchStmt {
    /// Type being caught (None for catch(...))
    exception_ty: Option<CppType>,
},

/// throw expression
ThrowExpr {
    /// Type being thrown (from the expression)
    exception_ty: Option<CppType>,
},
```

### 2. MIR Representation

For now, we'll represent exceptions as:
- TryStmt as a compound stmt with cleanup regions
- CatchStmt as conditional blocks
- ThrowExpr as a call to runtime exception handling

Later phases can add proper unwind handling.

### 3. Parse Support (parse.rs)

Handle the cursor kinds in `convert_cursor_kind()`:

```rust
clang_sys::CXCursor_CXXTryStmt => ClangNodeKind::TryStmt,
clang_sys::CXCursor_CXXCatchStmt => {
    // Get exception type from first child if it's a VarDecl
    ClangNodeKind::CatchStmt { exception_ty }
}
clang_sys::CXCursor_CXXThrowExpr => {
    // Get thrown type from child expression
    ClangNodeKind::ThrowExpr { exception_ty }
}
```

## Implementation Steps

1. Add `TryStmt`, `CatchStmt`, `ThrowExpr` to `ClangNodeKind` enum
2. Add match arms in `convert_cursor_kind()` for the three cursor types
3. Add MIR conversion (placeholder for now)
4. Add tests for exception parsing

## Test Plan

```cpp
void test_exceptions() {
    try {
        throw 42;
    } catch (int e) {
        // handle int
    } catch (...) {
        // handle all
    }
}
```

## Estimated LOC

- ast.rs: ~20 lines
- parse.rs: ~30 lines
- convert.rs: ~30 lines
- tests: ~60 lines
- Total: ~140 lines
