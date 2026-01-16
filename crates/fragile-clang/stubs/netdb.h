// Fragile stub header for <netdb.h>
// Network database operations

#ifndef _NETDB_H
#define _NETDB_H

#include <netinet/in.h>
#include <sys/socket.h>

#ifdef __cplusplus
extern "C" {
#endif

// Host entry structure
struct hostent {
    char*  h_name;       // Official name of host
    char** h_aliases;    // Alias list
    int    h_addrtype;   // Host address type
    int    h_length;     // Length of address
    char** h_addr_list;  // List of addresses
};
#define h_addr h_addr_list[0]

// Service entry structure
struct servent {
    char*  s_name;       // Official service name
    char** s_aliases;    // Alias list
    int    s_port;       // Port number
    char*  s_proto;      // Protocol to use
};

// Protocol entry structure
struct protoent {
    char*  p_name;       // Official protocol name
    char** p_aliases;    // Alias list
    int    p_proto;      // Protocol number
};

// Address info structure
struct addrinfo {
    int              ai_flags;
    int              ai_family;
    int              ai_socktype;
    int              ai_protocol;
    socklen_t        ai_addrlen;
    struct sockaddr* ai_addr;
    char*            ai_canonname;
    struct addrinfo* ai_next;
};

// ai_flags values
#define AI_PASSIVE     0x0001
#define AI_CANONNAME   0x0002
#define AI_NUMERICHOST 0x0004
#define AI_V4MAPPED    0x0008
#define AI_ALL         0x0010
#define AI_ADDRCONFIG  0x0020
#define AI_NUMERICSERV 0x0400

// Error codes for getaddrinfo
#define EAI_BADFLAGS   -1
#define EAI_NONAME     -2
#define EAI_AGAIN      -3
#define EAI_FAIL       -4
#define EAI_FAMILY     -6
#define EAI_SOCKTYPE   -7
#define EAI_SERVICE    -8
#define EAI_MEMORY     -10
#define EAI_SYSTEM     -11
#define EAI_OVERFLOW   -12

// ni_flags values for getnameinfo
#define NI_NUMERICHOST 0x0001
#define NI_NUMERICSERV 0x0002
#define NI_NOFQDN      0x0004
#define NI_NAMEREQD    0x0008
#define NI_DGRAM       0x0010

// Function declarations
struct hostent* gethostbyname(const char* name);
struct hostent* gethostbyaddr(const void* addr, socklen_t len, int type);
struct servent* getservbyname(const char* name, const char* proto);
struct servent* getservbyport(int port, const char* proto);
struct protoent* getprotobyname(const char* name);
struct protoent* getprotobynumber(int proto);

int getaddrinfo(const char* node, const char* service,
                const struct addrinfo* hints, struct addrinfo** res);
void freeaddrinfo(struct addrinfo* res);
const char* gai_strerror(int errcode);

int getnameinfo(const struct sockaddr* addr, socklen_t addrlen,
                char* host, socklen_t hostlen,
                char* serv, socklen_t servlen, int flags);

#ifdef __cplusplus
}
#endif

#endif // _NETDB_H
