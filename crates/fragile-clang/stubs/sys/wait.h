// Fragile stub header for <sys/wait.h>
// Process wait operations

#ifndef _SYS_WAIT_H
#define _SYS_WAIT_H

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

// Wait status macros
#define WIFEXITED(status)   (((status) & 0x7f) == 0)
#define WEXITSTATUS(status) (((status) >> 8) & 0xff)
#define WIFSIGNALED(status) (((status) & 0x7f) > 0 && ((status) & 0x7f) < 0x7f)
#define WTERMSIG(status)    ((status) & 0x7f)
#define WIFSTOPPED(status)  (((status) & 0xff) == 0x7f)
#define WSTOPSIG(status)    WEXITSTATUS(status)
#define WIFCONTINUED(status) ((status) == 0xffff)
#define WCOREDUMP(status)   ((status) & 0x80)

// Options for waitpid
#define WNOHANG   1   // Don't block waiting
#define WUNTRACED 2   // Report status of stopped children
#define WCONTINUED 8  // Report continued children

// Function declarations
pid_t wait(int* status);
pid_t waitpid(pid_t pid, int* status, int options);
int waitid(int idtype, id_t id, siginfo_t* infop, int options);
pid_t wait3(int* status, int options, struct rusage* rusage);
pid_t wait4(pid_t pid, int* status, int options, struct rusage* rusage);

// ID types for waitid
#define P_ALL  0
#define P_PID  1
#define P_PGID 2

#ifdef __cplusplus
}
#endif

#endif // _SYS_WAIT_H
