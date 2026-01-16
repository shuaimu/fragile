// strop_minimal.cpp - subset of rrr::strop functions for M6.3 testing
//
// This file contains the C-string based functions from vendor/mako/src/rrr/base/strop.cpp.
// These functions only depend on strlen/strncmp from <string.h>, not STL.
//
// The STL-dependent functions (format_decimal, strsplit) are deferred to later milestones.

#include <string.h>  // for strlen, strncmp

namespace rrr {

// Check if str starts with head
// Mangled: _ZN3rrr10startswithEPKcS1_
bool startswith(const char* str, const char* head) {
    size_t len_str = strlen(str);
    size_t len_head = strlen(head);
    if (len_head > len_str) {
        return false;
    }
    return strncmp(str, head, len_head) == 0;
}

// Check if str ends with tail
// Mangled: _ZN3rrr8endswithEPKcS1_
bool endswith(const char* str, const char* tail) {
    size_t len_str = strlen(str);
    size_t len_tail = strlen(tail);
    if (len_tail > len_str) {
        return false;
    }
    return strncmp(str + (len_str - len_tail), tail, len_tail) == 0;
}

} // namespace rrr
