// Minimal sched.h stub for fragile parsing
#ifndef _FRAGILE_SCHED_H_
#define _FRAGILE_SCHED_H_

#include "cstdint"

// CPU set type for affinity
#define __CPU_SETSIZE 1024
#define __NCPUBITS (8 * sizeof(unsigned long))

typedef struct {
    unsigned long __bits[__CPU_SETSIZE / __NCPUBITS];
} cpu_set_t;

// CPU set macros
#define CPU_ZERO(cpusetp) \
    do { \
        for (size_t __i = 0; __i < sizeof(cpu_set_t) / sizeof(unsigned long); __i++) \
            ((cpu_set_t*)(cpusetp))->__bits[__i] = 0; \
    } while (0)

#define CPU_SET(cpu, cpusetp) \
    ((cpusetp)->__bits[(cpu) / __NCPUBITS] |= (1UL << ((cpu) % __NCPUBITS)))

#define CPU_CLR(cpu, cpusetp) \
    ((cpusetp)->__bits[(cpu) / __NCPUBITS] &= ~(1UL << ((cpu) % __NCPUBITS)))

#define CPU_ISSET(cpu, cpusetp) \
    (((cpusetp)->__bits[(cpu) / __NCPUBITS] & (1UL << ((cpu) % __NCPUBITS))) != 0)

#define CPU_COUNT(cpusetp) \
    __builtin_popcountl((cpusetp)->__bits[0])

extern "C" {

// Scheduling policy constants
#define SCHED_OTHER 0
#define SCHED_FIFO 1
#define SCHED_RR 2
#define SCHED_BATCH 3
#define SCHED_IDLE 5
#define SCHED_DEADLINE 6

// Scheduling parameter
struct sched_param {
    int sched_priority;
};

// Scheduling functions
int sched_setscheduler(int pid, int policy, const struct sched_param* param);
int sched_getscheduler(int pid);
int sched_setparam(int pid, const struct sched_param* param);
int sched_getparam(int pid, struct sched_param* param);
int sched_get_priority_max(int policy);
int sched_get_priority_min(int policy);
int sched_yield(void);

// CPU affinity functions
int sched_setaffinity(int pid, size_t cpusetsize, const cpu_set_t* cpuset);
int sched_getaffinity(int pid, size_t cpusetsize, cpu_set_t* cpuset);

}

#endif // _FRAGILE_SCHED_H_
