# Plan: M5.8 - Run Basic Mako Operations

## Overview

This milestone validates the end-to-end compilation pipeline by running actual mako code from Rust. We'll start with the simplest possible mako functions.

## Current State

- M5.7.3: Linking infrastructure works (Rust can call C++ functions)
- Simple add.cpp test passes
- Need to extend to actual mako code

## Challenges

1. **Header dependencies**: Mako code includes STL headers (string.h, vector, iostream)
2. **Namespace mangling**: Functions in `rrr::` namespace have mangled names
3. **C++ runtime**: STL functions require full C++ runtime

## Approach: Minimal Isolated Test

Create a minimal C++ file that mimics mako's simplest functions without dependencies.

### Candidate: Create mako_simple.cpp

```cpp
// Isolated mako utility functions for testing
// No STL dependencies, only basic C types

namespace rrr {

// Simple string operations using only C functions
bool startswith(const char* str, const char* head) {
    // Simplified implementation
    while (*head) {
        if (*str != *head) return false;
        str++;
        head++;
    }
    return true;
}

bool endswith(const char* str, const char* tail) {
    const char* s = str;
    const char* t = tail;
    // Find end of strings
    while (*s) s++;
    while (*t) t++;
    // Compare from end
    while (t > tail && s > str) {
        s--; t--;
        if (*s != *t) return false;
    }
    return t == tail;
}

} // namespace rrr
```

### Test from Rust

```rust
extern "C" {
    #[link_name = "_ZN3rrr10startswithEPKcS1_"]
    fn startswith(str: *const i8, head: *const i8) -> bool;
}

fn main() {
    let str = "hello world\0";
    let head = "hello\0";
    let result = unsafe {
        startswith(str.as_ptr() as *const i8, head.as_ptr() as *const i8)
    };
    assert!(result);
}
```

## Implementation Steps

### Step 1: Create mako_simple.cpp (~20 LOC)
- Isolated startswith/endswith functions
- No STL dependencies

### Step 2: Add test_basic_mako_ops test (~50 LOC)
- Parse mako_simple.cpp
- Compile to object
- Link with Rust test
- Run and verify output

### Step 3: Verify with actual mako file (optional)
- Try compiling strop.cpp with full dependencies
- This may require additional STL header handling

## Estimated LOC

- mako_simple.cpp: ~20 LOC
- Test code: ~50 LOC
- **Total: ~70 LOC**

## Success Criteria

1. Rust code successfully calls mako's startswith function
2. Function returns correct result
3. Test passes on CI

## Future Work (M6+)

- Full mako file compilation with STL
- Complex mako data structures
- Mako benchmark operations
