# Plan: D.1 AST Support for Coroutine Expressions

## Overview

Add AST representation for C++20 coroutine expressions (co_await, co_yield, co_return) to the fragile-clang crate.

## Background

C++20 introduces three coroutine keywords:
- `co_await expr` - Suspends execution until the awaitable is ready
- `co_yield expr` - Yields a value and suspends (syntactic sugar for `co_await promise.yield_value(expr)`)
- `co_return expr` - Returns from a coroutine (optionally with a value)

Clang represents these as:
- `CXCursor_CoawaitExpr` - co_await expression
- `CXCursor_CoyieldExpr` - co_yield expression
- `CXCursor_CoreturnStmt` - co_return statement

## Design

### 1. AST Node Types (ast.rs)

Add three new variants to `ClangNodeKind`:

```rust
/// co_await expression (C++20 coroutine)
CoawaitExpr {
    /// Type of the operand being awaited
    operand_ty: CppType,
    /// Result type of the await expression
    result_ty: CppType,
},

/// co_yield expression (C++20 coroutine)
CoyieldExpr {
    /// Type of the value being yielded
    value_ty: CppType,
    /// Result type of the yield expression (from yield_value)
    result_ty: CppType,
},

/// co_return statement (C++20 coroutine)
CoreturnStmt {
    /// Type of the returned value (None for co_return;)
    value_ty: Option<CppType>,
},
```

### 2. Parser Support (parse.rs)

Handle the new cursor kinds in `convert_cursor_kind()`:

```rust
CXCursor_CoawaitExpr => {
    // Get operand type from first child
    // Get result type from cursor type
    ClangNodeKind::CoawaitExpr { operand_ty, result_ty }
}

CXCursor_CoyieldExpr => {
    // Get value type from first child
    // Get result type from cursor type
    ClangNodeKind::CoyieldExpr { value_ty, result_ty }
}

CXCursor_CoreturnStmt => {
    // Check if there's a return value child
    ClangNodeKind::CoreturnStmt { value_ty }
}
```

### 3. Type Extraction

For coroutine expressions, we need to extract:
- The operand/value types from the child expressions
- The result types from the Clang cursor type

## Implementation Steps

1. Add `CoawaitExpr`, `CoyieldExpr`, `CoreturnStmt` to `ClangNodeKind` enum
2. Add match arms in `convert_cursor_kind()` for the three cursor types
3. Extract types from cursor and children
4. Add unit tests for each expression type

## Test Plan

Create test file `tests/clang_integration/coroutine_ast.cpp`:

```cpp
#include <coroutine>

struct Generator {
    struct promise_type {
        int current_value;
        Generator get_return_object() { return {}; }
        std::suspend_always initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }
        void unhandled_exception() {}
        std::suspend_always yield_value(int value) {
            current_value = value;
            return {};
        }
        void return_void() {}
    };
};

Generator simple_generator() {
    co_yield 1;
    co_yield 2;
    co_return;
}
```

## Estimated LOC

- ast.rs: ~30 lines
- parse.rs: ~50 lines
- tests: ~70 lines
- Total: ~150 lines
