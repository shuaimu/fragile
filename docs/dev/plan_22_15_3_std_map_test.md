# Plan: Task 22.15.3 - Test std::map/std::unordered_map Operations

## Overview

Add E2E tests for `std::map` and `std::unordered_map` operations, following the same pattern as tasks 22.15.1 (std::vector) and 22.15.2 (std::string).

## Approach

Since we're not fully transpiling libc++ yet, we need to:
1. Add `std_map_int_int` and `std_unordered_map_int_int` stub structs in the preamble
2. Implement basic map operations as methods on the stubs
3. Add E2E tests that exercise these operations

## std_unordered_map_int_int Stub Design

The stub will implement a minimal `std::unordered_map<int, int>` equivalent using a simple hash table:

```rust
#[repr(C)]
pub struct std_unordered_map_int_int {
    _buckets: Vec<Vec<(i32, i32)>>,  // Hash buckets
    _size: usize,
}

impl std_unordered_map_int_int {
    pub fn new_0() -> Self;                  // Default constructor
    pub fn size(&self) -> usize;             // Element count
    pub fn empty(&self) -> bool;             // Check if empty
    pub fn insert(&mut self, key: i32, value: i32);  // Insert key-value
    pub fn find(&self, key: i32) -> Option<i32>;     // Find value by key
    pub fn op_index(&mut self, key: i32) -> &mut i32; // operator[]
    pub fn contains(&self, key: i32) -> bool;        // Check key existence
    pub fn erase(&mut self, key: i32) -> bool;       // Remove by key
    pub fn clear(&mut self);                         // Remove all elements
}
```

## Implementation Steps

1. Add `std_unordered_map_int_int` stub struct (~120 LOC)
2. Implement methods with simple bucket-based hash table (~100 LOC)
3. Add E2E test for unordered_map operations (~50 LOC)

## Test Cases

```rust
// Test 1: Default constructor
let mut m = std_unordered_map_int_int::new_0();
if !m.empty() { exit(1); }

// Test 2: Insert and find
m.insert(1, 100);
m.insert(2, 200);
if m.size() != 2 { exit(2); }
if m.find(1) != Some(100) { exit(3); }

// Test 3: operator[] access
*m.op_index(3) = 300;
if m.find(3) != Some(300) { exit(4); }

// Test 4: contains
if !m.contains(1) { exit(5); }
if m.contains(99) { exit(6); }

// Test 5: erase
m.erase(1);
if m.contains(1) { exit(7); }

// Test 6: clear
m.clear();
if !m.empty() { exit(8); }
```

## Estimated LOC

- Stub implementation: ~220 LOC
- E2E test: ~60 LOC
- Total: ~280 LOC (< 500 LOC threshold)

## Note on std::map

`std::map` is a sorted map (red-black tree). For a full implementation, we'd need a balanced tree.
For this stub, we can use a simpler approach (sorted Vec) since it's just for testing basic operations.
We can defer `std_map` stub to a separate sub-task if needed.
