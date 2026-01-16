# Plan: D.3 AST to MIR Conversion for Coroutines

## Overview

Convert C++20 coroutine AST nodes to MIR representation.

## Design

### 1. CoreturnStmt Conversion

In `convert_stmt`, add handling for `CoreturnStmt`:

```rust
ClangNodeKind::CoreturnStmt { value_ty } => {
    let value = if let Some(expr) = node.children.first() {
        Some(self.convert_expr(expr, builder)?)
    } else {
        None
    };
    builder.finish_block(MirTerminator::CoroutineReturn { value });
}
```

### 2. CoyieldExpr Conversion

In `convert_expr`, add handling for `CoyieldExpr`:

```rust
ClangNodeKind::CoyieldExpr { value_ty, result_ty } => {
    // Get the value being yielded
    let value = if let Some(expr) = node.children.first() {
        self.convert_expr(expr, builder)?
    } else {
        MirOperand::Constant(MirConstant::Unit)
    };

    // Create yield terminator
    let resume_block = builder.new_block();
    builder.finish_block(MirTerminator::Yield {
        value,
        resume: resume_block,
        drop: None,
    });

    // The yield expression returns a value (typically void for generators)
    Ok(MirOperand::Constant(MirConstant::Unit))
}
```

### 3. CoawaitExpr Conversion

In `convert_expr`, add handling for `CoawaitExpr`:

```rust
ClangNodeKind::CoawaitExpr { operand_ty, result_ty } => {
    // Get the awaitable being awaited
    let awaitable = if let Some(expr) = node.children.first() {
        self.convert_expr(expr, builder)?
    } else {
        MirOperand::Constant(MirConstant::Unit)
    };

    // Create a temporary for the result
    let result_local = builder.add_local(None, result_ty.clone(), false);
    let destination = MirPlace::local(result_local);

    // Create await terminator
    let resume_block = builder.new_block();
    builder.finish_block(MirTerminator::Await {
        awaitable,
        destination: destination.clone(),
        resume: resume_block,
        drop: None,
    });

    // Return the result place as an operand
    Ok(MirOperand::Copy(destination))
}
```

### 4. Update is_expression_kind

Add coroutine expressions to the expression kind check:

```rust
| ClangNodeKind::CoawaitExpr { .. }
| ClangNodeKind::CoyieldExpr { .. }
```

## Implementation Steps

1. Add `CoreturnStmt` case to `convert_stmt`
2. Add `CoawaitExpr` case to `convert_expr`
3. Add `CoyieldExpr` case to `convert_expr`
4. Update `is_expression_kind` to include coroutine expressions
5. Add integration tests for MIR conversion

## Test Plan

Add tests that verify:
- CoreturnStmt generates CoroutineReturn terminator
- CoyieldExpr generates Yield terminator
- CoawaitExpr generates Await terminator
- is_coroutine flag is set on MirBody

## Estimated LOC

- convert.rs: ~60 lines
- tests: ~50 lines
- Total: ~110 lines
