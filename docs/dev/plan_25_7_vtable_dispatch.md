# Plan: Task 25.7 - Generate Virtual Call Dispatch

## Overview

Switch from trait-based polymorphism to vtable-based dispatch for virtual method calls.

## Current State

The vtable infrastructure (Tasks 25.1-25.6) is complete:
- `{ClassName}_vtable` structs with function pointers
- `__vtable` pointer as first field in ROOT polymorphic classes
- Static `{CLASS}_VTABLE` instances for concrete classes
- Vtable wrapper functions that call actual methods
- Constructors set vtable pointers correctly

Current dispatch (trait-based):
```rust
// For a->speak() where a: *mut Animal
a.speak()  // Uses AnimalTrait implementation
```

Target dispatch (vtable-based):
```rust
// For a->speak() where a: *mut Animal
unsafe { ((*(*a).__vtable).speak)(a) }
```

## Analysis

The key code is in `expr_to_string()` for `MemberExpr` (line 9444-9447):
```rust
if is_trait_object {
    format!("{}.{}", base, member)  // e.g., "a.speak"
}
```

This generates member access that becomes a method call. For vtable dispatch, we need:
1. Access the vtable pointer: `(*a).__vtable`
2. Get the function pointer: `(*(*a).__vtable).speak`
3. Call with object as first arg: `((*(*a).__vtable).speak)(a)`

## Challenge

The MemberExpr for `a->speak` generates `a.speak`, then CallExpr adds `()` to make `a.speak()`.

For vtable dispatch:
- MemberExpr should generate function pointer access
- But we also need to pass `a` as the first argument

The issue is that CallExpr doesn't currently add `a` as an argument - it just calls the result of MemberExpr.

## Solution Options

### Option A: Modify MemberExpr to generate callable expression
Generate a complete callable that includes the self argument when called.

This is complex because MemberExpr doesn't know the call arguments.

### Option B: Modify CallExpr to detect virtual method calls
When CallExpr sees a call to a virtual method on a polymorphic pointer, generate vtable dispatch.

This is cleaner because CallExpr has access to all arguments.

### Option C: Use a marker in MemberExpr output
MemberExpr generates `__VTABLE_DISPATCH(base, method)` marker, CallExpr recognizes and transforms it.

## Chosen Approach: Option B

Modify CallExpr handling to detect virtual method calls and generate vtable dispatch.

Detection:
1. First child is MemberExpr with `is_arrow=true`
2. Base type is pointer to polymorphic class
3. Member is a virtual method (in vtables HashMap)

Generation:
```rust
// Instead of: a.speak()
// Generate: unsafe { ((*(*a).__vtable).speak)(a) }
```

## Implementation Steps

### 25.7.1 Detect virtual method calls in CallExpr

In `expr_to_string()` for `CallExpr`, check:
- Has a MemberExpr child with arrow access
- MemberExpr points to polymorphic class
- Member name is a virtual method

### 25.7.2 Generate vtable dispatch

For detected virtual calls, generate:
```rust
unsafe { ((*(*{base}).__vtable).{method})({base}, {args...}) }
```

Need to:
- Get the base expression
- Get the method name
- Collect other arguments
- Generate the vtable function pointer call

### 25.7.3 Handle inheritance chain

For derived class pointers calling inherited methods:
- The vtable pointer is in the root class
- Need to navigate through `__base` fields

## Testing

- Existing virtual method tests should pass with new dispatch
- test_e2e_dynamic_dispatch
- test_e2e_virtual_override
- test_e2e_virtual_diamond

## Estimated LOC

- Detection logic: ~30 LOC
- Dispatch generation: ~50 LOC
- Helper functions: ~20 LOC
- Total: ~100 LOC
