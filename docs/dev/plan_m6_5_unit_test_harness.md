# Plan: M6.5 - Unit Test Harness

## Overview

M6.5 creates a minimal unit test harness inspired by mako's unittest framework. This demonstrates that the compilation pipeline can handle:
- Class inheritance with virtual functions
- Singleton patterns
- std::vector with class pointers
- Macros for test definition

## Analysis

### Mako's unittest.cpp

The original unittest has:
1. `TestCase` - Base class with virtual `run()`
2. `TestMgr` - Singleton managing test cases
3. `TEST()` macro - Creates TestCase subclasses
4. `EXPECT_*` macros - Assertion helpers

### Complexity

The full framework requires:
- `strsplit()` function (STL string operations)
- `Log::info/error` (logging framework)
- Complex command-line parsing
- Raw pointer management with new/delete

### Simplified Approach

Create a minimal version that:
1. Defines a simple TestCase base class
2. Has a basic TestMgr that just runs all tests
3. Uses simple EXPECT macros that print results
4. Can be extended later

## Implementation Plan (~200 LOC)

### Step 1: unittest_minimal.cpp (~80 LOC)

```cpp
// unittest_minimal.cpp - Minimal unit test harness
#include <cstdio>
#include <vector>

namespace test {

class TestCase {
    const char* name_;
    int failures_;
public:
    TestCase(const char* name) : name_(name), failures_(0) {}
    virtual ~TestCase() {}
    virtual void run() = 0;
    const char* name() const { return name_; }
    void fail() { failures_++; }
    int failures() const { return failures_; }
};

class TestMgr {
    std::vector<TestCase*> tests_;
public:
    void reg(TestCase* t) { tests_.push_back(t); }
    int run_all();
    static TestMgr& instance();
};

} // namespace test

// C wrappers for Rust
extern "C" {
    void test_register(test::TestCase* t);
    int test_run_all();
}
```

### Step 2: Test file using the harness (~50 LOC)

```cpp
// test_strop.cpp - Tests for strop functions using minimal harness
#include "unittest_minimal.hpp"
#include <cstring>

namespace rrr {
extern bool startswith(const char*, const char*);
extern bool endswith(const char*, const char*);
}

class TestStartswith : public test::TestCase {
public:
    TestStartswith() : TestCase("startswith") {}
    void run() override {
        if (!rrr::startswith("hello", "hel")) fail();
        if (rrr::startswith("hello", "world")) fail();
    }
};

// Register tests
static TestStartswith test_startswith;
```

### Step 3: Rust test that runs the harness (~70 LOC)

```rust
extern "C" {
    fn test_run_all() -> i32;
}

fn main() {
    let failures = unsafe { test_run_all() };
    assert_eq!(failures, 0, "Some tests failed");
    println!("All tests passed!");
}
```

## Success Criteria

1. unittest_minimal.cpp compiles with STL
2. Test cases register and run
3. Pass/fail is reported correctly
4. Can be extended to run more tests

## Estimated Effort

- unittest_minimal.cpp: ~80 LOC
- test_strop.cpp: ~50 LOC
- Rust test: ~70 LOC
- **Total: ~200 LOC**
