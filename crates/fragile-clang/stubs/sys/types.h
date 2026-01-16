// Minimal sys/types.h stub for fragile parsing
#ifndef _FRAGILE_SYS_TYPES_H_
#define _FRAGILE_SYS_TYPES_H_

#include "../cstdint"

// POSIX types
typedef int pid_t;
typedef unsigned int uid_t;
typedef unsigned int gid_t;
typedef unsigned int mode_t;
typedef long off_t;
typedef long ssize_t;
typedef unsigned long ino_t;
typedef unsigned long dev_t;
typedef unsigned long nlink_t;
typedef long blksize_t;
typedef long blkcnt_t;
typedef long time_t;
typedef long suseconds_t;
typedef unsigned int useconds_t;
typedef int clockid_t;

// Socket types
typedef unsigned int socklen_t;
typedef unsigned short sa_family_t;
typedef int key_t;

// 64-bit variants
typedef long long off64_t;
typedef unsigned long long ino64_t;
typedef long long blkcnt64_t;

#endif // _FRAGILE_SYS_TYPES_H_
