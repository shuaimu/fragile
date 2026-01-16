// Minimal fcntl.h stub for fragile parsing
#ifndef _FCNTL_H
#define _FCNTL_H

#include "sys/types.h"

// File access modes
#define O_RDONLY    0x0000
#define O_WRONLY    0x0001
#define O_RDWR      0x0002
#define O_ACCMODE   0x0003

// File creation flags
#define O_CREAT     0x0040
#define O_EXCL      0x0080
#define O_NOCTTY    0x0100
#define O_TRUNC     0x0200
#define O_APPEND    0x0400
#define O_NONBLOCK  0x0800
#define O_NDELAY    O_NONBLOCK
#define O_DSYNC     0x1000
#define O_SYNC      0x101000
#define O_RSYNC     O_SYNC
#define O_DIRECT    0x4000
#define O_LARGEFILE 0x8000
#define O_DIRECTORY 0x10000
#define O_NOFOLLOW  0x20000
#define O_NOATIME   0x40000
#define O_CLOEXEC   0x80000

// fcntl command values
#define F_DUPFD     0
#define F_GETFD     1
#define F_SETFD     2
#define F_GETFL     3
#define F_SETFL     4
#define F_GETLK     5
#define F_SETLK     6
#define F_SETLKW    7

// File descriptor flags
#define FD_CLOEXEC  1

// File locking
#define F_RDLCK     0
#define F_WRLCK     1
#define F_UNLCK     2

// flock structure for record locking
struct flock {
    short l_type;   // Type of lock: F_RDLCK, F_WRLCK, F_UNLCK
    short l_whence; // How to interpret l_start
    off_t l_start;  // Starting offset for lock
    off_t l_len;    // Number of bytes to lock
    pid_t l_pid;    // Process ID of process blocking our lock
};

// File control functions
int fcntl(int fd, int cmd, ...);
int open(const char *pathname, int flags, ...);
int openat(int dirfd, const char *pathname, int flags, ...);
int creat(const char *pathname, mode_t mode);

// Advisory locks
int posix_fadvise(int fd, off_t offset, off_t len, int advice);
int posix_fallocate(int fd, off_t offset, off_t len);

// posix_fadvise advice values
#define POSIX_FADV_NORMAL     0
#define POSIX_FADV_RANDOM     1
#define POSIX_FADV_SEQUENTIAL 2
#define POSIX_FADV_WILLNEED   3
#define POSIX_FADV_DONTNEED   4
#define POSIX_FADV_NOREUSE    5

#endif // _FCNTL_H
