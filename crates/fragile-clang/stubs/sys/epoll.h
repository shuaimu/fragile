// Fragile stub header for <sys/epoll.h>
// Event poll interface

#ifndef _SYS_EPOLL_H
#define _SYS_EPOLL_H

#include <stdint.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

// Event types
#define EPOLLIN      0x001
#define EPOLLPRI     0x002
#define EPOLLOUT     0x004
#define EPOLLERR     0x008
#define EPOLLHUP     0x010
#define EPOLLNVAL    0x020
#define EPOLLRDNORM  0x040
#define EPOLLRDBAND  0x080
#define EPOLLWRNORM  0x100
#define EPOLLWRBAND  0x200
#define EPOLLMSG     0x400
#define EPOLLRDHUP   0x2000
#define EPOLLEXCLUSIVE 0x10000000
#define EPOLLWAKEUP  0x20000000
#define EPOLLONESHOT 0x40000000
#define EPOLLET      0x80000000

// Operation types for epoll_ctl
#define EPOLL_CTL_ADD 1
#define EPOLL_CTL_DEL 2
#define EPOLL_CTL_MOD 3

// Flags for epoll_create1
#define EPOLL_CLOEXEC 0x80000

// Data union
typedef union epoll_data {
    void* ptr;
    int fd;
    uint32_t u32;
    uint64_t u64;
} epoll_data_t;

// Event structure
struct epoll_event {
    uint32_t events;    // Epoll events
    epoll_data_t data;  // User data variable
};

// Function declarations
int epoll_create(int size);
int epoll_create1(int flags);
int epoll_ctl(int epfd, int op, int fd, struct epoll_event* event);
int epoll_wait(int epfd, struct epoll_event* events, int maxevents, int timeout);
int epoll_pwait(int epfd, struct epoll_event* events, int maxevents, int timeout,
                const sigset_t* sigmask);

#ifdef __cplusplus
}
#endif

#endif // _SYS_EPOLL_H
