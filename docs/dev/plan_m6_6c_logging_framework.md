# Plan: M6.6c Logging Framework Test

## Overview

M6.6c adds a simplified logging framework test inspired by mako's `rrr::Log` class. This demonstrates:
- Variadic functions with `va_list`/`va_start`/`va_end`
- `pthread_mutex_t` for thread safety
- `FILE*` and output stream handling
- Log levels (FATAL, ERROR, WARN, INFO, DEBUG)
- `sprintf`/`vsprintf` for formatting

## Files to Create

### 1. `tests/clang_integration/test_logging_harness.cpp`

A self-contained logging test that:
- Defines a minimal `Log` class with static members
- Uses pthread mutex for thread safety (static initialization with `PTHREAD_MUTEX_INITIALIZER`)
- Implements variadic log functions
- Has 5 test cases covering:
  - Basic log levels
  - Level filtering
  - Format strings with arguments
  - Output capture/verification
  - Edge cases (empty format, null handling)

### 2. Update `crates/fragile-rustc-driver/src/driver.rs`

Add `test_logging_harness` integration test following the pattern of M6.6a/M6.6b.

## Implementation Details

### Key C++ Features Used

```cpp
// Variadic functions
void log(int level, const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    // ... use args
    va_end(args);
}

// pthread mutex (static init)
static pthread_mutex_t mutex_ = PTHREAD_MUTEX_INITIALIZER;

// Thread-safe operations
pthread_mutex_lock(&mutex_);
// ... critical section
pthread_mutex_unlock(&mutex_);
```

### Test Cases

1. **TestLogBasicLevels**: Verify all 5 log levels work
2. **TestLogFiltering**: Test that level filtering works correctly
3. **TestLogFormat**: Test format strings with %d, %s, %f arguments
4. **TestLogOutput**: Capture and verify log output to buffer
5. **TestLogEdgeCases**: Empty strings, boundary conditions

### C-compatible Wrappers

```cpp
extern "C" {
    int logging_test_run_all();
    int logging_test_count();
}
```

## Size Estimate

- `test_logging_harness.cpp`: ~350 lines
- Rust test addition: ~30 lines

Total: ~380 lines (within 500 LOC limit)

## Dependencies

- `<cstdio>` - FILE*, sprintf, vsprintf
- `<cstdarg>` - va_list, va_start, va_end
- `<pthread.h>` - pthread_mutex_t
- `<cstring>` - strlen, strcmp
- `<vector>` - test registration

## Success Criteria

1. `test_logging_harness.cpp` compiles with clang
2. Rust integration test passes
3. All 5 logging test cases pass
4. No regression in existing tests
