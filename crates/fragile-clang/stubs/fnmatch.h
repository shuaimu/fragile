// Minimal fnmatch.h stub for fragile parsing
#ifndef _FRAGILE_FNMATCH_H_
#define _FRAGILE_FNMATCH_H_

// Flags for fnmatch
#define FNM_PATHNAME    (1 << 0)  // Match pathname (no wildcard matches /)
#define FNM_NOESCAPE    (1 << 1)  // Disable backslash escaping
#define FNM_PERIOD      (1 << 2)  // Leading period must be matched explicitly
#define FNM_FILE_NAME   FNM_PATHNAME
#define FNM_LEADING_DIR (1 << 3)  // Ignore /... after a match
#define FNM_CASEFOLD    (1 << 4)  // Case-insensitive matching

// Return values
#define FNM_NOMATCH     1         // Pattern did not match
#define FNM_ERROR       (-1)      // Error occurred (obsolete, never returned)

#ifdef __cplusplus
extern "C" {
#endif

// Match filename/pathname using shell wildcard pattern
int fnmatch(const char *pattern, const char *string, int flags);

#ifdef __cplusplus
}
#endif

#endif // _FRAGILE_FNMATCH_H_
