# Plan: Fix Subscript Operator [] Implementation

## Problem

The subscript operator (`operator[]`) is not working correctly. There are several issues:

### Issue 1: Self mutability

C++ `int& operator[](int idx)` returns a mutable reference. The transpiler generates:
```rust
pub fn op_index(&self, idx: i32) -> &mut i32 { ... }
```

This is invalid Rust - you can't return `&mut T` from `&self`. The fix should detect that the return type is a mutable reference and generate `&mut self`.

### Issue 2: Argument passing

The generated call `arr.op_index(&3i32)` wraps the integer with `&`. This is incorrect - the operator takes a value parameter, not a reference.

### Issue 3: Return value handling

The generated code:
```rust
return self.data[idx as usize];
```

Should be:
```rust
return &mut self.data[idx as usize];
```

## Analysis

### Root Cause 1: Self mutability detection

In `ast_codegen.rs`, method generation checks if a method modifies `self` to decide between `&self` and `&mut self`. For `operator[]`, we need to also check if the return type is a mutable reference.

### Root Cause 2: Argument passing

The subscript operator is being detected as needing reference arguments, but this is incorrect for the subscript operator specifically.

### Root Cause 3: Missing address-of

When the return type is a reference (`int&`), we need to take the address of the return expression.

## Solution

1. **For self mutability**: In method generation, if return type is a mutable reference (`T&` where T is not const), use `&mut self`

2. **For argument passing**: Don't add `&` to subscript operator arguments (already handles value semantics)

3. **For return**: When return type is `&T` or `&mut T`, generate `&` or `&mut` around the return expression if it's not already a reference

## Implementation Steps

1. Find where method self-type is determined
2. Add check for return type being mutable reference
3. Find subscript operator call handling and fix argument wrapping
4. Find return statement handling for reference returns
