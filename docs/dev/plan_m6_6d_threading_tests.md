# Plan: M6.6d Basic Threading Tests

## Overview

M6.6d adds basic threading tests using C++11 standard library threading primitives. This demonstrates:
- `std::thread` for thread creation and joining
- `std::mutex` for mutual exclusion
- `std::lock_guard` for RAII-based locking
- `std::atomic` for lock-free operations
- `std::condition_variable` for signaling (optional)

## Files to Create

### 1. `tests/clang_integration/test_threading_harness.cpp`

A self-contained threading test that:
- Creates and joins threads with std::thread
- Uses std::mutex and std::lock_guard for synchronization
- Tests std::atomic for counter increments
- Has 5 test cases covering:
  - Basic thread creation and join
  - Mutex-protected shared counter
  - Lock guard RAII
  - Atomic operations
  - Multiple threads with shared state

### 2. Update `crates/fragile-rustc-driver/src/driver.rs`

Add `test_threading_harness` integration test.

## Implementation Details

### Key C++ Features Used

```cpp
// Thread creation and join
std::thread t([]{ /* work */ });
t.join();

// Mutex for shared data
std::mutex mtx;
mtx.lock();
// ... critical section
mtx.unlock();

// Lock guard for RAII
{
    std::lock_guard<std::mutex> guard(mtx);
    // ... critical section
}  // Automatically unlocks

// Atomic operations
std::atomic<int> counter{0};
counter.fetch_add(1);
```

### Test Cases

1. **TestThreadBasic**: Create a thread, pass data, join
2. **TestMutexProtect**: Multiple threads increment protected counter
3. **TestLockGuard**: RAII-based locking with lock_guard
4. **TestAtomic**: Atomic counter incremented by multiple threads
5. **TestThreadLambda**: Threads with lambda captures

### C-compatible Wrappers

```cpp
extern "C" {
    int threading_test_run_all();
    int threading_test_count();
}
```

## Size Estimate

- `test_threading_harness.cpp`: ~350 lines
- Rust test addition: ~30 lines

Total: ~380 lines (within 500 LOC limit)

## Dependencies

- `<thread>` - std::thread
- `<mutex>` - std::mutex, std::lock_guard
- `<atomic>` - std::atomic
- `<vector>` - test registration and thread storage
- `<cstdio>` - printf for output

## Success Criteria

1. `test_threading_harness.cpp` compiles with clang
2. Rust integration test passes
3. All 5 threading test cases pass
4. No regression in existing tests
5. No race conditions (all tests are deterministic)
