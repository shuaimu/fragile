# Plan: F.1 Mako Integration - rand.cpp

## Overview

Attempt to compile the first Mako file: `vendor/mako/src/rrr/misc/rand.cpp`

## Status: BLOCKED

### Blocking Issue
GCC libstdc++ headers (version 12) don't parse correctly with libclang. Specific errors:
- `type_traits` line 755: anonymous struct definition issues
- `wchar.h` attribute incompatibilities
- Cascading failures cause `uint64_t`, `std::mt19937` etc. to not be recognized

### Resolution Options
1. **Install libc++**: Use LLVM's C++ standard library instead of GCC's libstdc++
2. **Create stub headers**: Minimal STL type definitions for parsing
3. **Use compatible toolchain**: Match GCC/Clang versions that work together

## Work Completed [26:01:16]

### Submodule Initialization
- Initialized `vendor/mako/third-party/rusty-cpp` (Rust-like C++ wrappers)
- Initialized other submodules: yaml-cpp, erpc, makocon

### Parser Improvements
1. Better error messages with file/line/column information
2. `KeepGoing` mode to continue past errors
3. System header error filtering (only fail on user code errors)
4. Removed error limit (`-ferror-limit=0`)
5. Template depth increase (`-ftemplate-depth=1024`)

### Tests Added
- `test_mako_rand_patterns`: Tests rand.cpp patterns without external dependencies
- `test_mako_rand_cpp_actual`: Attempts to parse actual file (documents system header issues)

## File Analysis

### Dependencies
- `<string>` - STL
- `<vector>` - STL
- `"base/all.hpp"` - Mako base utilities
- `"rand.hpp"` - Header for RandomGenerator class
- `<rusty/box.hpp>` etc. - From rusty-cpp submodule

### Features Used
1. **C++ Classes**: `class RandomGenerator`
2. **Static members**: `static pthread_key_t seed_key_`
3. **pthread**: `pthread_key_create`, `pthread_once`, etc.
4. **Inline assembly**: `__asm__ __volatile__("rdtsc")`
5. **thread_local**: `thread_local unsigned int seed_`
6. **Preprocessor conditionals**: `#if defined(__APPLE__)`
7. **STL**: `std::string`, `std::vector`
8. **Namespaces**: `namespace rrr`

## Pattern Test Coverage

The `test_mako_rand_patterns` test covers all key patterns from rand.cpp:
- Static class members
- thread_local storage
- Method overloading
- STL usage (std::string, std::vector, std::to_string)
- Namespace organization

## Next Steps

1. **Option A**: Install libc++-19-dev and configure parser to use it
2. **Option B**: Create minimal stub headers for required types
3. **Continue with other tasks**: The pattern test provides good coverage
