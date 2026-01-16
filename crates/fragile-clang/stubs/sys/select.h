// Minimal sys/select.h stub for fragile parsing
#ifndef _FRAGILE_SYS_SELECT_H_
#define _FRAGILE_SYS_SELECT_H_

#include "types.h"
#include "time.h"  // For timeval, timespec, fd_set

extern "C" {

// select functions
int select(int nfds, fd_set* readfds, fd_set* writefds, fd_set* exceptfds, struct timeval* timeout);
int pselect(int nfds, fd_set* readfds, fd_set* writefds, fd_set* exceptfds, const struct timespec* timeout, const void* sigmask);

}

#endif // _FRAGILE_SYS_SELECT_H_
