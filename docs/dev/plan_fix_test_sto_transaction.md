# Plan: Fix test_sto_transaction Blocker

## Problem

The `test_sto_transaction` test was blocked because the `compiler.hh` header from Masstree/STO redefines `static_assert` and `constexpr` when the feature detection macros are not set:

```cpp
#if !HAVE_CXX_STATIC_ASSERT
#define static_assert(x, msg) switch (x) case 0: case !!(x):
#endif

#if !HAVE_CXX_CONSTEXPR
#define constexpr const
#endif
```

This breaks C++20/23 compilation because:
1. `static_assert` is a keyword in C++11+
2. `constexpr` is a keyword in C++11+

## Root Cause

The Masstree code was originally written for C++03 compatibility and uses autoconf-style feature detection macros (`HAVE_CXX_*`). These macros are typically set by running `./configure` which generates `config.h`.

In the Fragile build system, we don't use autoconf - we compile directly with Clang and C++20/23 standards. The feature macros were never being defined.

## Solution

Added the required C++ feature detection macros to the global `defines` section in `vendor/mako/fragile.toml`:

```toml
defines = [
    "CONFIG_H=\"mako/config/config-perf.h\"",
    "NDEBUG",
    # C++ feature macros for modern C++ (C++11/14/17/20)
    # These prevent compiler.hh from redefining static_assert/constexpr
    "HAVE_CXX_STATIC_ASSERT=1",
    "HAVE_CXX_CONSTEXPR=1",
    "HAVE_CXX_RVALUE_REFERENCES=1",
    "HAVE_CXX_TEMPLATE_ALIAS=1",
    "HAVE_TYPE_TRAITS=1",
    "HAVE_STD_IS_TRIVIALLY_COPYABLE=1",
    "HAVE_STD_IS_TRIVIALLY_DESTRUCTIBLE=1",
]
```

## Implementation

1. Added HAVE_CXX_* defines to fragile.toml global section
2. Enabled test_sto_transaction target in fragile.toml (was commented out)
3. Added TRcu.cc source file to the test sources (provides TRcuSet implementation)
4. Added src/mako/benchmarks/sto to include paths

## Test Results

```
[==========] Running 13 tests from 1 test suite.
[----------] 13 tests from STOTRcuTest
[ RUN      ] STOTRcuTest.Constructor_InitializesCorrectly
[       OK ] STOTRcuTest.Constructor_InitializesCorrectly (0 ms)
[ RUN      ] STOTRcuTest.MultipleInstances_Independent
[       OK ] STOTRcuTest.MultipleInstances_Independent (0 ms)
...
[  PASSED  ] 13 tests.
```

All 13 tests pass:
- Constructor_InitializesCorrectly
- MultipleInstances_Independent
- CleanUntil_ZeroEpoch
- CleanUntil_SequentialEpochs
- CleanUntil_NonSequential
- CleanUntil_LargeEpoch
- ConcurrentCleanup_MultipleThreads
- ConcurrentCleanup_NoRaceConditions
- Perf_1000CleanOperations
- EdgeCase_RepeatedSameEpoch
- EdgeCase_BackwardsEpochs
- Stress_RapidCleanupCycles
- BatchCleanup_LargeRange

## Impact

- Increases test coverage: 54 executables, 880+ gtest tests
- Enables testing of STO (Software Transactional Objects) TRcu (Transactional RCU) functionality
- No regression in existing tests

## Date

[26:01:17]
