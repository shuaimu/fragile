// Stub header for boost/algorithm/string.hpp
#pragma once

#include <string>
#include <vector>
#include <algorithm>
#include <cctype>
#include <sstream>
#include <iterator>

namespace boost {
namespace algorithm {

// Case conversion
template<typename T>
T to_lower_copy(const T& input) {
    T result = input;
    std::transform(result.begin(), result.end(), result.begin(), ::tolower);
    return result;
}

template<typename T>
void to_lower(T& input) {
    std::transform(input.begin(), input.end(), input.begin(), ::tolower);
}

template<typename T>
T to_upper_copy(const T& input) {
    T result = input;
    std::transform(result.begin(), result.end(), result.begin(), ::toupper);
    return result;
}

template<typename T>
void to_upper(T& input) {
    std::transform(input.begin(), input.end(), input.begin(), ::toupper);
}

// Trimming
template<typename T>
void trim_left(T& input) {
    auto it = input.begin();
    while (it != input.end() && std::isspace(*it)) ++it;
    input.erase(input.begin(), it);
}

template<typename T>
T trim_left_copy(const T& input) {
    T result = input;
    trim_left(result);
    return result;
}

template<typename T>
void trim_right(T& input) {
    auto it = input.end();
    while (it != input.begin() && std::isspace(*(it - 1))) --it;
    input.erase(it, input.end());
}

template<typename T>
T trim_right_copy(const T& input) {
    T result = input;
    trim_right(result);
    return result;
}

template<typename T>
void trim(T& input) {
    trim_left(input);
    trim_right(input);
}

template<typename T>
T trim_copy(const T& input) {
    T result = input;
    trim(result);
    return result;
}

// Predicates
inline bool is_space(char c) { return std::isspace(c); }
inline bool is_alpha(char c) { return std::isalpha(c); }
inline bool is_digit(char c) { return std::isdigit(c); }
inline bool is_alnum(char c) { return std::isalnum(c); }
inline bool is_upper(char c) { return std::isupper(c); }
inline bool is_lower(char c) { return std::islower(c); }

// Comparison predicates
template<typename T>
bool starts_with(const T& input, const T& test) {
    if (test.size() > input.size()) return false;
    return std::equal(test.begin(), test.end(), input.begin());
}

// Overload for string with const char*
inline bool starts_with(const std::string& input, const char* test) {
    std::string test_str(test);
    if (test_str.size() > input.size()) return false;
    return std::equal(test_str.begin(), test_str.end(), input.begin());
}

template<typename T>
bool ends_with(const T& input, const T& test) {
    if (test.size() > input.size()) return false;
    return std::equal(test.rbegin(), test.rend(), input.rbegin());
}

// Overload for string with const char*
inline bool ends_with(const std::string& input, const char* test) {
    std::string test_str(test);
    if (test_str.size() > input.size()) return false;
    return std::equal(test_str.rbegin(), test_str.rend(), input.rbegin());
}

template<typename T>
bool contains(const T& input, const T& test) {
    return input.find(test) != T::npos;
}

// Overload for string with const char*
inline bool contains(const std::string& input, const char* test) {
    return input.find(test) != std::string::npos;
}

template<typename T>
bool iequals(const T& a, const T& b) {
    if (a.size() != b.size()) return false;
    for (size_t i = 0; i < a.size(); ++i) {
        if (std::tolower(a[i]) != std::tolower(b[i])) return false;
    }
    return true;
}

// Split
template<typename Container, typename Predicate>
void split(Container& result, const std::string& input, Predicate pred, bool compress_empty = true) {
    result.clear();
    std::string current;
    for (char c : input) {
        if (pred(c)) {
            if (!current.empty() || !compress_empty) {
                result.push_back(current);
                current.clear();
            }
        } else {
            current += c;
        }
    }
    if (!current.empty() || !compress_empty) {
        result.push_back(current);
    }
}

// is_any_of predicate for split
class is_any_of {
public:
    is_any_of(const std::string& chars) : chars_(chars) {}
    bool operator()(char c) const {
        return chars_.find(c) != std::string::npos;
    }
private:
    std::string chars_;
};

// Join
template<typename Container>
std::string join(const Container& input, const std::string& separator) {
    std::string result;
    bool first = true;
    for (const auto& item : input) {
        if (!first) result += separator;
        result += item;
        first = false;
    }
    return result;
}

// Replace
template<typename T>
void replace_all(T& input, const T& search, const T& replace) {
    size_t pos = 0;
    while ((pos = input.find(search, pos)) != T::npos) {
        input.replace(pos, search.length(), replace);
        pos += replace.length();
    }
}

template<typename T>
T replace_all_copy(const T& input, const T& search, const T& replace) {
    T result = input;
    replace_all(result, search, replace);
    return result;
}

template<typename T>
void replace_first(T& input, const T& search, const T& replace) {
    size_t pos = input.find(search);
    if (pos != T::npos) {
        input.replace(pos, search.length(), replace);
    }
}

template<typename T>
T replace_first_copy(const T& input, const T& search, const T& replace) {
    T result = input;
    replace_first(result, search, replace);
    return result;
}

// Token compress
struct token_compress_on_type {};
struct token_compress_off_type {};
inline token_compress_on_type token_compress_on;
inline token_compress_off_type token_compress_off;

} // namespace algorithm

// Pull common functions into boost namespace
using algorithm::to_lower;
using algorithm::to_lower_copy;
using algorithm::to_upper;
using algorithm::to_upper_copy;
using algorithm::trim;
using algorithm::trim_copy;
using algorithm::trim_left;
using algorithm::trim_left_copy;
using algorithm::trim_right;
using algorithm::trim_right_copy;
using algorithm::starts_with;
using algorithm::ends_with;
using algorithm::contains;
using algorithm::iequals;
using algorithm::split;
using algorithm::is_any_of;
using algorithm::join;
using algorithm::replace_all;
using algorithm::replace_all_copy;
using algorithm::replace_first;
using algorithm::replace_first_copy;
using algorithm::token_compress_on;
using algorithm::token_compress_off;

} // namespace boost
