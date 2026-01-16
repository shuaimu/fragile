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

// M6.2: String utilities without STL

// String compare (like strcmp)
// Returns: negative if s1 < s2, 0 if equal, positive if s1 > s2
// Uses C++ name mangling: _ZN3rrr7str_cmpEPKcS1_
int str_cmp(const char* s1, const char* s2) {
    while (*s1 && (*s1 == *s2)) {
        s1++;
        s2++;
    }
    return static_cast<unsigned char>(*s1) - static_cast<unsigned char>(*s2);
}

// String compare with length limit (like strncmp)
// Uses C++ name mangling: _ZN3rrr8str_ncmpEPKcS1_i
int str_ncmp(const char* s1, const char* s2, int n) {
    for (int i = 0; i < n; i++) {
        if (s1[i] != s2[i]) {
            return static_cast<unsigned char>(s1[i]) - static_cast<unsigned char>(s2[i]);
        }
        if (s1[i] == '\0') {
            return 0;
        }
    }
    return 0;
}

// Copy string (like strcpy)
// Returns pointer to destination
// Uses C++ name mangling: _ZN3rrr7str_cpyEPcPKc
char* str_cpy(char* dest, const char* src) {
    char* ret = dest;
    while ((*dest++ = *src++)) {
        // Copy until null terminator
    }
    return ret;
}

// Copy string with length limit (like strncpy)
// Uses C++ name mangling: _ZN3rrr8str_ncpyEPcPKci
char* str_ncpy(char* dest, const char* src, int n) {
    char* ret = dest;
    int i = 0;
    while (i < n && src[i] != '\0') {
        dest[i] = src[i];
        i++;
    }
    // Pad with zeros if source is shorter
    while (i < n) {
        dest[i] = '\0';
        i++;
    }
    return ret;
}

// Find character in string (like strchr)
// Returns pointer to first occurrence or null
// Uses C++ name mangling: _ZN3rrr7str_chrEPKcc
const char* str_chr(const char* str, char c) {
    while (*str) {
        if (*str == c) {
            return str;
        }
        str++;
    }
    // Check for null terminator match
    if (c == '\0') {
        return str;
    }
    return nullptr;
}

// Find last character in string (like strrchr)
// Returns pointer to last occurrence or null
// Uses C++ name mangling: _ZN3rrr8str_rchrEPKcc
const char* str_rchr(const char* str, char c) {
    const char* last = nullptr;
    while (*str) {
        if (*str == c) {
            last = str;
        }
        str++;
    }
    // Check for null terminator match
    if (c == '\0') {
        return str;
    }
    return last;
}

} // namespace rrr
