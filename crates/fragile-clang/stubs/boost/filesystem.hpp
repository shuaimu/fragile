// Stub header for boost/filesystem.hpp
#pragma once

#include <string>
#include <vector>
#include <cstdint>
#include <chrono>
#include <system_error>

namespace boost {
namespace filesystem {

// Path class
class path {
public:
    path() = default;
    path(const std::string& p) : path_(p) {}
    path(const char* p) : path_(p) {}
    path(const path&) = default;
    path(path&&) = default;
    path& operator=(const path&) = default;
    path& operator=(path&&) = default;

    // Concatenation
    path& operator/=(const path& p) {
        if (!path_.empty() && path_.back() != '/') path_ += '/';
        path_ += p.path_;
        return *this;
    }

    path operator/(const path& p) const {
        path result(*this);
        result /= p;
        return result;
    }

    // Modifiers
    path& remove_filename() {
        size_t pos = path_.rfind('/');
        if (pos != std::string::npos) {
            path_.erase(pos);
        }
        return *this;
    }

    path& replace_extension(const path& new_ext = path()) {
        size_t dot = path_.rfind('.');
        size_t slash = path_.rfind('/');
        if (dot != std::string::npos && (slash == std::string::npos || dot > slash)) {
            path_.erase(dot);
        }
        if (!new_ext.empty()) {
            if (new_ext.path_[0] != '.') path_ += '.';
            path_ += new_ext.path_;
        }
        return *this;
    }

    // Decomposition
    path root_path() const { return path_.substr(0, 1) == "/" ? path("/") : path(); }
    path root_name() const { return path(); }
    path root_directory() const { return path_.substr(0, 1) == "/" ? path("/") : path(); }
    path relative_path() const { return path_.substr(0, 1) == "/" ? path(path_.substr(1)) : *this; }
    path parent_path() const {
        size_t pos = path_.rfind('/');
        return pos != std::string::npos && pos > 0 ? path(path_.substr(0, pos)) : path();
    }
    path filename() const {
        size_t pos = path_.rfind('/');
        return path(pos != std::string::npos ? path_.substr(pos + 1) : path_);
    }
    path stem() const {
        std::string fn = filename().path_;
        size_t dot = fn.rfind('.');
        return path(dot != std::string::npos && dot > 0 ? fn.substr(0, dot) : fn);
    }
    path extension() const {
        std::string fn = filename().path_;
        size_t dot = fn.rfind('.');
        return path(dot != std::string::npos && dot > 0 ? fn.substr(dot) : "");
    }

    // Queries
    bool empty() const { return path_.empty(); }
    bool has_root_path() const { return !root_path().empty(); }
    bool has_root_name() const { return !root_name().empty(); }
    bool has_root_directory() const { return !root_directory().empty(); }
    bool has_relative_path() const { return !relative_path().empty(); }
    bool has_parent_path() const { return !parent_path().empty(); }
    bool has_filename() const { return !filename().empty(); }
    bool has_stem() const { return !stem().empty(); }
    bool has_extension() const { return !extension().empty(); }
    bool is_absolute() const { return !path_.empty() && path_[0] == '/'; }
    bool is_relative() const { return !is_absolute(); }

    // Conversions
    const std::string& string() const { return path_; }
    const std::string& native() const { return path_; }
    const char* c_str() const { return path_.c_str(); }

    // Comparison
    bool operator==(const path& other) const { return path_ == other.path_; }
    bool operator!=(const path& other) const { return path_ != other.path_; }
    bool operator<(const path& other) const { return path_ < other.path_; }

    // Iterator support (simplified)
    class iterator {
    public:
        using value_type = path;
        using reference = const path&;
        using pointer = const path*;
        using difference_type = std::ptrdiff_t;
        using iterator_category = std::bidirectional_iterator_tag;

        iterator() = default;
        iterator(const path* p, size_t pos) : p_(p), pos_(pos) {}

        reference operator*() const { return current_; }
        iterator& operator++() { advance(); return *this; }
        iterator operator++(int) { iterator tmp(*this); advance(); return tmp; }
        bool operator==(const iterator& other) const { return pos_ == other.pos_; }
        bool operator!=(const iterator& other) const { return pos_ != other.pos_; }

    private:
        void advance() { pos_ = std::string::npos; }
        const path* p_ = nullptr;
        size_t pos_ = std::string::npos;
        path current_;
    };

    iterator begin() const { return iterator(this, 0); }
    iterator end() const { return iterator(this, std::string::npos); }

private:
    std::string path_;
};

inline path operator/(const path& lhs, const path& rhs) {
    return path(lhs) /= rhs;
}

// File status
enum class file_type {
    none,
    not_found,
    regular,
    directory,
    symlink,
    block,
    character,
    fifo,
    socket,
    unknown
};

enum class perms {
    none = 0,
    owner_read = 0400,
    owner_write = 0200,
    owner_exec = 0100,
    owner_all = 0700,
    group_read = 040,
    group_write = 020,
    group_exec = 010,
    group_all = 070,
    others_read = 04,
    others_write = 02,
    others_exec = 01,
    others_all = 07,
    all = 0777,
    set_uid = 04000,
    set_gid = 02000,
    sticky_bit = 01000,
    mask = 07777,
    unknown = 0xFFFF
};

class file_status {
public:
    file_status() : type_(file_type::none), perms_(perms::unknown) {}
    file_status(file_type type, perms p = perms::unknown) : type_(type), perms_(p) {}

    file_type type() const { return type_; }
    void type(file_type t) { type_ = t; }
    perms permissions() const { return perms_; }
    void permissions(perms p) { perms_ = p; }

private:
    file_type type_;
    perms perms_;
};

// Operations (stubs)
inline bool exists(const path&) { return true; }
inline bool exists(const path&, std::error_code&) { return true; }
inline bool is_directory(const path&) { return false; }
inline bool is_directory(const path&, std::error_code&) { return false; }
inline bool is_regular_file(const path&) { return true; }
inline bool is_regular_file(const path&, std::error_code&) { return true; }
inline bool is_symlink(const path&) { return false; }
inline bool is_symlink(const path&, std::error_code&) { return false; }
inline bool is_empty(const path&) { return false; }
inline bool is_empty(const path&, std::error_code&) { return false; }

inline std::uintmax_t file_size(const path&) { return 0; }
inline std::uintmax_t file_size(const path&, std::error_code&) { return 0; }

inline file_status status(const path&) { return file_status(file_type::regular); }
inline file_status status(const path&, std::error_code&) { return file_status(file_type::regular); }
inline file_status symlink_status(const path&) { return file_status(file_type::regular); }
inline file_status symlink_status(const path&, std::error_code&) { return file_status(file_type::regular); }

inline bool create_directory(const path&) { return true; }
inline bool create_directory(const path&, std::error_code&) { return true; }
inline bool create_directories(const path&) { return true; }
inline bool create_directories(const path&, std::error_code&) { return true; }

inline bool remove(const path&) { return true; }
inline bool remove(const path&, std::error_code&) { return true; }
inline std::uintmax_t remove_all(const path&) { return 0; }
inline std::uintmax_t remove_all(const path&, std::error_code&) { return 0; }

inline void rename(const path&, const path&) {}
inline void rename(const path&, const path&, std::error_code&) {}

inline void copy(const path&, const path&) {}
inline void copy(const path&, const path&, std::error_code&) {}
inline void copy_file(const path&, const path&) {}
inline void copy_file(const path&, const path&, std::error_code&) {}

inline path current_path() { return path("."); }
inline path current_path(std::error_code&) { return path("."); }
inline void current_path(const path&) {}
inline void current_path(const path&, std::error_code&) {}

inline path absolute(const path& p) { return p; }
inline path absolute(const path& p, std::error_code&) { return p; }
inline path canonical(const path& p) { return p; }
inline path canonical(const path& p, std::error_code&) { return p; }

// Directory iterator (simplified stub)
class directory_entry {
public:
    directory_entry() = default;
    directory_entry(const path& p) : path_(p) {}
    const class path& path() const { return path_; }
    bool is_directory() const { return false; }
    bool is_regular_file() const { return true; }
    bool is_symlink() const { return false; }
private:
    class path path_;
};

class directory_iterator {
public:
    using value_type = directory_entry;
    using reference = const directory_entry&;
    using pointer = const directory_entry*;
    using difference_type = std::ptrdiff_t;
    using iterator_category = std::input_iterator_tag;

    directory_iterator() = default;
    directory_iterator(const path&) {}
    directory_iterator(const path&, std::error_code&) {}

    reference operator*() const { return entry_; }
    pointer operator->() const { return &entry_; }
    directory_iterator& operator++() { return *this; }
    directory_iterator operator++(int) { return *this; }

    bool operator==(const directory_iterator& other) const { return true; }
    bool operator!=(const directory_iterator& other) const { return false; }

private:
    directory_entry entry_;
};

inline directory_iterator begin(directory_iterator iter) { return iter; }
inline directory_iterator end(directory_iterator) { return directory_iterator(); }

class recursive_directory_iterator {
public:
    using value_type = directory_entry;
    using reference = const directory_entry&;
    using pointer = const directory_entry*;
    using difference_type = std::ptrdiff_t;
    using iterator_category = std::input_iterator_tag;

    recursive_directory_iterator() = default;
    recursive_directory_iterator(const path&) {}
    recursive_directory_iterator(const path&, std::error_code&) {}

    reference operator*() const { return entry_; }
    pointer operator->() const { return &entry_; }
    recursive_directory_iterator& operator++() { return *this; }
    recursive_directory_iterator operator++(int) { return *this; }

    bool operator==(const recursive_directory_iterator& other) const { return true; }
    bool operator!=(const recursive_directory_iterator& other) const { return false; }

    int depth() const { return 0; }
    void pop() {}
    void disable_recursion_pending() {}

private:
    directory_entry entry_;
};

inline recursive_directory_iterator begin(recursive_directory_iterator iter) { return iter; }
inline recursive_directory_iterator end(recursive_directory_iterator) { return recursive_directory_iterator(); }

// Filesystem error
class filesystem_error : public std::system_error {
public:
    filesystem_error(const std::string& what_arg, std::error_code ec)
        : std::system_error(ec, what_arg) {}
    filesystem_error(const std::string& what_arg, const path& p1, std::error_code ec)
        : std::system_error(ec, what_arg), path1_(p1) {}
    filesystem_error(const std::string& what_arg, const path& p1, const path& p2, std::error_code ec)
        : std::system_error(ec, what_arg), path1_(p1), path2_(p2) {}

    const path& path1() const { return path1_; }
    const path& path2() const { return path2_; }

private:
    path path1_, path2_;
};

} // namespace filesystem
} // namespace boost
