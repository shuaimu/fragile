# Plan: Struct Literals / Aggregate Initialization (Task 3.1.3)

## Date: 2026-01-17
## Status: COMPLETED [26:01:17, 22:45]

## Problem Statement

C++ aggregate initialization (`Point{1, 2}`) was not being converted to MIR:
- `InitListExpr` was not parsed or converted
- No `MirRvalue::Aggregate` variant existed for aggregate construction
- Cast expressions (like `CXXFunctionalCastExpr`) weren't properly finding the value child

## Changes Made

### 1. AST Node (ast.rs)

Added `InitListExpr` variant:
```rust
InitListExpr {
    /// Result type of the initialization (the struct/array type)
    ty: CppType,
}
```

### 2. Parser (parse.rs)

Added handling for:
- `CXCursor_InitListExpr` - initialization list expressions
- `CXCursor_CXXFunctionalCastExpr` - functional cast (treated as CastExpr)

### 3. MIR Representation (lib.rs)

Added `MirRvalue::Aggregate` variant:
```rust
Aggregate {
    /// The aggregate type (struct, array, etc.)
    ty: CppType,
    /// Field values in order, with optional field names
    fields: Vec<(Option<String>, MirOperand)>,
}
```

### 4. MIR Conversion (convert.rs)

Added `InitListExpr` handling:
- Converts each child expression to an operand
- Creates a temporary local to hold the aggregate
- Assigns `MirRvalue::Aggregate` to the temporary
- Returns `MirOperand::Copy(temp)` for use in surrounding expressions

Fixed `CastExpr` handling:
- Now uses `is_expression_kind()` to find the actual value child
- Skips non-expression children like `TypeRef` that Clang inserts

### 5. rustc Driver (mir_convert.rs)

Added handling for `MirRvalue::Aggregate`:
- Converts to `Rvalue::Aggregate` with `AggregateKind::Tuple` as placeholder
- Full struct type resolution is a future improvement

### 6. Tests

Added 3 tests:
- `test_convert_init_list_struct`: Struct aggregate init `Point{1, 2}`
- `test_convert_init_list_variable`: Local variable initialization
- `test_convert_init_list_array`: Array initialization `{1, 2, 3}`

## Files Changed

1. `crates/fragile-clang/src/ast.rs` - Added `InitListExpr` node kind
2. `crates/fragile-clang/src/parse.rs` - Added parser handling
3. `crates/fragile-clang/src/lib.rs` - Added `MirRvalue::Aggregate`
4. `crates/fragile-clang/src/convert.rs` - Added conversion + 3 tests
5. `crates/fragile-rustc-driver/src/mir_convert.rs` - Added aggregate handling

## Design Decisions

1. **Field names are optional**: Init list order corresponds to field order, so names aren't strictly needed for aggregate init. Names can be added later for designated initializers.

2. **Temporary for aggregate value**: The aggregate is assigned to a temporary, then that temporary is used. This matches MIR's SSA-like nature.

3. **Tuple placeholder for rustc**: The rustc driver uses `AggregateKind::Tuple` as a placeholder since proper struct type resolution requires more type system work.

## Future Improvements

- Designated initializers (`Point{.x = 1, .y = 2}`)
- Proper struct type resolution in rustc driver
- Partial initialization with default values
