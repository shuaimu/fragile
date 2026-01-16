// Minimal limits.h stub for fragile parsing
#ifndef _FRAGILE_LIMITS_H_
#define _FRAGILE_LIMITS_H_

#include "climits"
#include "algorithm"  // For std::max/std::min (commonly expected)

// Make max/min available in global namespace for C-style code
using std::max;
using std::min;

// POSIX path limits
#ifndef PATH_MAX
#define PATH_MAX 4096
#endif

#ifndef NAME_MAX
#define NAME_MAX 255
#endif

#ifndef PIPE_BUF
#define PIPE_BUF 4096
#endif

#endif // _FRAGILE_LIMITS_H_
