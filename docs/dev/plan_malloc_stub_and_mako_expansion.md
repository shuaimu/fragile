# Plan: Add malloc.h Stub and Expand Mako Module Parsing

## Goal
Expand mako module parsing from 11.6% (18/155 files) by adding missing stub headers.

## Current Blockers
1. `malloc.h` - Required by masstree files, txn.h, benchmarks
2. Some files need `jemalloc/jemalloc.h` - used in bench.cc, multiversion.hh

## Task 1: Create malloc.h Stub

The `<malloc.h>` header provides memory allocation functions beyond what's in `<cstdlib>`.

Key functions needed:
- `malloc()`, `free()` - already in cstdlib
- `memalign()` - aligned allocation
- `posix_memalign()` - POSIX aligned allocation
- `valloc()` - page-aligned allocation
- `mallinfo()` - heap information structure
- `mallopt()` - tuning options
- `malloc_usable_size()` - allocated size query

## Task 2: Test Low-Dependency Mako Files

Files identified as lowest complexity to try:
1. `ticker.cc` - 4 lines, only static member init
2. `core.cc` - core ID management, depends on silo_runtime.h
3. `thread.cc` - thread wrapper
4. `silo_runtime.cc` - runtime management (depends on numa.h, sys/mman.h - already have)

## Implementation

### malloc.h stub (~50 lines)
```cpp
#ifndef _MALLOC_H
#define _MALLOC_H

#include <cstddef>

extern "C" {
    void* malloc(size_t size);
    void free(void* ptr);
    void* realloc(void* ptr, size_t size);
    void* calloc(size_t nmemb, size_t size);

    // Extended functions
    void* memalign(size_t alignment, size_t size);
    int posix_memalign(void** memptr, size_t alignment, size_t size);
    void* valloc(size_t size);
    void* pvalloc(size_t size);
    void* aligned_alloc(size_t alignment, size_t size);
    size_t malloc_usable_size(void* ptr);

    // mallinfo structure
    struct mallinfo {
        int arena;
        int ordblks;
        int smblks;
        int hblks;
        int hblkhd;
        int usmblks;
        int fsmblks;
        int uordblks;
        int fordblks;
        int keepcost;
    };

    struct mallinfo mallinfo(void);
    int mallopt(int param, int value);
}

#endif // _MALLOC_H
```

## Success Criteria
- malloc.h stub compiles without errors
- At least one additional mako file parses successfully
- All existing 226 tests continue to pass

## Timeline
- Task 1: ~10 minutes
- Task 2: ~20 minutes
- Testing: ~15 minutes
