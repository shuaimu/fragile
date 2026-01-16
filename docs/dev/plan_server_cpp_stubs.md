# Stub Headers for server.cpp Parsing

## Overview
This document describes the stub header additions made to successfully parse `vendor/mako/src/rrr/rpc/server.cpp`.

## Result
**SUCCESS**: server.cpp now parses with 4667 functions extracted.

## Issues Addressed

### Missing Headers Created
1. `fstream` - std::ofstream for logging
2. `array` - std::array for fixed-size containers
3. `future` - std::future, std::shared_future, std::future_status for async operations
4. `concepts` - std::invocable and other C++20 concepts
5. `cstdarg` - va_list and variadic argument macros
6. `stdarg.h` - C wrapper for cstdarg
7. `stdio.h` - C wrapper for cstdio

### Type Traits Extensions
- Added `is_arithmetic_v` variable template
- Added `bool_constant` template alias

### Mutex Extensions
- Added `once_flag` and `call_once` for one-time initialization
- Added `unique_lock` constructors for `defer_lock_t`, `adopt_lock_t`, `try_to_lock_t`
- Added `scoped_lock` (C++17)

### Socket Constants
- Added `SOMAXCONN` to sys/socket.h

### C String Ambiguity Fix
Fixed duplicate declarations by having C headers (string.h, stdio.h) simply include their C++ counterparts:
- `string.h` now includes `cstring`
- `stdio.h` now includes `cstdio`

### Chrono Extensions
- Added duration comparison operators
- Added time_point comparison operators
- Added time_point arithmetic with duration

### Functional Extensions
- Added `std::invoke` (C++17)
- Added `std::mem_fn`
- Added `std::bind` (simplified)
- Added `std::not_fn` (C++17)
- Added `std::identity` (C++20)

### Dependency Fixes
- `cstddef` now includes `cstdlib` (for std::abort)
- `cstdio` now includes `cstdarg` (for va_list)

## Design Rationale

### Why Stub Headers Instead of Real Headers?
The rusty-cpp library and Mako code depend on various C++ standard library features. Rather than trying to use system GCC/libc++ headers (which have compatibility issues with libclang), we use minimal stub headers that provide just enough to parse the code.

### C Header Strategy
To avoid ambiguous calls between C and C++ namespaces (e.g., `memset` vs `std::memset`), C headers like `string.h` simply include their C++ counterparts (`cstring`). This ensures all functions are declared once in the std namespace and then brought to the global namespace via `using`.

## Files Modified/Created

### Created
- `stubs/fstream`
- `stubs/array`
- `stubs/future`
- `stubs/concepts`
- `stubs/cstdarg`
- `stubs/stdarg.h`
- `stubs/stdio.h`

### Modified
- `stubs/type_traits` - added `is_arithmetic_v`, `bool_constant`
- `stubs/mutex` - added `once_flag`, `call_once`, `unique_lock` constructors, `scoped_lock`
- `stubs/sys/socket.h` - added `SOMAXCONN`
- `stubs/chrono` - added comparison operators
- `stubs/functional` - added `invoke`, `mem_fn`, `bind`, `not_fn`, `identity`
- `stubs/cstddef` - added `cstdlib` include
- `stubs/cstdio` - added `cstdarg` include
- `stubs/string.h` - simplified to include `cstring`
