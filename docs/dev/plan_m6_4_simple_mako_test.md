# Plan: M6.4 - Simple Mako Test Executable with STL

## Overview

M6.4 creates a test executable that links with libstdc++ to use STL-dependent functions from mako. This validates that the compilation pipeline can handle real C++ code with STL dependencies.

## Analysis

### STL Functions in strop.cpp

The following functions use STL:
1. `format_decimal(double)` - uses std::ostringstream, std::string
2. `format_decimal(int)` - uses std::ostringstream, std::string
3. `strsplit(const std::string&, char)` - uses std::string, std::vector

### Challenge

Linking with STL requires:
1. `-lstdc++` linker flag
2. Proper C++ ABI compatibility
3. Name mangling for STL templates

### Approach

Start with simpler STL usage:
1. Create a C++ file that uses basic std::string operations
2. Test with format_decimal (simpler than strsplit)
3. Progress to strsplit if format_decimal works

## Implementation Plan (~200 LOC)

### Step 1: Create strop_stl.cpp (~50 LOC)

A subset of strop.cpp with STL functions:

```cpp
// strop_stl.cpp - STL-dependent functions from mako strop.cpp
#include <sstream>
#include <string>
#include <iomanip>

namespace rrr {

std::string format_decimal(double val) {
    std::ostringstream o;
    o.precision(2);
    o << std::fixed << val;
    // ... rest of implementation
    return o.str();
}

std::string format_decimal(int val) {
    std::ostringstream o;
    o << val;
    // ... formatting logic
    return o.str();
}

} // namespace rrr
```

### Step 2: Create test that calls format_decimal (~100 LOC)

Test from Rust that:
1. Compiles strop_stl.cpp with libstdc++
2. Calls format_decimal from Rust
3. Verifies the output

### Challenge: C++ string return type

The main challenge is that these functions return `std::string`, not `const char*`.
Options:
1. Modify to return `const char*` (requires static buffer - not thread safe)
2. Add wrapper functions that take output buffer
3. Use C++ wrapper that manages string lifetime

### Alternative: Simpler First Step

Instead of full format_decimal, start with:
1. A simpler function that uses std::string internally but returns simple types
2. Or a wrapper function that takes a buffer

### Revised Approach: Use C Wrappers

Create C wrappers that expose STL functionality:

```cpp
// strop_stl.cpp
#include <string>
#include <cstring>

namespace rrr {

// Internal implementation using std::string
static std::string format_decimal_impl(int val) {
    // ... STL implementation
}

// C-compatible wrapper
extern "C" {
    int format_decimal_to_buf(int val, char* buf, int buf_size) {
        std::string result = format_decimal_impl(val);
        if (result.size() >= buf_size) {
            return -1; // buffer too small
        }
        std::strcpy(buf, result.c_str());
        return result.size();
    }
}

} // namespace rrr
```

This allows Rust to call C functions while the C++ code uses STL internally.

## Success Criteria

1. strop_stl.cpp compiles with libstdc++
2. Rust can call the wrapper functions
3. format_decimal output matches expected format
4. All existing tests pass

## Estimated Effort

- strop_stl.cpp: ~50 LOC
- Rust test: ~100 LOC
- Plan documentation: ~50 LOC
- **Total: ~200 LOC**
