// Minimal unistd.h stub for fragile parsing (POSIX API)
// Uses same include guard as system header to avoid conflicts
#ifndef _UNISTD_H
#define _UNISTD_H

#include "cstdint"

// Standard symbolic constants
#define STDIN_FILENO 0
#define STDOUT_FILENO 1
#define STDERR_FILENO 2

// NULL
#ifndef NULL
#define NULL nullptr
#endif

// Types (using conditional defines to avoid redefinition)
#ifndef __pid_t_defined
typedef int pid_t;
#define __pid_t_defined
#endif

#ifndef __uid_t_defined
typedef unsigned int uid_t;
#define __uid_t_defined
#endif

#ifndef __gid_t_defined
typedef unsigned int gid_t;
#define __gid_t_defined
#endif

#ifndef __ssize_t_defined
typedef long ssize_t;
#define __ssize_t_defined
#endif

#ifndef __off_t_defined
typedef long off_t;
#define __off_t_defined
#endif

#ifndef __useconds_t_defined
typedef unsigned int useconds_t;
#define __useconds_t_defined
#endif

// Note: intptr_t is defined in cstdint, don't redefine it

extern "C" {

// Process control
pid_t fork(void);
pid_t vfork(void);
int execve(const char* pathname, char* const argv[], char* const envp[]);
int execv(const char* path, char* const argv[]);
int execvp(const char* file, char* const argv[]);
int execl(const char* path, const char* arg, ...);
int execlp(const char* file, const char* arg, ...);
void _exit(int status);

// Process identification
pid_t getpid(void);
pid_t getppid(void);
pid_t getpgrp(void);
pid_t getpgid(pid_t pid);
int setpgid(pid_t pid, pid_t pgid);
pid_t setsid(void);
pid_t getsid(pid_t pid);

// User/group identification
uid_t getuid(void);
uid_t geteuid(void);
gid_t getgid(void);
gid_t getegid(void);
int setuid(uid_t uid);
int seteuid(uid_t uid);
int setgid(gid_t gid);
int setegid(gid_t gid);

// File operations
int close(int fd);
ssize_t read(int fd, void* buf, size_t count);
ssize_t write(int fd, const void* buf, size_t count);
off_t lseek(int fd, off_t offset, int whence);
int dup(int oldfd);
int dup2(int oldfd, int newfd);
int pipe(int pipefd[2]);

// File system
int chdir(const char* path);
int fchdir(int fd);
char* getcwd(char* buf, size_t size);
int access(const char* pathname, int mode);
int faccessat(int dirfd, const char* pathname, int mode, int flags);
int link(const char* oldpath, const char* newpath);
int unlink(const char* pathname);
int rmdir(const char* pathname);
// Note: rename is in cstdio, don't redeclare
int symlink(const char* target, const char* linkpath);
ssize_t readlink(const char* pathname, char* buf, size_t bufsiz);
int truncate(const char* path, off_t length);
int ftruncate(int fd, off_t length);

// Misc
int isatty(int fd);
char* ttyname(int fd);
int ttyname_r(int fd, char* buf, size_t buflen);
unsigned int sleep(unsigned int seconds);
int usleep(useconds_t usec);
unsigned int alarm(unsigned int seconds);
int pause(void);
int chown(const char* pathname, uid_t owner, gid_t group);
int fchown(int fd, uid_t owner, gid_t group);
long sysconf(int name);
long pathconf(const char* path, int name);
long fpathconf(int fd, int name);
int getopt(int argc, char* const argv[], const char* optstring);
char* getlogin(void);
int getlogin_r(char* buf, size_t bufsize);
int gethostname(char* name, size_t len);
int sethostname(const char* name, size_t len);

// Sync
void sync(void);
int fsync(int fd);
int fdatasync(int fd);

// Access mode flags
#define F_OK 0
#define X_OK 1
#define W_OK 2
#define R_OK 4

// Seek constants
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

// sysconf constants
#define _SC_ARG_MAX              0
#define _SC_CHILD_MAX            1
#define _SC_CLK_TCK              2
#define _SC_NGROUPS_MAX          3
#define _SC_OPEN_MAX             4
#define _SC_STREAM_MAX           5
#define _SC_TZNAME_MAX           6
#define _SC_JOB_CONTROL          7
#define _SC_SAVED_IDS            8
#define _SC_REALTIME_SIGNALS     9
#define _SC_PRIORITY_SCHEDULING 10
#define _SC_TIMERS              11
#define _SC_ASYNCHRONOUS_IO     12
#define _SC_PRIORITIZED_IO      13
#define _SC_SYNCHRONIZED_IO     14
#define _SC_FSYNC               15
#define _SC_MAPPED_FILES        16
#define _SC_MEMLOCK             17
#define _SC_MEMLOCK_RANGE       18
#define _SC_MEMORY_PROTECTION   19
#define _SC_MESSAGE_PASSING     20
#define _SC_SEMAPHORES          21
#define _SC_SHARED_MEMORY_OBJECTS 22
#define _SC_AIO_LISTIO_MAX      23
#define _SC_AIO_MAX             24
#define _SC_AIO_PRIO_DELTA_MAX  25
#define _SC_DELAYTIMER_MAX      26
#define _SC_MQ_OPEN_MAX         27
#define _SC_MQ_PRIO_MAX         28
#define _SC_VERSION             29
#define _SC_PAGESIZE            30
#define _SC_PAGE_SIZE           _SC_PAGESIZE
#define _SC_RTSIG_MAX           31
#define _SC_SEM_NSEMS_MAX       32
#define _SC_SEM_VALUE_MAX       33
#define _SC_SIGQUEUE_MAX        34
#define _SC_TIMER_MAX           35
#define _SC_NPROCESSORS_CONF    83
#define _SC_NPROCESSORS_ONLN    84

// syscall - direct system call interface
long syscall(long number, ...);

// System call numbers (Linux x86_64)
#define SYS_read           0
#define SYS_write          1
#define SYS_open           2
#define SYS_close          3
#define SYS_stat           4
#define SYS_fstat          5
#define SYS_lstat          6
#define SYS_poll           7
#define SYS_lseek          8
#define SYS_mmap           9
#define SYS_mprotect      10
#define SYS_munmap        11
#define SYS_brk           12
#define SYS_ioctl         16
#define SYS_access        21
#define SYS_pipe          22
#define SYS_dup           32
#define SYS_dup2          33
#define SYS_pause         34
#define SYS_getpid        39
#define SYS_socket        41
#define SYS_connect       42
#define SYS_accept        43
#define SYS_sendto        44
#define SYS_recvfrom      45
#define SYS_sendmsg       46
#define SYS_recvmsg       47
#define SYS_shutdown      48
#define SYS_bind          49
#define SYS_listen        50
#define SYS_getsockname   51
#define SYS_getpeername   52
#define SYS_fork          57
#define SYS_vfork         58
#define SYS_execve        59
#define SYS_exit          60
#define SYS_wait4         61
#define SYS_kill          62
#define SYS_fcntl         72
#define SYS_gettid       186
#define SYS_epoll_create 213
#define SYS_epoll_ctl    233
#define SYS_epoll_wait   232

// External variables
extern char* optarg;
extern int optind, opterr, optopt;

}

#endif // _UNISTD_H
