#ifndef _MALLOC_H
#define _MALLOC_H

// Minimal malloc.h stub for fragile-clang
// Provides memory allocation functions beyond cstdlib

#include <cstddef>
#include <cstdlib>  // Get basic malloc/free/realloc/calloc from cstdlib to avoid redeclaration

#ifdef __cplusplus
extern "C" {
#endif

// Basic allocation is already provided by cstdlib
// (malloc, free, realloc, calloc)

// Extended allocation functions
void* memalign(size_t alignment, size_t size);
// posix_memalign is provided by cstdlib to avoid conflicts
void* valloc(size_t size);       // Page-aligned allocation
void* pvalloc(size_t size);      // Page-aligned, rounded up
void* aligned_alloc(size_t alignment, size_t size);

// Query functions
size_t malloc_usable_size(void* ptr);

// Heap information structure
struct mallinfo {
    int arena;     // Non-mmapped space allocated (bytes)
    int ordblks;   // Number of free chunks
    int smblks;    // Number of free fastbin blocks
    int hblks;     // Number of mmapped regions
    int hblkhd;    // Space allocated in mmapped regions (bytes)
    int usmblks;   // Maximum total allocated space (bytes)
    int fsmblks;   // Space in freed fastbin blocks (bytes)
    int uordblks;  // Total allocated space (bytes)
    int fordblks;  // Total free space (bytes)
    int keepcost;  // Top-most, releasable space (bytes)
};

// mallinfo2 for 64-bit support (glibc 2.33+)
struct mallinfo2 {
    size_t arena;
    size_t ordblks;
    size_t smblks;
    size_t hblks;
    size_t hblkhd;
    size_t usmblks;
    size_t fsmblks;
    size_t uordblks;
    size_t fordblks;
    size_t keepcost;
};

struct mallinfo mallinfo(void);
struct mallinfo2 mallinfo2(void);

// Tuning
int mallopt(int param, int value);

// mallopt parameters
#define M_MXFAST        1
#define M_NLBLKS        2
#define M_GRAIN         3
#define M_KEEP          4
#define M_TRIM_THRESHOLD -1
#define M_TOP_PAD       -2
#define M_MMAP_THRESHOLD -3
#define M_MMAP_MAX      -4
#define M_CHECK_ACTION  -5
#define M_PERTURB       -6
#define M_ARENA_TEST    -7
#define M_ARENA_MAX     -8

// Debugging hooks (deprecated in modern glibc but still used)
typedef void* (*__malloc_hook_t)(size_t, const void*);
typedef void* (*__realloc_hook_t)(void*, size_t, const void*);
typedef void (*__free_hook_t)(void*, const void*);
typedef void* (*__memalign_hook_t)(size_t, size_t, const void*);

extern __malloc_hook_t __malloc_hook;
extern __realloc_hook_t __realloc_hook;
extern __free_hook_t __free_hook;
extern __memalign_hook_t __memalign_hook;

// Memory trimming
int malloc_trim(size_t pad);

// Statistics
void malloc_stats(void);

// Memory info
int malloc_info(int options, void* stream);

#ifdef __cplusplus
}
#endif

#endif // _MALLOC_H
