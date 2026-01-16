# Plan: M6 - Mako Tests Pass

## Overview

M6 represents the final milestone: running actual Mako tests through the Fragile compilation pipeline. This is a large undertaking that needs to be broken into smaller pieces.

## Prerequisites

- [x] M5.8: Basic mako operations work (startswith, endswith, add_int)
- [x] C++ → object file compilation working
- [x] Rust + C++ linking working
- [x] 624 tests passing

## Challenge Analysis

Running Mako tests requires:
1. Compiling all Mako C++ code (338+ files)
2. External dependencies (RocksDB, eRPC, protobuf, etc.)
3. Complex STL usage (actual std::string, std::vector, etc.)
4. Proper name mangling for all symbols
5. C++ runtime (libstdc++/libc++)

## Incremental Approach

Break M6 into smaller milestones:

### M6.1: Extended mako_simple.cpp (~50 LOC)
- Add more utility functions without STL
- Test integer operations (min, max, clamp)
- Test pointer operations
- Estimated: <100 LOC, 1 task

### M6.2: String utilities without STL (~100 LOC)
- Implement strcmp, strncpy, strlen equivalents
- Test string manipulation
- No std::string dependency yet

### M6.3: First real mako file - strop.cpp functions (~150 LOC)
- Try to compile actual strop.cpp
- May need to stub strlen/strncmp
- Link with C runtime

### M6.4: Simple mako test executable (~200 LOC)
- Create minimal test that uses mako functions
- Tests rrr::startswith/endswith with real implementation
- Links with STL (libstdc++)

### M6.5: Unit test harness (~200 LOC)
- Port mako's unittest framework
- Run simple assertions
- Report pass/fail

### M6.6+: Full test suite
- Requires external dependencies
- Out of scope for immediate work

## Recommended Next Step: M6.1

M6.1 is the smallest step: extend mako_simple.cpp with more functions.

### Functions to Add

```cpp
namespace rrr {
    // Integer utilities
    int min_int(int a, int b);
    int max_int(int a, int b);
    int clamp_int(int value, int min, int max);

    // Pointer utilities
    bool is_null(const void* ptr);

    // String length (no strlen dependency)
    int str_len(const char* str);
}
```

### Test Coverage

```rust
// Test all new functions
assert_eq!(min_int(1, 2), 1);
assert_eq!(max_int(1, 2), 2);
assert_eq!(clamp_int(5, 0, 10), 5);
assert_eq!(clamp_int(-5, 0, 10), 0);
assert_eq!(is_null(std::ptr::null()), true);
assert_eq!(str_len("hello"), 5);
```

## Implementation Status

### M6.1 - Extended mako_simple.cpp [COMPLETED]

Added to mako_simple.cpp:
- `min_int(int, int)` - integer minimum
- `max_int(int, int)` - integer maximum
- `clamp_int(int, int, int)` - clamp to range
- `is_null(const void*)` - null pointer check
- `str_len(const char*)` - string length

Test coverage added:
- 5 new extern "C" declarations with proper name mangling
- 15+ new assertions testing all functions
- Edge cases: empty strings, boundary values, null pointers

## Estimated LOC for M6.1

- mako_simple.cpp additions: ~35 LOC
- Test additions: ~60 LOC
- **Total: ~95 LOC**

## Success Criteria for M6.1

1. All new functions compile and link
2. Tests pass
3. No STL dependencies
4. Builds on CI
