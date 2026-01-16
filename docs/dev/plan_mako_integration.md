# Plan: F.1 Mako Integration - rand.cpp

## Overview

Attempt to compile the first Mako file: `vendor/mako/src/rrr/misc/rand.cpp`

## File Analysis

### Dependencies
- `<string>` - STL
- `<vector>` - STL
- `"base/all.hpp"` - Mako base utilities
- `"rand.hpp"` - Header for RandomGenerator class

### Features Used
1. **C++ Classes**: `class RandomGenerator`
2. **Static members**: `static pthread_key_t seed_key_`
3. **pthread**: `pthread_key_create`, `pthread_once`, etc.
4. **Inline assembly**: `__asm__ __volatile__("rdtsc")`
5. **thread_local**: `thread_local unsigned int seed_`
6. **Preprocessor conditionals**: `#if defined(__APPLE__)`
7. **STL**: `std::string`, `std::vector`
8. **Namespaces**: `namespace rrr`

## Test Plan

1. Try to parse rand.cpp with our ClangParser
2. Document what features are missing/failing
3. Prioritize fixes based on frequency of use

## Expected Issues

1. Include path resolution for "base/all.hpp"
2. pthread types and functions
3. Inline assembly handling
4. thread_local storage class

## Test Code

```rust
#[test]
fn test_mako_rand_cpp() {
    let parser = ClangParser::with_system_includes()
        .with_include_path("vendor/mako/src/rrr")
        .unwrap();

    let content = std::fs::read_to_string("vendor/mako/src/rrr/misc/rand.cpp")
        .unwrap();

    let result = parser.parse_string(&content, "rand.cpp");
    // Document what works and what doesn't
}
```

## Next Steps

Based on parsing results, identify which features need implementation:
- Missing AST nodes
- Missing type support
- Missing expression handling
