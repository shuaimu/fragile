// Fragile stub header for <dirent.h>
// Directory entry operations

#ifndef _DIRENT_H
#define _DIRENT_H

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

// Directory entry structure
struct dirent {
    unsigned long d_ino;       // Inode number
    unsigned long d_off;       // Offset to next dirent
    unsigned short d_reclen;   // Length of this record
    unsigned char d_type;      // Type of file
    char d_name[256];          // Filename
};

// Directory stream
typedef struct __dirstream DIR;

// File types
#define DT_UNKNOWN  0
#define DT_FIFO     1
#define DT_CHR      2
#define DT_DIR      4
#define DT_BLK      6
#define DT_REG      8
#define DT_LNK      10
#define DT_SOCK     12
#define DT_WHT      14

// Directory operations
DIR* opendir(const char* name);
DIR* fdopendir(int fd);
int closedir(DIR* dirp);
struct dirent* readdir(DIR* dirp);
int readdir_r(DIR* dirp, struct dirent* entry, struct dirent** result);
void rewinddir(DIR* dirp);
void seekdir(DIR* dirp, long loc);
long telldir(DIR* dirp);
int dirfd(DIR* dirp);

// Scanning
int scandir(const char* dirp, struct dirent*** namelist,
            int (*filter)(const struct dirent*),
            int (*compar)(const struct dirent**, const struct dirent**));
int alphasort(const struct dirent** a, const struct dirent** b);
int versionsort(const struct dirent** a, const struct dirent** b);

#ifdef __cplusplus
}
#endif

#endif // _DIRENT_H
