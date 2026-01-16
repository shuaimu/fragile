# Plan: D.7 Generators

## Overview

Add comprehensive test coverage for C++20 generator-style coroutines using co_yield. Generators produce a sequence of values lazily, suspending after each value.

## Generator Pattern

A generator requires:
1. Promise type with `yield_value()` method
2. Iterator interface to consume values
3. `co_yield expr` translates to `co_await promise.yield_value(expr)`

## Design

### 1. Test Cases to Add

1. **Basic co_yield test**: Single co_yield with integer
2. **Multiple co_yield test**: Sequence of yielded values
3. **co_yield with different types**: Various value types
4. **Generator with loop**: co_yield inside a loop
5. **Infinite generator pattern**: Generator without termination
6. **Generator with state**: Generator maintaining state between yields

### 2. Implementation Notes

No new AST nodes needed - co_yield is already parsed as CoyieldExpr.
Generator patterns use existing yield_value in promise_type.

Key things to verify:
- CoyieldExpr nodes are correctly created
- yield_value method is called correctly
- Value type flows through correctly

## Test Plan

```cpp
#include <coroutine>

// Basic generator structure
struct Generator {
    struct promise_type {
        int current_value;
        Generator get_return_object() { return {}; }
        std::suspend_always initial_suspend() { return {}; }
        std::suspend_always final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() {}
        std::suspend_always yield_value(int value) {
            current_value = value;
            return {};
        }
    };
};

// Test: Basic yield
Generator basic_yield() {
    co_yield 42;
}

// Test: Multiple yields
Generator multi_yield() {
    co_yield 1;
    co_yield 2;
    co_yield 3;
}

// Test: Yield in loop
Generator loop_yield(int n) {
    for (int i = 0; i < n; i++) {
        co_yield i;
    }
}
```

## Estimated LOC

- Tests: ~100 lines (6 test cases)
- No production code changes needed
- Total: ~100 lines
