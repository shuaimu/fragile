// Minimal stdbool.h stub for fragile parsing
// In C++, bool is built-in so this header is mostly for C compatibility
#ifndef _FRAGILE_STDBOOL_H_
#define _FRAGILE_STDBOOL_H_

#ifndef __cplusplus
// C definitions
#define bool    _Bool
#define true    1
#define false   0
#endif

// Macro indicating <stdbool.h> conformance
#define __bool_true_false_are_defined 1

#endif // _FRAGILE_STDBOOL_H_
