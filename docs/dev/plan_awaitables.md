# Plan: D.6 Awaitables

## Overview

Add comprehensive test coverage for C++20 awaitable types and the co_await expression protocol. Awaitables are types that can be used with co_await, implementing the await_ready/await_suspend/await_resume protocol.

## Awaitable Protocol

An awaitable type must provide:
1. `await_ready()` - Returns bool; if true, skips suspension
2. `await_suspend(handle)` - Called when coroutine suspends; controls resumption
3. `await_resume()` - Called when coroutine resumes; returns the co_await result

Standard awaitables:
- `std::suspend_always` - always suspends (await_ready returns false)
- `std::suspend_never` - never suspends (await_ready returns true)

## Design

### 1. Test Cases to Add

Since awaitable types use the same infrastructure as other C++ classes/structs, we need tests that verify:

1. **await_ready test**: Verify method returning bool is parsed
2. **await_suspend test**: Verify method taking coroutine_handle is parsed
3. **await_resume test**: Verify method returning value/void is parsed
4. **Custom awaitable test**: Full custom awaitable implementation
5. **co_await with custom awaitable**: Verify co_await expression works with custom awaitables
6. **Awaitable transformation**: Test await_transform in promise_type

### 2. Implementation Notes

No new AST nodes needed - awaitable methods are just regular CXXMethodDecl nodes.
The co_await expression parsing is already done (CoawaitExpr).

Key things to verify:
- Method signatures match awaitable protocol
- Return types are correctly parsed
- coroutine_handle parameter is handled

## Test Plan

```cpp
#include <coroutine>

// Test 1: Custom awaitable with all protocol methods
struct CustomAwaitable {
    bool await_ready() { return false; }
    void await_suspend(std::coroutine_handle<> h) { h.resume(); }
    int await_resume() { return 42; }
};

// Test 2: Awaitable that immediately resumes
struct ImmediateAwaitable {
    bool await_ready() { return true; }
    void await_suspend(std::coroutine_handle<>) {}
    void await_resume() {}
};

// Test 3: Awaitable with different suspend return types
struct ConditionalAwaitable {
    bool await_ready() { return false; }
    bool await_suspend(std::coroutine_handle<> h) {
        return true; // return false to resume immediately
    }
    int await_resume() { return 0; }
};

// Test 4: Using custom awaitable in coroutine
struct Task {
    struct promise_type {
        Task get_return_object() { return {}; }
        std::suspend_never initial_suspend() { return {}; }
        std::suspend_never final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() {}
    };
};

Task test_custom_await() {
    CustomAwaitable awaitable;
    int result = co_await awaitable;
}
```

## Estimated LOC

- Tests: ~100 lines (6 test cases)
- No production code changes needed
- Total: ~100 lines
