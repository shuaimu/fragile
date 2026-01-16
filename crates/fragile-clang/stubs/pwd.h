// Minimal pwd.h stub for fragile parsing
#ifndef _PWD_H
#define _PWD_H

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

// passwd structure
struct passwd {
    char* pw_name;    // Username
    char* pw_passwd;  // User password
    uid_t pw_uid;     // User ID
    gid_t pw_gid;     // Group ID
    char* pw_gecos;   // User information
    char* pw_dir;     // Home directory
    char* pw_shell;   // Shell program
};

// Get password entry by UID
struct passwd* getpwuid(uid_t uid);

// Get password entry by name
struct passwd* getpwnam(const char* name);

// Get password entry (reentrant versions)
int getpwuid_r(uid_t uid, struct passwd* pwd, char* buf, size_t buflen, struct passwd** result);
int getpwnam_r(const char* name, struct passwd* pwd, char* buf, size_t buflen, struct passwd** result);

// Set/end password database access
void setpwent(void);
void endpwent(void);
struct passwd* getpwent(void);

#ifdef __cplusplus
}
#endif

#endif // _PWD_H
