# Plan: Array Placement New (Task 17.2.2)

## Overview

Handle array placement new syntax: `new (ptr) T[n]`

## Current Implementation

The current placement new handles single objects:
- `new (ptr) T(args)` → `std::ptr::write(ptr, T::new(args))`

## Target Implementation

Array placement new:
- `new (ptr) T[n]` → construct n objects at ptr

```rust
// For: new (ptr) int[5] with value initialization
{
    let __ptr = ptr as *mut i32;
    debug_assert!((__ptr as usize) % std::mem::align_of::<i32>() == 0);
    unsafe {
        for __i in 0..5 {
            std::ptr::write(__ptr.add(__i), 0i32);
        }
    }
    __ptr
}
```

```rust
// For: new (ptr) MyClass[3]
{
    let __ptr = ptr as *mut MyClass;
    debug_assert!((__ptr as usize) % std::mem::align_of::<MyClass>() == 0);
    unsafe {
        for __i in 0..3 {
            std::ptr::write(__ptr.add(__i), MyClass::new());
        }
    }
    __ptr
}
```

## Implementation Steps

1. Check if CXXNewExpr has both `is_array` and `is_placement` set
2. Extract the array size expression from children
3. Generate loop that writes each element

## Files to Modify

- `crates/fragile-clang/src/ast_codegen.rs`:
  - Update CXXNewExpr handling for array + placement case

## Estimated LOC: ~50
