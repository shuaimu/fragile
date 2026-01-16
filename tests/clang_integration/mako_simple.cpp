// Minimal mako utility functions for testing Fragile compilation pipeline
// M5.8: Basic mako operations
//
// These functions mimic rrr::startswith/endswith from mako but without
// STL dependencies. This allows testing the full compilation pipeline
// from C++ source through rustc linking.

namespace rrr {

// Check if string starts with prefix
// Uses C++ name mangling: _ZN3rrr10startswithEPKcS1_
bool startswith(const char* str, const char* head) {
    // Walk both strings in parallel
    while (*head) {
        if (*str != *head) {
            return false;
        }
        str++;
        head++;
    }
    return true;
}

// Check if string ends with suffix
// Uses C++ name mangling: _ZN3rrr8endswithEPKcS1_
bool endswith(const char* str, const char* tail) {
    // Find length of both strings without using strlen
    const char* s = str;
    const char* t = tail;
    while (*s) s++;
    while (*t) t++;

    int str_len = s - str;
    int tail_len = t - tail;

    if (tail_len > str_len) {
        return false;
    }

    // Compare suffix
    const char* str_end = str + str_len - tail_len;
    while (*tail) {
        if (*str_end != *tail) {
            return false;
        }
        str_end++;
        tail++;
    }
    return true;
}

// Simple integer addition (for sanity check)
// Uses C++ name mangling: _ZN3rrr7add_intEii
int add_int(int a, int b) {
    return a + b;
}

} // namespace rrr
