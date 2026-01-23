# Plan: C++20 Designated Initializers Support

## Task
Implement support for C++20 designated initializers (`{ .field = value }` syntax).

## Analysis

### C++ Designated Initializers
- C++20 feature for aggregate initialization with named fields
- Syntax: `Point p = { .x = 10, .y = 20 };`
- Fields can be specified in any order (Clang reorders them)

### Clang AST Structure

For designated initializers like `{ .x = 10, .y = 20 }`, Clang produces:
```
InitListExpr { ty: Point }
  UnexposedExpr
    MemberRef { name: "x" }  <- field designator
    IntegerLiteral { 10 }    <- value
  UnexposedExpr
    MemberRef { name: "y" }
    IntegerLiteral { 20 }
```

For non-designated initializers like `{ 10, 20 }`:
```
InitListExpr { ty: Point }
  IntegerLiteral { 10 }
  IntegerLiteral { 20 }
```

### Implementation Strategy

1. Detect designated vs non-designated by checking if InitListExpr children are wrapped in UnexposedExpr with MemberRef
2. For designated: extract field names from MemberRef nodes
3. For non-designated: use positional mapping from class_fields

## Implementation Complete

**Date**: 2026-01-22

**Changes Made**:
1. Modified `InitListExpr` handler in `ast_codegen.rs` to detect and handle designated initializers
2. Extract field names from MemberRef nodes inside UnexposedExpr wrappers
3. Generate proper Rust struct initialization: `Struct { field: value, ... }`
4. Added E2E test covering both designated and non-designated initializers

**Key Code Location**: `crates/fragile-clang/src/ast_codegen.rs`, lines 3686-3746

**Test Added**: `test_e2e_designated_initializers` in `crates/fragile-clang/tests/integration_test.rs`
