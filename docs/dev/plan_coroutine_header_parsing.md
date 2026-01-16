# Plan: D.4 Coroutine Header Parsing

## Analysis

The coroutine tests added in D.1 already successfully parse C++20 coroutine code with the `<coroutine>` header. The standard library types are being handled through the existing type system:

- `std::coroutine_handle<T>` - Parsed as a template specialization, stored as CppType::Named
- `std::suspend_always` / `std::suspend_never` - Parsed as class types

## What D.4 Should Add

D.4 should add explicit tests to verify these types are properly recognized and handled:

1. Test that `std::coroutine_handle<promise_type>` is correctly parsed
2. Test that `std::suspend_always` and `std::suspend_never` are correctly parsed
3. Test that coroutine_traits is properly handled

## Current Status

The infrastructure for parsing these types already exists. D.4 is primarily about:
1. Adding explicit tests for coroutine header types
2. Verifying the types are correctly represented in our type system

## Implementation

Since the parsing already works (as demonstrated by D.1 tests), D.4 is essentially about adding more comprehensive tests.

## Estimated LOC

- Tests: ~100 lines
