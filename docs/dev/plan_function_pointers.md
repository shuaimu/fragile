# Plan: Function Pointer Support

## Task
Complete function pointer codegen for C++ to Rust transpilation.

## Analysis

### C++ Function Pointers
```cpp
int (*ptr)(int, int) = add;  // Function pointer declaration and init
ptr(3, 4);                    // Function pointer call
ptr = multiply;               // Reassignment
```

### Current Issues
1. **Type**: `Pointer { pointee: Function { ... } }` was generating `*mut fn(...)`
2. **Assignments**: Function-to-pointer decay not handled
3. **Calls**: Direct call on Option type doesn't work
4. **Null**: `nullptr` should be `None`, not `null_mut()`

### Solution: Use Option<extern "C" fn(...)>
Rust function pointers are non-null, so use `Option` to represent nullable C++ function pointers.

## Implementation Progress

### Task 10.1.1: Type Generation ✅
Modified `CppType::Pointer` handling in `to_rust_type_str()` to detect function pointees and generate `Option<extern "C" fn(...)>`.

**File**: `crates/fragile-clang/src/types.rs`

### Task 10.1.2: Function-to-Pointer Decay (TODO)
When assigning a function name to a function pointer variable, wrap with `Some()`.

### Task 10.1.3: Function Pointer Calls (TODO)
When calling through a function pointer, use `.unwrap()()` or `if let Some(f) = ptr { f(...) }`.

### Task 10.1.4: Null Initialization (TODO)
When initializing with nullptr or no value, use `None` instead of `std::ptr::null_mut()`.

## Testing
The type generation change doesn't break existing tests (57/57 pass).
Full function pointer E2E test will be added after all subtasks are complete.
