# Plan: Pointer Operations Support (Task 2.4.2)

## Date: 2026-01-17
## Status: COMPLETED

## Problem Statement

Pointer operations (`&x` and `*ptr`) in the C++ → MIR pipeline are incorrectly handled:
- `UnaryOp::AddrOf` (address-of `&x`) is mapped to `MirUnaryOp::Neg` (wrong!)
- `UnaryOp::Deref` (dereference `*ptr`) is mapped to `MirUnaryOp::Neg` (wrong!)

This causes incorrect codegen for any code that uses pointers.

## Current Flow (Broken)

```
C++ Source: int* ptr = &x; int y = *ptr;

Address-of (&x):
    UnaryOperator { op: AddrOf, ... }
    ↓
    convert_unaryop(AddrOf) → MirUnaryOp::Neg  // WRONG!
    ↓
    MirRvalue::UnaryOp { op: Neg, operand: x }
    ↓
    Result: Negates x instead of taking its address!

Dereference (*ptr):
    UnaryOperator { op: Deref, ... }
    ↓
    convert_unaryop(Deref) → MirUnaryOp::Neg  // WRONG!
    ↓
    MirRvalue::UnaryOp { op: Neg, operand: ptr }
    ↓
    Result: Negates ptr instead of dereferencing it!
```

## Target Flow (Fixed)

```
Address-of (&x):
    UnaryOperator { op: AddrOf, ... }
    ↓
    (special case, not via convert_unaryop)
    ↓
    MirRvalue::Ref { place: x, mutability: true }
    ↓
    Result: Takes address of x correctly

Dereference (*ptr):
    UnaryOperator { op: Deref, ... }
    ↓
    (special case, not via convert_unaryop)
    ↓
    MirPlace { local: ptr, projection: [Deref] }
    ↓
    MirOperand::Copy(place_with_deref)
    ↓
    Result: Reads value at address ptr correctly
```

## MIR Types Available

From `lib.rs`:
- `MirRvalue::Ref { place: MirPlace, mutability: bool }` - for address-of
- `MirProjection::Deref` - for dereference projection
- `MirPlace { local: usize, projection: Vec<MirProjection> }` - places with projections

## Implementation Steps

### Step 1: Modify UnaryOperator handling in convert_expr()

File: `crates/fragile-clang/src/convert.rs`

Current code (lines 890-905):
```rust
ClangNodeKind::UnaryOperator { op, ty } => {
    if let Some(operand_node) = node.children.first() {
        let operand = self.convert_expr(operand_node, builder)?;
        let result_local = builder.add_local(None, ty.clone(), false);

        let mir_op = convert_unaryop(*op);
        builder.add_statement(MirStatement::Assign {
            target: MirPlace::local(result_local),
            value: MirRvalue::UnaryOp { op: mir_op, operand },
        });

        Ok(MirOperand::Copy(MirPlace::local(result_local)))
    } else {
        Ok(MirOperand::Constant(MirConstant::Int { value: 0, bits: 32, signed: true }))
    }
}
```

Change to handle AddrOf and Deref specially:
```rust
ClangNodeKind::UnaryOperator { op, ty } => {
    if let Some(operand_node) = node.children.first() {
        match op {
            UnaryOp::AddrOf => {
                // Address-of: convert operand to a place, then take reference
                let operand = self.convert_expr(operand_node, builder)?;
                let place = match operand {
                    MirOperand::Copy(place) | MirOperand::Move(place) => place,
                    MirOperand::Constant(_) => {
                        // Can't take address of constant - store in temp first
                        let temp_local = builder.add_local(None, operand_node.get_type(), false);
                        builder.add_statement(MirStatement::Assign {
                            target: MirPlace::local(temp_local),
                            value: MirRvalue::Use(operand),
                        });
                        MirPlace::local(temp_local)
                    }
                };
                let result_local = builder.add_local(None, ty.clone(), false);
                builder.add_statement(MirStatement::Assign {
                    target: MirPlace::local(result_local),
                    value: MirRvalue::Ref { place, mutability: true },
                });
                Ok(MirOperand::Copy(MirPlace::local(result_local)))
            }
            UnaryOp::Deref => {
                // Dereference: convert operand to place, add Deref projection
                let operand = self.convert_expr(operand_node, builder)?;
                match operand {
                    MirOperand::Copy(mut place) | MirOperand::Move(mut place) => {
                        place.projection.push(MirProjection::Deref);
                        Ok(MirOperand::Copy(place))
                    }
                    MirOperand::Constant(_) => {
                        // Dereferencing a constant pointer - store in temp first
                        let temp_local = builder.add_local(None, operand_node.get_type(), false);
                        builder.add_statement(MirStatement::Assign {
                            target: MirPlace::local(temp_local),
                            value: MirRvalue::Use(operand),
                        });
                        let mut place = MirPlace::local(temp_local);
                        place.projection.push(MirProjection::Deref);
                        Ok(MirOperand::Copy(place))
                    }
                }
            }
            _ => {
                // Other unary ops: use existing path
                let operand = self.convert_expr(operand_node, builder)?;
                let result_local = builder.add_local(None, ty.clone(), false);
                let mir_op = convert_unaryop(*op);
                builder.add_statement(MirStatement::Assign {
                    target: MirPlace::local(result_local),
                    value: MirRvalue::UnaryOp { op: mir_op, operand },
                });
                Ok(MirOperand::Copy(MirPlace::local(result_local)))
            }
        }
    } else {
        Ok(MirOperand::Constant(MirConstant::Int { value: 0, bits: 32, signed: true }))
    }
}
```

### Step 2: Add helper method to get type from ClangNode

File: `crates/fragile-clang/src/convert.rs`

We need a way to get the type of an operand node for the constant case. Add:
```rust
impl ClangNode {
    fn get_cpp_type(&self) -> CppType {
        match &self.kind {
            ClangNodeKind::IntegerLiteral { cpp_type, .. } => cpp_type.clone().unwrap_or(CppType::Int { signed: true }),
            ClangNodeKind::UnaryOperator { ty, .. } => ty.clone(),
            ClangNodeKind::BinaryOperator { ty, .. } => ty.clone(),
            // ... etc
            _ => CppType::Int { signed: true }, // Default
        }
    }
}
```

Or we can get the type from the operand node's ClangNodeKind variant.

### Step 3: Remove AddrOf/Deref from convert_unaryop

File: `crates/fragile-clang/src/convert.rs`

Since AddrOf and Deref are now handled specially, we can remove them from convert_unaryop or keep them as unreachable for safety:
```rust
UnaryOp::AddrOf => unreachable!("AddrOf handled specially in convert_expr"),
UnaryOp::Deref => unreachable!("Deref handled specially in convert_expr"),
```

## Test Plan

Create test in `crates/fragile-clang/src/convert.rs`:

```rust
#[test]
fn test_convert_address_of() {
    let parser = ClangParser::new().unwrap();
    let ast = parser.parse_string(
        r#"
        int get_addr() {
            int x = 42;
            int* ptr = &x;
            return 0;
        }
        "#,
        "test.cpp",
    ).unwrap();
    let module = convert_module(&ast).unwrap();
    let func = module.functions.get("get_addr").unwrap();
    // Verify MirRvalue::Ref is used
    let has_ref = func.body.as_ref().unwrap().basic_blocks.iter()
        .flat_map(|bb| &bb.statements)
        .any(|stmt| matches!(stmt, MirStatement::Assign { value: MirRvalue::Ref { .. }, .. }));
    assert!(has_ref, "Should have MirRvalue::Ref for address-of");
}

#[test]
fn test_convert_dereference() {
    let parser = ClangParser::new().unwrap();
    let ast = parser.parse_string(
        r#"
        int deref_ptr(int* ptr) {
            return *ptr;
        }
        "#,
        "test.cpp",
    ).unwrap();
    let module = convert_module(&ast).unwrap();
    let func = module.functions.get("deref_ptr").unwrap();
    // Verify MirProjection::Deref is used
    let has_deref = func.body.as_ref().unwrap().basic_blocks.iter()
        .flat_map(|bb| &bb.statements)
        .any(|stmt| {
            if let MirStatement::Assign { value: MirRvalue::Use(MirOperand::Copy(place)), .. } = stmt {
                place.projection.iter().any(|p| matches!(p, MirProjection::Deref))
            } else {
                false
            }
        });
    assert!(has_deref, "Should have MirProjection::Deref for dereference");
}
```

## Files Changed

1. `crates/fragile-clang/src/convert.rs` - Handle AddrOf/Deref specially (~60 lines)

## Estimated LOC: ~80 lines (including tests)
