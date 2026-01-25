# Plan: Task 23.11.2 - Test Small Projects (1K-5K LOC)

## Current Blockers

Before testing actual 1K-5K LOC projects, several features need to work:

1. **iostream** (Phase 5) - BLOCKED
   - Many small projects use std::cout for output
   - Static initialization of cout/cin/cerr objects not working

2. **Threading** (Phase 6) - BLOCKED
   - Some projects use std::thread
   - pthread mapping exists but full libc++ thread support incomplete

## Strategy

Since iostream and threading are blocked, focus on projects that:
1. Don't use std::cout/std::cin (use return codes or custom I/O)
2. Don't use std::thread (single-threaded algorithms)
3. Don't require complex STL (beyond vector basics)

## Candidate Projects

### Option 1: Algorithm-focused code
- Sorting algorithms (quicksort, mergesort, heapsort)
- Data structures (binary tree, hash table, graph)
- Numeric algorithms (matrix operations, FFT)

### Option 2: Self-contained utilities
- String manipulation utilities
- Argument parsing (without streams)
- Simple calculators

### Option 3: Header-only libraries
- stb_image (C-style, minimal C++)
- picojson (JSON parsing, no iostream dependency if configured)

## Recommended Approach

1. Start with algorithm tests that use only:
   - Classes with methods
   - Pointers and memory management
   - Control flow
   - Return codes (no cout)

2. Create several 100-300 LOC tests that together exercise the range of features

## Test Cases to Add

### Test 1: Binary Search Tree (already have linked list, this adds tree traversal)
- Insert, search, delete operations
- In-order traversal
- Memory cleanup in destructor

### Test 2: Simple Stack/Queue
- Push/pop operations
- Using array-based storage
- Copy/move semantics

### Test 3: Matrix Operations
- 2D array access
- Operator overloading (+, *, ==)
- Static factory methods

## LOC Estimate

Each test case: ~100-150 LOC
Total: ~300-450 LOC for 3 comprehensive tests
