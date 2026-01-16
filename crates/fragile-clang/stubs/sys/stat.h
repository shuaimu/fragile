// Fragile stub header for <sys/stat.h>
// File status and permission operations

#ifndef _SYS_STAT_H
#define _SYS_STAT_H

#include <sys/types.h>
#include <time.h>

#ifdef __cplusplus
extern "C" {
#endif

// File type masks
#define S_IFMT   0170000  // Type of file
#define S_IFSOCK 0140000  // Socket
#define S_IFLNK  0120000  // Symbolic link
#define S_IFREG  0100000  // Regular file
#define S_IFBLK  0060000  // Block device
#define S_IFDIR  0040000  // Directory
#define S_IFCHR  0020000  // Character device
#define S_IFIFO  0010000  // FIFO

// File type test macros
#define S_ISREG(m)  (((m) & S_IFMT) == S_IFREG)
#define S_ISDIR(m)  (((m) & S_IFMT) == S_IFDIR)
#define S_ISCHR(m)  (((m) & S_IFMT) == S_IFCHR)
#define S_ISBLK(m)  (((m) & S_IFMT) == S_IFBLK)
#define S_ISFIFO(m) (((m) & S_IFMT) == S_IFIFO)
#define S_ISLNK(m)  (((m) & S_IFMT) == S_IFLNK)
#define S_ISSOCK(m) (((m) & S_IFMT) == S_IFSOCK)

// Permission bits
#define S_ISUID 04000  // Set user ID on execution
#define S_ISGID 02000  // Set group ID on execution
#define S_ISVTX 01000  // Sticky bit

#define S_IRWXU 00700  // Owner permissions mask
#define S_IRUSR 00400  // Owner read
#define S_IWUSR 00200  // Owner write
#define S_IXUSR 00100  // Owner execute

#define S_IRWXG 00070  // Group permissions mask
#define S_IRGRP 00040  // Group read
#define S_IWGRP 00020  // Group write
#define S_IXGRP 00010  // Group execute

#define S_IRWXO 00007  // Others permissions mask
#define S_IROTH 00004  // Others read
#define S_IWOTH 00002  // Others write
#define S_IXOTH 00001  // Others execute

// Stat structure
struct stat {
    dev_t     st_dev;      // ID of device containing file
    ino_t     st_ino;      // Inode number
    mode_t    st_mode;     // File mode
    nlink_t   st_nlink;    // Number of hard links
    uid_t     st_uid;      // User ID of owner
    gid_t     st_gid;      // Group ID of owner
    dev_t     st_rdev;     // Device ID (if special file)
    off_t     st_size;     // Total size, in bytes
    blksize_t st_blksize;  // Block size for filesystem I/O
    blkcnt_t  st_blocks;   // Number of 512B blocks allocated
    time_t    st_atime;    // Time of last access
    time_t    st_mtime;    // Time of last modification
    time_t    st_ctime;    // Time of last status change
};

// Function declarations
int stat(const char* pathname, struct stat* statbuf);
int fstat(int fd, struct stat* statbuf);
int lstat(const char* pathname, struct stat* statbuf);
int fstatat(int dirfd, const char* pathname, struct stat* statbuf, int flags);
int chmod(const char* pathname, mode_t mode);
int fchmod(int fd, mode_t mode);
int mkdir(const char* pathname, mode_t mode);
int mkdirat(int dirfd, const char* pathname, mode_t mode);
int mkfifo(const char* pathname, mode_t mode);
int mknod(const char* pathname, mode_t mode, dev_t dev);
mode_t umask(mode_t mask);

// Additional flags for fstatat
#define AT_SYMLINK_NOFOLLOW 0x100
#define AT_EMPTY_PATH       0x1000

#ifdef __cplusplus
}
#endif

#endif // _SYS_STAT_H
