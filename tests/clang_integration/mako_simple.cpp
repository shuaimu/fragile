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

// Integer minimum
// Uses C++ name mangling: _ZN3rrr7min_intEii
int min_int(int a, int b) {
    return (a < b) ? a : b;
}

// Integer maximum
// Uses C++ name mangling: _ZN3rrr7max_intEii
int max_int(int a, int b) {
    return (a > b) ? a : b;
}

// Clamp integer to range [min_val, max_val]
// Uses C++ name mangling: _ZN3rrr9clamp_intEiii
int clamp_int(int value, int min_val, int max_val) {
    if (value < min_val) return min_val;
    if (value > max_val) return max_val;
    return value;
}

// Check if pointer is null
// Uses C++ name mangling: _ZN3rrr7is_nullEPKv
bool is_null(const void* ptr) {
    return ptr == nullptr;
}

// String length without stdlib
// Uses C++ name mangling: _ZN3rrr7str_lenEPKc
int str_len(const char* str) {
    int len = 0;
    while (*str) {
        len++;
        str++;
    }
    return len;
}

} // namespace rrr
