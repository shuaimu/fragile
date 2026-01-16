// strop_stl.cpp - STL-dependent functions from mako strop.cpp
// M6.4: Simple mako test executable with STL
//
// This file uses std::string and std::ostringstream internally but provides
// C-compatible wrapper functions that can be called from Rust.

#include <sstream>
#include <string>
#include <iomanip>
#include <cstring>

namespace rrr {

// Internal implementation: format as -#,###.##
static std::string format_decimal_double_impl(double val) {
    std::ostringstream o;
    o.precision(2);
    o << std::fixed << val;
    std::string s = o.str();
    std::string str;

    // Find decimal point position
    size_t idx = 0;
    while (idx < s.size()) {
        if (s[idx] == '.') {
            break;
        }
        idx++;
    }

    str.reserve(s.size() + 16);

    // Add commas every 3 digits before decimal
    for (size_t i = 0; i < idx; i++) {
        if ((idx - i) % 3 == 0 && i != 0 && s[i - 1] != '-') {
            str += ',';
        }
        str += s[i];
    }

    // Add decimal part
    str += s.substr(idx);

    // Handle -0.00 case
    if (str == "-0.00") {
        str = "0.00";
    }

    return str;
}

// Internal implementation: format as -#,###
static std::string format_decimal_int_impl(int val) {
    std::ostringstream o;
    o << val;
    std::string s = o.str();
    std::string str;

    str.reserve(s.size() + 8);

    // Add commas every 3 digits
    for (size_t i = 0; i < s.size(); i++) {
        if ((s.size() - i) % 3 == 0 && i != 0 && s[i - 1] != '-') {
            str += ',';
        }
        str += s[i];
    }

    return str;
}

} // namespace rrr

// C-compatible wrapper functions for calling from Rust
extern "C" {

// Format double to buffer with commas
// Returns: length of result on success, -1 if buffer too small
// Mangled: format_decimal_double_to_buf (extern "C" - no mangling)
int format_decimal_double_to_buf(double val, char* buf, int buf_size) {
    std::string result = rrr::format_decimal_double_impl(val);
    if ((int)result.size() >= buf_size) {
        return -1; // buffer too small
    }
    std::strcpy(buf, result.c_str());
    return (int)result.size();
}

// Format int to buffer with commas
// Returns: length of result on success, -1 if buffer too small
// Mangled: format_decimal_int_to_buf (extern "C" - no mangling)
int format_decimal_int_to_buf(int val, char* buf, int buf_size) {
    std::string result = rrr::format_decimal_int_impl(val);
    if ((int)result.size() >= buf_size) {
        return -1; // buffer too small
    }
    std::strcpy(buf, result.c_str());
    return (int)result.size();
}

} // extern "C"
