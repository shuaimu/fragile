// Fragile stub header for <sys/resource.h>
// Resource usage and limits

#ifndef _SYS_RESOURCE_H
#define _SYS_RESOURCE_H

#include <sys/time.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

// Resource usage who
#define RUSAGE_SELF     0
#define RUSAGE_CHILDREN -1
#define RUSAGE_THREAD   1

// Resource limits
#define RLIMIT_CPU      0   // CPU time in seconds
#define RLIMIT_FSIZE    1   // Maximum file size
#define RLIMIT_DATA     2   // Max data size
#define RLIMIT_STACK    3   // Max stack size
#define RLIMIT_CORE     4   // Max core file size
#define RLIMIT_RSS      5   // Max resident set size
#define RLIMIT_NPROC    6   // Max number of processes
#define RLIMIT_NOFILE   7   // Max number of open files
#define RLIMIT_MEMLOCK  8   // Max locked memory
#define RLIMIT_AS       9   // Address space limit

#define RLIM_INFINITY   (~0UL)

// Priority values
#define PRIO_MIN        -20
#define PRIO_MAX        20
#define PRIO_PROCESS    0
#define PRIO_PGRP       1
#define PRIO_USER       2

typedef unsigned long rlim_t;

// Resource usage structure
struct rusage {
    struct timeval ru_utime;    // User time used
    struct timeval ru_stime;    // System time used
    long   ru_maxrss;           // Maximum resident set size
    long   ru_ixrss;            // Integral shared memory size
    long   ru_idrss;            // Integral unshared data size
    long   ru_isrss;            // Integral unshared stack size
    long   ru_minflt;           // Page reclaims (soft page faults)
    long   ru_majflt;           // Page faults (hard page faults)
    long   ru_nswap;            // Swaps
    long   ru_inblock;          // Block input operations
    long   ru_oublock;          // Block output operations
    long   ru_msgsnd;           // IPC messages sent
    long   ru_msgrcv;           // IPC messages received
    long   ru_nsignals;         // Signals received
    long   ru_nvcsw;            // Voluntary context switches
    long   ru_nivcsw;           // Involuntary context switches
};

// Resource limit structure
struct rlimit {
    rlim_t rlim_cur;    // Soft limit
    rlim_t rlim_max;    // Hard limit
};

// Function declarations
int getrusage(int who, struct rusage* usage);
int getrlimit(int resource, struct rlimit* rlim);
int setrlimit(int resource, const struct rlimit* rlim);
int getpriority(int which, id_t who);
int setpriority(int which, id_t who, int prio);

#ifdef __cplusplus
}
#endif

#endif // _SYS_RESOURCE_H
