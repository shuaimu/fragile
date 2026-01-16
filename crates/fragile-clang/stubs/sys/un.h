// Minimal sys/un.h stub for fragile parsing (Unix domain sockets)
#ifndef _FRAGILE_SYS_UN_H_
#define _FRAGILE_SYS_UN_H_

#include "socket.h"

// Path length for Unix domain sockets
#define UNIX_PATH_MAX 108

// Unix domain socket address structure
struct sockaddr_un {
    sa_family_t sun_family;
    char sun_path[UNIX_PATH_MAX];
};

#endif // _FRAGILE_SYS_UN_H_
