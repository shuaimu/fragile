# Plan: dynamic_cast Improvements (Task 16.3)

## Overview

Improve dynamic_cast handling for reference types and deep hierarchies.

## Current Implementation

```rust
// Current (basic)
/* dynamic_cast */ expr as *mut Derived
```

## Issues

1. **Reference types**: `dynamic_cast<Derived&>(base)` should:
   - Throw `std::bad_cast` if cast fails (not return null)
   - Map to reference type in Rust

2. **Deep hierarchies**: Multiple inheritance chains need proper traversal

## Target Implementation

### 16.3.1: Improve trait object dynamic_cast

For polymorphic types with virtual methods, we already generate trait objects.
The cast should check runtime type information.

```rust
// For pointer dynamic_cast:
unsafe { fragile_dynamic_cast_ptr::<Base, Derived>(ptr) }
// Returns Option<*mut Derived>

// Helper function:
fn fragile_dynamic_cast_ptr<B, D>(ptr: *mut B) -> Option<*mut D>
where D: std::any::Any {
    // Use TypeId comparison
    if std::any::TypeId::of::<D>() == /* actual type */ {
        Some(ptr as *mut D)
    } else {
        None
    }
}
```

For now, keep the simple cast with a comment indicating runtime check needed.

### 16.3.2: Handle reference types

```cpp
// C++ input:
Derived& d = dynamic_cast<Derived&>(base);
```

```rust
// Rust output - reference dynamic_cast throws on failure
match fragile_dynamic_cast_ref::<Base, Derived>(&base) {
    Some(d) => d,
    None => panic!("std::bad_cast"),
}
```

## Implementation Steps

1. Detect if target_ty is a reference type (CppType::Reference)
2. For reference types:
   - Generate panic on failure instead of returning null
   - Use reference syntax instead of pointer
3. For pointer types:
   - Keep current behavior but improve comment

## Files to Modify

- `crates/fragile-clang/src/ast_codegen.rs`:
  - Update DynamicCastExpr handling in expr_to_string

## Estimated LOC: ~60
