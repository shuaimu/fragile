// Minimal sys/time.h stub for fragile parsing
// Uses same include guard as system header to avoid conflicts
#ifndef _SYS_TIME_H
#define _SYS_TIME_H

#include "../time.h"
#include "../cerrno"
#include "../cstddef"  // For NULL

// timeval structure - use conditional to avoid conflict with system headers
#ifndef __timeval_defined
struct timeval {
    time_t      tv_sec;   // Seconds
    long        tv_usec;  // Microseconds
};
#define __timeval_defined
#endif

// timezone structure (obsolete, but still used)
#ifndef __timezone_defined
struct timezone {
    int tz_minuteswest;  // Minutes west of Greenwich
    int tz_dsttime;      // Type of DST correction
};
#define __timezone_defined
#endif

// fd_set for select (simplified)
#ifndef __fd_set_defined
typedef struct {
    unsigned long fds_bits[16];
} fd_set;
#define __fd_set_defined
#endif

#ifndef FD_SETSIZE
#define FD_SETSIZE 1024
#define FD_ZERO(set)    ((void)((set)->fds_bits[0] = 0))
#define FD_SET(fd, set) ((void)((set)->fds_bits[(fd)/64] |= (1UL << ((fd) % 64))))
#define FD_CLR(fd, set) ((void)((set)->fds_bits[(fd)/64] &= ~(1UL << ((fd) % 64))))
#define FD_ISSET(fd, set) (((set)->fds_bits[(fd)/64] & (1UL << ((fd) % 64))) != 0)
#endif

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
#ifndef ITIMER_REAL
#define ITIMER_REAL    0
#define ITIMER_VIRTUAL 1
#define ITIMER_PROF    2
#endif

#ifndef __itimerval_defined
struct itimerval {
    struct timeval it_interval;  // Interval for periodic timer
    struct timeval it_value;     // Time until next expiration
};
#define __itimerval_defined
#endif

int getitimer(int which, struct itimerval* value);
int setitimer(int which, const struct itimerval* new_value, struct itimerval* old_value);

}

// Timer manipulation macros (BSD/POSIX)
#ifndef timerclear
#define timerclear(tvp)    ((tvp)->tv_sec = (tvp)->tv_usec = 0)
#define timerisset(tvp)    ((tvp)->tv_sec || (tvp)->tv_usec)
#define timercmp(a, b, CMP) \
    (((a)->tv_sec == (b)->tv_sec) ? \
     ((a)->tv_usec CMP (b)->tv_usec) : \
     ((a)->tv_sec CMP (b)->tv_sec))
#define timeradd(a, b, result) \
    do { \
        (result)->tv_sec = (a)->tv_sec + (b)->tv_sec; \
        (result)->tv_usec = (a)->tv_usec + (b)->tv_usec; \
        if ((result)->tv_usec >= 1000000) { \
            ++(result)->tv_sec; \
            (result)->tv_usec -= 1000000; \
        } \
    } while (0)
#define timersub(a, b, result) \
    do { \
        (result)->tv_sec = (a)->tv_sec - (b)->tv_sec; \
        (result)->tv_usec = (a)->tv_usec - (b)->tv_usec; \
        if ((result)->tv_usec < 0) { \
            --(result)->tv_sec; \
            (result)->tv_usec += 1000000; \
        } \
    } while (0)
#endif

#endif // _SYS_TIME_H
