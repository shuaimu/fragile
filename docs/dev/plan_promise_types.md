# Plan: D.5 Promise Types

## Overview

Add comprehensive test coverage for C++20 coroutine promise type methods. Promise types are the mechanism through which coroutines customize their behavior. While our parser already handles methods via CXXMethodDecl, we need to verify that promise type patterns are correctly parsed and represented.

## Promise Type Requirements

A C++20 coroutine promise type must provide:
1. `get_return_object()` - Creates the coroutine return object
2. `initial_suspend()` - Called at coroutine start, returns awaitable
3. `final_suspend() noexcept` - Called at coroutine end, returns awaitable
4. Either `return_void()` or `return_value(T)` - Handles co_return

Optional methods:
- `yield_value(T)` - Handles co_yield
- `unhandled_exception()` - Called on exception

## Design

### 1. Test Cases to Add

Since promise type methods are regular C++ methods inside a nested struct, our existing parser should handle them. We need tests that verify:

1. **get_return_object test**: Verify the method is parsed with correct return type
2. **initial_suspend test**: Verify returns awaitable type (suspend_always/never)
3. **final_suspend test**: Verify noexcept specifier is parsed
4. **return_void test**: Verify void return method is parsed
5. **return_value test**: Verify method with parameter is parsed
6. **yield_value test**: Verify yield method with parameter

### 2. Implementation Notes

No new AST nodes needed - promise type methods are just CXXMethodDecl nodes inside RecordDecl (the promise_type struct).

Key things to verify:
- Method names are correctly captured
- Return types are correctly parsed (especially suspend_always/suspend_never)
- noexcept specifier on final_suspend
- Parameters on return_value/yield_value

## Test Plan

```cpp
#include <coroutine>

// Test 1: Basic promise type with all required methods
struct BasicPromiseTest {
    struct promise_type {
        BasicPromiseTest get_return_object() { return {}; }
        std::suspend_never initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() {}
    };
};

// Test 2: Promise with return_value instead of return_void
struct ValuePromiseTest {
    struct promise_type {
        int result;
        ValuePromiseTest get_return_object() { return {}; }
        std::suspend_never initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }
        void return_value(int v) { result = v; }
        void unhandled_exception() {}
    };
};

// Test 3: Generator-style promise with yield_value
struct GeneratorPromiseTest {
    struct promise_type {
        int current;
        GeneratorPromiseTest get_return_object() { return {}; }
        std::suspend_always initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() {}
        std::suspend_always yield_value(int value) {
            current = value;
            return {};
        }
    };
};
```

## Estimated LOC

- Tests: ~100 lines (6 test cases)
- No production code changes needed
- Total: ~100 lines
