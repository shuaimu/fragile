// Minimal sys/mman.h stub for fragile parsing
#ifndef _FRAGILE_SYS_MMAN_H_
#define _FRAGILE_SYS_MMAN_H_

#include "../cstdint"
#include "../cstddef"

// mmap protection flags
#define PROT_NONE     0x0
#define PROT_READ     0x1
#define PROT_WRITE    0x2
#define PROT_EXEC     0x4

// mmap flags
#define MAP_SHARED    0x01
#define MAP_PRIVATE   0x02
#define MAP_FIXED     0x10
#define MAP_ANONYMOUS 0x20
#define MAP_ANON      MAP_ANONYMOUS
#define MAP_GROWSDOWN 0x0100
#define MAP_LOCKED    0x2000
#define MAP_NORESERVE 0x4000
#define MAP_POPULATE  0x8000
#define MAP_HUGETLB   0x40000

// mmap failure return value
#define MAP_FAILED    ((void*)-1)

// msync flags
#define MS_ASYNC      1
#define MS_SYNC       4
#define MS_INVALIDATE 2

// madvise advice values
#define MADV_NORMAL     0
#define MADV_RANDOM     1
#define MADV_SEQUENTIAL 2
#define MADV_WILLNEED   3
#define MADV_DONTNEED   4
#define MADV_FREE       8
#define MADV_HUGEPAGE   14
#define MADV_NOHUGEPAGE 15

// mlock flags (for mlock2)
#define MLOCK_ONFAULT   0x01

extern "C" {

// Memory mapping
void* mmap(void* addr, size_t length, int prot, int flags, int fd, long offset);
int munmap(void* addr, size_t length);
int mprotect(void* addr, size_t len, int prot);
int msync(void* addr, size_t length, int flags);
int madvise(void* addr, size_t length, int advice);

// Memory locking
int mlock(const void* addr, size_t len);
int mlock2(const void* addr, size_t len, unsigned int flags);
int munlock(const void* addr, size_t len);
int mlockall(int flags);
int munlockall(void);

// Shared memory
int shm_open(const char* name, int oflag, unsigned int mode);
int shm_unlink(const char* name);

// Memory residency
int mincore(void* addr, size_t length, unsigned char* vec);

}

#endif // _FRAGILE_SYS_MMAN_H_
