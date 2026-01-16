// Minimal sys/time.h stub for fragile parsing
#ifndef _FRAGILE_SYS_TIME_H_
#define _FRAGILE_SYS_TIME_H_

#include "../time.h"
#include "../cerrno"
#include "../cstddef"  // For NULL

// timeval structure
struct timeval {
    time_t      tv_sec;   // Seconds
    long        tv_usec;  // Microseconds
};

// timezone structure (obsolete, but still used)
struct timezone {
    int tz_minuteswest;  // Minutes west of Greenwich
    int tz_dsttime;      // Type of DST correction
};

// fd_set for select (simplified)
typedef struct {
    unsigned long fds_bits[16];
} fd_set;

#define FD_SETSIZE 1024
#define FD_ZERO(set)    ((void)((set)->fds_bits[0] = 0))
#define FD_SET(fd, set) ((void)((set)->fds_bits[(fd)/64] |= (1UL << ((fd) % 64))))
#define FD_CLR(fd, set) ((void)((set)->fds_bits[(fd)/64] &= ~(1UL << ((fd) % 64))))
#define FD_ISSET(fd, set) (((set)->fds_bits[(fd)/64] & (1UL << ((fd) % 64))) != 0)

extern "C" {

// gettimeofday
int gettimeofday(struct timeval* tv, struct timezone* tz);
int settimeofday(const struct timeval* tv, const struct timezone* tz);

// select
int select(int nfds, fd_set* readfds, fd_set* writefds,
           fd_set* exceptfds, struct timeval* timeout);

// pselect
int pselect(int nfds, fd_set* readfds, fd_set* writefds,
            fd_set* exceptfds, const struct timespec* timeout,
            const void* sigmask);

// interval timers
#define ITIMER_REAL    0
#define ITIMER_VIRTUAL 1
#define ITIMER_PROF    2

struct itimerval {
    struct timeval it_interval;  // Interval for periodic timer
    struct timeval it_value;     // Time until next expiration
};

int getitimer(int which, struct itimerval* value);
int setitimer(int which, const struct itimerval* new_value, struct itimerval* old_value);

}

#endif // _FRAGILE_SYS_TIME_H_
