// Minimal sys/uio.h stub for fragile parsing
#ifndef _SYS_UIO_H
#define _SYS_UIO_H

#include "types.h"

// I/O vector structure for readv/writev
struct iovec {
    void  *iov_base;    // Starting address
    size_t iov_len;     // Number of bytes to transfer
};

// Maximum number of iovec entries
#define IOV_MAX 1024
#define UIO_MAXIOV IOV_MAX

// Scatter/gather I/O functions
ssize_t readv(int fd, const struct iovec *iov, int iovcnt);
ssize_t writev(int fd, const struct iovec *iov, int iovcnt);

// Positioned scatter/gather I/O (POSIX.1-2008)
ssize_t preadv(int fd, const struct iovec *iov, int iovcnt, off_t offset);
ssize_t pwritev(int fd, const struct iovec *iov, int iovcnt, off_t offset);

#endif // _SYS_UIO_H
