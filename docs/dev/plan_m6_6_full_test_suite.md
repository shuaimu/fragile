# Plan: M6.6 - Full Test Suite (Analysis)

## Overview

M6.6+ aims to run actual mako unit tests through the Fragile compilation pipeline. This is a large undertaking that needs to be broken into smaller milestones.

## Analysis

### Current State

We have demonstrated:
- M6.1-M6.2: Basic functions without STL
- M6.3: C library functions (strlen, strncmp)
- M6.4: STL functions (std::string, std::ostringstream)
- M6.5: Unit test harness (virtual functions, std::vector)

### Remaining Challenges

To run actual mako tests, we need:

1. **External Dependencies**
   - RocksDB (key-value store)
   - eRPC (high-performance RPC)
   - protobuf (serialization)
   - boost (various utilities)

2. **Complex STL Operations**
   - std::thread
   - std::mutex, std::condition_variable
   - std::shared_ptr, std::unique_ptr
   - std::unordered_map, std::unordered_set

3. **Threading/Concurrency**
   - pthread integration
   - Atomic operations
   - Memory ordering

4. **I/O Operations**
   - File I/O
   - Network I/O
   - Logging framework

### Proposed Sub-milestones

Since the full test suite requires external dependencies that may not be available, focus on:

#### M6.6a: Self-contained tests (~200 LOC)
- Find mako tests that don't require external dependencies
- Port them to use unittest_minimal.cpp
- Run through Fragile pipeline

#### M6.6b: strop tests (~150 LOC)
- Port the actual strop.cpp tests from mako
- Test startswith, endswith, format_decimal, strsplit

#### M6.6c: Logging framework (~200 LOC)
- Port basic rrr::Log functionality
- Needed for most mako tests

#### M6.6d: Basic threading (~300 LOC)
- std::thread wrapper
- std::mutex, std::lock_guard
- Simple thread pool

### Recommended Next Step: M6.6a

Find and port self-contained tests that:
1. Don't require external dependencies
2. Don't require threading
3. Test basic functionality

Good candidates:
- strop tests (startswith, endswith)
- base/misc utility tests
- Simple data structure tests

## Estimated Effort

Each sub-milestone: ~200-300 LOC
Total for M6.6: ~1000+ LOC (multiple sessions)

## Decision Point

At this stage, M6.6+ may require:
1. Installing external dependencies (RocksDB, eRPC, etc.)
2. Creating stubs for external APIs
3. Or accepting that full tests require infrastructure work

For now, recommend focusing on self-contained tests that validate the compilation pipeline works for increasingly complex C++ code.
