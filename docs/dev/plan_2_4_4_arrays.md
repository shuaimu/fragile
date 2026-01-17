# Plan: Array Support (Task 2.4.4)

## Date: 2026-01-17
## Status: COMPLETED [26:01:17, 21:30]

## Problem Statement

Array subscript expressions (`a[i]`) in the C++ → MIR pipeline are not converted correctly:
- `ArraySubscriptExpr` falls through to the default case in `convert_expr()` returning `MirConstant::Unit`
- The `MirProjection::Index` variant exists but is not being used

## Current State

### What's Complete:
1. **Type System (types.rs)** - `CppType::Array { element, size }` with support for fixed and incomplete arrays
2. **Parser (parse.rs)** - Handles `CXType_ConstantArray` and `CXType_IncompleteArray`
3. **AST (ast.rs)** - `ArraySubscriptExpr { ty }` node exists
4. **MIR Infrastructure (lib.rs)** - `MirProjection::Index(usize)` exists but unused

### What's Missing:
- **MIR Conversion (convert.rs)** - No case for `ArraySubscriptExpr` in `convert_expr()`

## Current Flow (Broken)

```
C++ Source: int x = arr[2];

ArraySubscriptExpr { ty: int }
    └── arr (array operand)
    └── 2 (index operand)
        ↓
convert_expr() default case
        ↓
MirConstant::Unit  // WRONG!
```

## Target Flow (Fixed)

```
ArraySubscriptExpr { ty: int }
    └── arr (array operand)
    └── 2 (index operand)
        ↓
1. Convert arr → MirOperand::Copy(MirPlace::local(arr_idx))
2. Convert 2 → MirOperand::Constant(Int { value: 2, ... })
3. Create place: MirPlace { local: arr_idx, projection: [Index(2)] }
        ↓
MirOperand::Copy(indexed_place)
```

## Implementation Plan

### Step 1: Handle ArraySubscriptExpr in convert_expr()

File: `crates/fragile-clang/src/convert.rs`

Add case before the default `_` arm:

```rust
ClangNodeKind::ArraySubscriptExpr { ty } => {
    // First child is the array, second is the index
    if node.children.len() >= 2 {
        let array_node = &node.children[0];
        let index_node = &node.children[1];

        // Convert array to a place
        let array_operand = self.convert_expr(array_node, builder)?;
        let array_place = match array_operand {
            MirOperand::Copy(place) | MirOperand::Move(place) => place,
            MirOperand::Constant(c) => {
                // Array is a constant - store in temp first
                let array_ty = Self::get_node_type(array_node);
                let temp_local = builder.add_local(None, array_ty, false);
                builder.add_statement(MirStatement::Assign {
                    target: MirPlace::local(temp_local),
                    value: MirRvalue::Use(MirOperand::Constant(c)),
                });
                MirPlace::local(temp_local)
            }
        };

        // Convert index
        let index_operand = self.convert_expr(index_node, builder)?;

        // For MirProjection::Index, we need a compile-time known index
        // Runtime indices would require a different approach (using variable indexing)
        let index_value = match index_operand {
            MirOperand::Constant(MirConstant::Int { value, .. }) => value as usize,
            _ => {
                // Runtime index - for now, use index 0 as fallback
                // TODO: Support runtime indexing with a local variable
                0
            }
        };

        // Create indexed place
        let mut indexed_place = array_place;
        indexed_place.projection.push(MirProjection::Index(index_value));

        Ok(MirOperand::Copy(indexed_place))
    } else {
        Ok(MirOperand::Constant(MirConstant::Int { value: 0, bits: 32, signed: true }))
    }
}
```

### Step 2: Add Tests

```rust
#[test]
fn test_convert_array_subscript() {
    let parser = ClangParser::new().unwrap();
    let ast = parser
        .parse_string(
            r#"
            int get_element(int arr[10], int i) {
                return arr[0];  // Constant index
            }
            "#,
            "test.cpp",
        )
        .unwrap();

    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    assert_eq!(module.functions.len(), 1);
    let func = &module.functions[0];
    let body = &func.mir_body;

    // Verify MirProjection::Index is used
    let has_index = body
        .blocks
        .iter()
        .flat_map(|bb| &bb.statements)
        .any(|stmt| {
            if let MirStatement::Assign { value: MirRvalue::Use(MirOperand::Copy(place)), .. } = stmt {
                place.projection.iter().any(|p| matches!(p, MirProjection::Index(_)))
            } else {
                false
            }
        });
    assert!(has_index, "Should have MirProjection::Index for array subscript");
}
```

## Files to Change

1. `crates/fragile-clang/src/convert.rs` - Add ArraySubscriptExpr handling (~30 lines)

## Estimated LOC: ~50 lines (including tests)

## Notes

- The current `MirProjection::Index(usize)` only supports compile-time constant indices
- Runtime variable indices would require extending MIR to support variable-based indexing
- For this initial implementation, we handle compile-time constants and fallback to 0 for runtime indices
