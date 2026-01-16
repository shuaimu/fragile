// Minimal asm-generic/mman.h stub for fragile parsing
#ifndef _ASM_GENERIC_MMAN_H
#define _ASM_GENERIC_MMAN_H

// Memory protection flags
#define PROT_READ       0x1
#define PROT_WRITE      0x2
#define PROT_EXEC       0x4
#define PROT_NONE       0x0

// mmap flags
#define MAP_SHARED      0x01
#define MAP_PRIVATE     0x02
#define MAP_FIXED       0x10
#define MAP_ANONYMOUS   0x20
#define MAP_ANON        MAP_ANONYMOUS
#define MAP_GROWSDOWN   0x0100
#define MAP_LOCKED      0x2000
#define MAP_NORESERVE   0x4000
#define MAP_POPULATE    0x8000
#define MAP_NONBLOCK    0x10000
#define MAP_STACK       0x20000
#define MAP_HUGETLB     0x40000

// MAP_FAILED return value
#define MAP_FAILED      ((void*)-1)

// madvise flags
#define MADV_NORMAL     0
#define MADV_RANDOM     1
#define MADV_SEQUENTIAL 2
#define MADV_WILLNEED   3
#define MADV_DONTNEED   4
#define MADV_HUGEPAGE   14
#define MADV_NOHUGEPAGE 15

// msync flags
#define MS_ASYNC        1
#define MS_INVALIDATE   2
#define MS_SYNC         4

// mlock flags
#define MCL_CURRENT     1
#define MCL_FUTURE      2
#define MCL_ONFAULT     4

#endif // _ASM_GENERIC_MMAN_H
