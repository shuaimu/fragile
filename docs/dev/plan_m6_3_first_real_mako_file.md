# Plan: M6.3 - First Real Mako File (strop.cpp)

## Overview

M6.3 aims to compile the first real file from the Mako project: `strop.cpp`. This file contains string utility functions used throughout the Mako codebase.

## Analysis

### strop.cpp Contents

The file `/vendor/mako/src/rrr/base/strop.cpp` contains:

1. **C-string functions** (simple, only use strlen/strncmp):
   - `startswith(const char*, const char*)` - check if string starts with prefix
   - `endswith(const char*, const char*)` - check if string ends with suffix

2. **STL-dependent functions** (complex, require std::ostringstream):
   - `format_decimal(double)` - format double with commas
   - `format_decimal(int)` - format int with commas
   - `strsplit(const std::string&, char)` - split string by separator

### Challenge

The STL-dependent functions use:
- `std::ostringstream` for formatting
- `std::string` operations (substr, reserve, +=)
- `std::vector<std::string>` return type

These require linking with actual libstdc++, which is non-trivial.

## Approach

### Phase 1: C-string functions (this milestone)

Focus on `startswith` and `endswith` which only need:
- `strlen()` from `<string.h>`
- `strncmp()` from `<string.h>`

These can be linked with the C runtime (libc), which is always available.

### Phase 2: STL functions (future milestone)

The STL functions require:
1. Linking with libstdc++
2. Ensuring name mangling matches
3. Proper template instantiation

This is deferred to M6.4+.

## Implementation Plan (~150 LOC)

### Step 1: Create strop_minimal.cpp test file (~30 LOC)

Create a minimal version of strop.cpp that only includes the C-string functions:

```cpp
// strop_minimal.cpp - subset of rrr::strop functions for testing
#include <string.h>  // for strlen, strncmp

namespace rrr {

bool startswith(const char* str, const char* head) {
    size_t len_str = strlen(str);
    size_t len_head = strlen(head);
    if (len_head > len_str) {
        return false;
    }
    return strncmp(str, head, len_head) == 0;
}

bool endswith(const char* str, const char* tail) {
    size_t len_str = strlen(str);
    size_t len_tail = strlen(tail);
    if (len_tail > len_str) {
        return false;
    }
    return strncmp(str + (len_str - len_tail), tail, len_tail) == 0;
}

} // namespace rrr
```

### Step 2: Update rustc integration test (~80 LOC)

Add a test that:
1. Parses strop_minimal.cpp
2. Compiles it to an object file
3. Links with Rust code that calls the functions
4. Verifies the functions work correctly

### Step 3: Add test for parsing real strop.cpp (~40 LOC)

Add a parsing test (not execution) for the full strop.cpp to ensure:
1. Parser can handle the STL includes
2. All 5 functions are extracted
3. Name mangling is correct

## Success Criteria

1. strop_minimal.cpp compiles and links with Rust
2. startswith/endswith work correctly when called from Rust
3. Real strop.cpp parses without errors (execution deferred)
4. All existing tests pass

## Differences from mako_simple.cpp

The key difference is that strop_minimal.cpp uses:
- Actual C library functions (strlen, strncmp) instead of hand-rolled equivalents
- The exact same code structure as the real mako strop.cpp
- The same function signatures and mangled names

This validates that we can compile real mako code, not just test stubs.

## Estimated Effort

- strop_minimal.cpp: ~30 LOC
- Rust test for compilation/execution: ~80 LOC
- Parser test for full strop.cpp: ~40 LOC
- **Total: ~150 LOC**
