# Plan: Task 22.15.2 - Test std::string Operations

## Overview

Add E2E tests for `std::string` operations, following the same pattern as task 22.15.1 (std::vector).

## Approach

Since we're not fully transpiling libc++ yet, we need to:
1. Add a `std_string` stub struct in the preamble generation
2. Implement basic string operations as methods on the stub
3. Add E2E test(s) that exercise these operations

## std_string Stub Design

The stub will implement a minimal `std::string` equivalent:

```rust
#[repr(C)]
#[derive(Default)]
pub struct std_string {
    _data: *mut i8,      // char*
    _size: usize,
    _capacity: usize,
}

impl std_string {
    pub fn new_0() -> Self;                          // Default constructor
    pub fn new_1(s: *const i8) -> Self;              // From C string
    pub fn c_str(&self) -> *const i8;                // Get C string pointer
    pub fn size(&self) -> usize;                     // String length
    pub fn length(&self) -> usize;                   // Alias for size
    pub fn empty(&self) -> bool;                     // Check if empty
    pub fn push_back(&mut self, c: i8);              // Append character
    pub fn append(&mut self, s: *const i8) -> &mut Self;  // Append string
    pub fn op_plus_assign(&mut self, s: *const i8); // operator+=
}
```

## Implementation Steps

1. Add `std_string` stub struct (~80 LOC)
2. Implement constructor and basic accessors (~40 LOC)
3. Implement modification methods (~60 LOC)
4. Add `generated_structs.insert("std_string")` to avoid duplicates
5. Add E2E test for string operations (~40 LOC)

## Test Cases

```cpp
#include <string>
int main() {
    std::string s;
    if (!s.empty()) return 1;

    s.push_back('H');
    s.push_back('i');
    if (s.size() != 2) return 2;

    // Test c_str()
    const char* cs = s.c_str();
    if (cs[0] != 'H' || cs[1] != 'i') return 3;

    return 0;
}
```

## Estimated LOC

- Stub implementation: ~150 LOC
- E2E test: ~40 LOC
- Total: ~190 LOC (< 500 LOC threshold)

## Dependencies

- Existing preamble generation infrastructure
- E2E test harness (`transpile_compile_run`)
