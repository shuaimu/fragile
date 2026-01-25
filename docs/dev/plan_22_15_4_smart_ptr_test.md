# Plan: Task 22.15.4 - Test Smart Pointer Usage

## Overview

Add E2E tests for smart pointer operations (`std::unique_ptr`, `std::shared_ptr`), following the same pattern as previous STL stub tasks.

## Approach

Since we're not fully transpiling libc++ yet, we need to:
1. Add `std_unique_ptr_int` and `std_shared_ptr_int` stub structs in the preamble
2. Implement basic smart pointer operations as methods on the stubs
3. Add E2E tests that exercise these operations

## std_unique_ptr_int Stub Design

The stub will implement a minimal `std::unique_ptr<int>` equivalent:

```rust
#[repr(C)]
pub struct std_unique_ptr_int {
    _ptr: *mut i32,
}

impl std_unique_ptr_int {
    pub fn new_0() -> Self;              // Default (null) constructor
    pub fn new_1(ptr: *mut i32) -> Self; // From raw pointer
    pub fn get(&self) -> *mut i32;       // Get raw pointer
    pub fn op_deref(&self) -> &mut i32;  // operator*
    pub fn op_arrow(&self) -> *mut i32;  // operator->
    pub fn release(&mut self) -> *mut i32; // Release ownership
    pub fn reset(&mut self);             // Reset to null
}
```

## std_shared_ptr_int Stub Design

The stub will implement a minimal `std::shared_ptr<int>` equivalent with reference counting:

```rust
#[repr(C)]
pub struct std_shared_ptr_int {
    _ptr: *mut i32,
    _refcount: *mut usize,  // Heap-allocated ref count
}

impl std_shared_ptr_int {
    pub fn new_0() -> Self;              // Default (null) constructor
    pub fn new_1(ptr: *mut i32) -> Self; // From raw pointer
    pub fn get(&self) -> *mut i32;       // Get raw pointer
    pub fn op_deref(&self) -> &mut i32;  // operator*
    pub fn use_count(&self) -> usize;    // Reference count
    pub fn reset(&mut self);             // Reset to null
}
impl Clone for std_shared_ptr_int     // Copy increases ref count
impl Drop for std_shared_ptr_int      // Decreases ref count, frees when 0
```

## Implementation Steps

1. Add `std_unique_ptr_int` stub struct (~60 LOC)
2. Add `std_shared_ptr_int` stub struct (~100 LOC)
3. Add E2E tests for both (~80 LOC)

## Test Cases

### unique_ptr tests:
- Default constructor creates null pointer
- Constructor from raw pointer holds the pointer
- op_deref returns the value
- release() returns pointer and clears ownership
- reset() frees memory

### shared_ptr tests:
- Default constructor creates null pointer
- Constructor from raw pointer, use_count == 1
- Clone increases use_count
- Drop decreases use_count
- Memory freed when use_count reaches 0

## Estimated LOC

- unique_ptr stub: ~60 LOC
- shared_ptr stub: ~100 LOC
- E2E tests: ~80 LOC
- Total: ~240 LOC (< 500 LOC threshold)
