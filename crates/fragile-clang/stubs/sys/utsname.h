// Minimal sys/utsname.h stub for fragile parsing
#ifndef _SYS_UTSNAME_H
#define _SYS_UTSNAME_H

// Length of utsname fields (Linux)
#define _UTSNAME_LENGTH 65

// System name structure
struct utsname {
    char sysname[_UTSNAME_LENGTH];    // Operating system name
    char nodename[_UTSNAME_LENGTH];   // Node name on network
    char release[_UTSNAME_LENGTH];    // OS release
    char version[_UTSNAME_LENGTH];    // OS version
    char machine[_UTSNAME_LENGTH];    // Hardware type
#ifdef _GNU_SOURCE
    char domainname[_UTSNAME_LENGTH]; // NIS domain name (GNU extension)
#endif
};

// Get system information
extern "C" int uname(struct utsname* name);

#endif // _SYS_UTSNAME_H
