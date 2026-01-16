// Minimal unistd.h stub for fragile parsing (POSIX API)
#ifndef _FRAGILE_UNISTD_H_
#define _FRAGILE_UNISTD_H_

#include "cstdint"

// Standard symbolic constants
#define STDIN_FILENO 0
#define STDOUT_FILENO 1
#define STDERR_FILENO 2

// NULL
#ifndef NULL
#define NULL nullptr
#endif

// Types
typedef int pid_t;
typedef unsigned int uid_t;
typedef unsigned int gid_t;
typedef long ssize_t;
typedef long off_t;
typedef unsigned int useconds_t;
typedef int intptr_t;

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
int rename(const char* oldpath, const char* newpath);
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

// External variables
extern char* optarg;
extern int optind, opterr, optopt;

}

#endif // _FRAGILE_UNISTD_H_
