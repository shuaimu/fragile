// Minimal sys/select.h stub for fragile parsing
#ifndef _FRAGILE_SYS_SELECT_H_
#define _FRAGILE_SYS_SELECT_H_

#include "types.h"
#include "../cstdint"

// fd_set and related macros
#define FD_SETSIZE 1024

typedef struct {
    unsigned long fds_bits[FD_SETSIZE / (8 * sizeof(unsigned long))];
} fd_set;

#define FD_ZERO(set) do { \
    for (size_t __i = 0; __i < sizeof((set)->fds_bits) / sizeof((set)->fds_bits[0]); __i++) \
        (set)->fds_bits[__i] = 0; \
} while (0)

#define FD_SET(fd, set) ((set)->fds_bits[(fd) / (8 * sizeof(unsigned long))] |= (1UL << ((fd) % (8 * sizeof(unsigned long)))))
#define FD_CLR(fd, set) ((set)->fds_bits[(fd) / (8 * sizeof(unsigned long))] &= ~(1UL << ((fd) % (8 * sizeof(unsigned long)))))
#define FD_ISSET(fd, set) (((set)->fds_bits[(fd) / (8 * sizeof(unsigned long))] & (1UL << ((fd) % (8 * sizeof(unsigned long))))) != 0)

// timeval structure
struct timeval {
    time_t tv_sec;
    suseconds_t tv_usec;
};

// timespec structure (also used by pselect)
struct timespec {
    time_t tv_sec;
    long tv_nsec;
};

extern "C" {

// select functions
int select(int nfds, fd_set* readfds, fd_set* writefds, fd_set* exceptfds, struct timeval* timeout);
int pselect(int nfds, fd_set* readfds, fd_set* writefds, fd_set* exceptfds, const struct timespec* timeout, const void* sigmask);

}

#endif // _FRAGILE_SYS_SELECT_H_
