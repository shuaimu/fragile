// Minimal time.h stub for fragile parsing
// Uses same include guard as system header to avoid conflicts
#ifndef _TIME_H
#define _TIME_H

#include "cstdint"
#include "cerrno"  // Common to need errno with time functions

// clockid_t for clock_gettime
#ifndef __clockid_t_defined
typedef int clockid_t;
#define __clockid_t_defined
#endif

// Clock identifiers
#ifndef CLOCK_REALTIME
#define CLOCK_REALTIME           0
#define CLOCK_MONOTONIC          1
#define CLOCK_PROCESS_CPUTIME_ID 2
#define CLOCK_THREAD_CPUTIME_ID  3
#define CLOCK_MONOTONIC_RAW      4
#define CLOCK_REALTIME_COARSE    5
#define CLOCK_MONOTONIC_COARSE   6
#define CLOCK_BOOTTIME           7
#endif

// time_t
#ifndef __time_t_defined
typedef long time_t;
#define __time_t_defined
#endif

// clock_t
#ifndef __clock_t_defined
typedef long clock_t;
#define __clock_t_defined
#endif

// timespec structure - use same guard as Linux kernel/glibc
#ifndef _STRUCT_TIMESPEC
#define _STRUCT_TIMESPEC 1
struct timespec {
    time_t tv_sec;   // Seconds
    long   tv_nsec;  // Nanoseconds
};
#endif

// tm structure for broken-down time
#ifndef __tm_defined
struct tm {
    int tm_sec;    // Seconds (0-60)
    int tm_min;    // Minutes (0-59)
    int tm_hour;   // Hours (0-23)
    int tm_mday;   // Day of month (1-31)
    int tm_mon;    // Month (0-11)
    int tm_year;   // Year - 1900
    int tm_wday;   // Day of week (0-6)
    int tm_yday;   // Day of year (0-365)
    int tm_isdst;  // Daylight saving time flag
};
#define __tm_defined
#endif

// Time functions
extern "C" {

time_t time(time_t* tloc);
double difftime(time_t time1, time_t time0);
time_t mktime(struct tm* timeptr);

char* asctime(const struct tm* timeptr);
char* ctime(const time_t* timer);
struct tm* gmtime(const time_t* timer);
struct tm* localtime(const time_t* timer);
size_t strftime(char* s, size_t maxsize, const char* format, const struct tm* timeptr);

// Thread-safe variants (POSIX)
struct tm* gmtime_r(const time_t* timer, struct tm* result);
struct tm* localtime_r(const time_t* timer, struct tm* result);

// POSIX clock functions
int clock_gettime(clockid_t clk_id, struct timespec* tp);
int clock_settime(clockid_t clk_id, const struct timespec* tp);
int clock_getres(clockid_t clk_id, struct timespec* res);

// nanosleep
int nanosleep(const struct timespec* req, struct timespec* rem);

// clock()
clock_t clock(void);

// C++17 timespec_get
int timespec_get(struct timespec* ts, int base);
#define TIME_UTC 1

}

#endif // _TIME_H
