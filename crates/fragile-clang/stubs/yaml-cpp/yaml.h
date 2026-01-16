// Stub header for yaml-cpp/yaml.h
#pragma once

#include <string>
#include <vector>
#include <map>
#include <stdexcept>
#include <memory>
#include <sstream>
#include <iostream>
#include <fstream>

namespace YAML {

// Forward declarations
class Node;
class Emitter;
class Parser;

// Exception types
class Exception : public std::runtime_error {
public:
    Exception(const std::string& msg) : std::runtime_error(msg) {}
};

class BadFile : public Exception {
public:
    BadFile(const std::string& msg = "bad file") : Exception(msg) {}
};

class BadConversion : public Exception {
public:
    BadConversion(const std::string& msg = "bad conversion") : Exception(msg) {}
};

class InvalidNode : public Exception {
public:
    InvalidNode(const std::string& msg = "invalid node") : Exception(msg) {}
};

class BadDereference : public Exception {
public:
    BadDereference(const std::string& msg = "bad dereference") : Exception(msg) {}
};

class KeyNotFound : public Exception {
public:
    template<typename T>
    KeyNotFound(const T&) : Exception("key not found") {}
};

class BadSubscript : public Exception {
public:
    BadSubscript(const std::string& msg = "bad subscript") : Exception(msg) {}
};

class ParserException : public Exception {
public:
    ParserException(const std::string& msg = "parser exception") : Exception(msg) {}
};

class RepresentationException : public Exception {
public:
    RepresentationException(const std::string& msg = "representation exception") : Exception(msg) {}
};

class EmitterException : public Exception {
public:
    EmitterException(const std::string& msg = "emitter exception") : Exception(msg) {}
};

// Node types
enum class NodeType {
    Undefined,
    Null,
    Scalar,
    Sequence,
    Map
};

// YAML node class
class Node {
public:
    // Constructors
    Node() : type_(NodeType::Undefined) {}
    Node(const Node& other) = default;
    Node(Node&& other) noexcept = default;

    template<typename T>
    Node(const T& value) : type_(NodeType::Scalar), scalar_value_(std::to_string(value)) {}

    Node(const char* value) : type_(NodeType::Scalar), scalar_value_(value) {}
    Node(const std::string& value) : type_(NodeType::Scalar), scalar_value_(value) {}
    Node(bool value) : type_(NodeType::Scalar), scalar_value_(value ? "true" : "false") {}
    Node(std::nullptr_t) : type_(NodeType::Null) {}

    // Assignment
    Node& operator=(const Node& other) = default;
    Node& operator=(Node&& other) noexcept = default;

    template<typename T>
    Node& operator=(const T& value) {
        type_ = NodeType::Scalar;
        scalar_value_ = std::to_string(value);
        return *this;
    }

    Node& operator=(const std::string& value) {
        type_ = NodeType::Scalar;
        scalar_value_ = value;
        return *this;
    }

    Node& operator=(const char* value) {
        type_ = NodeType::Scalar;
        scalar_value_ = value;
        return *this;
    }

    // Node type queries
    NodeType Type() const { return type_; }
    bool IsDefined() const { return type_ != NodeType::Undefined; }
    bool IsNull() const { return type_ == NodeType::Null; }
    bool IsScalar() const { return type_ == NodeType::Scalar; }
    bool IsSequence() const { return type_ == NodeType::Sequence; }
    bool IsMap() const { return type_ == NodeType::Map; }

    // Explicit bool conversion
    explicit operator bool() const { return IsDefined(); }

    // Size for sequences and maps
    std::size_t size() const {
        if (type_ == NodeType::Sequence) return sequence_.size();
        if (type_ == NodeType::Map) return map_.size();
        return 0;
    }

    // Scalar conversion
    template<typename T>
    T as() const {
        if constexpr (std::is_same_v<T, std::string>) {
            return scalar_value_;
        } else if constexpr (std::is_same_v<T, bool>) {
            return scalar_value_ == "true" || scalar_value_ == "1" || scalar_value_ == "yes";
        } else if constexpr (std::is_integral_v<T>) {
            return static_cast<T>(std::stoll(scalar_value_));
        } else if constexpr (std::is_floating_point_v<T>) {
            return static_cast<T>(std::stod(scalar_value_));
        }
        return T();
    }

    template<typename T>
    T as(const T& fallback) const {
        if (!IsDefined() || IsNull()) return fallback;
        try {
            return as<T>();
        } catch (...) {
            return fallback;
        }
    }

    // Sequence access
    Node operator[](std::size_t index) {
        if (type_ == NodeType::Undefined) {
            type_ = NodeType::Sequence;
        }
        if (type_ == NodeType::Sequence) {
            while (sequence_.size() <= index) {
                sequence_.push_back(Node());
            }
            return sequence_[index];
        }
        return Node();
    }

    const Node operator[](std::size_t index) const {
        if (type_ == NodeType::Sequence && index < sequence_.size()) {
            return sequence_[index];
        }
        return Node();
    }

    // Map access
    Node operator[](const std::string& key) {
        if (type_ == NodeType::Undefined) {
            type_ = NodeType::Map;
        }
        if (type_ == NodeType::Map) {
            return map_[key];
        }
        return Node();
    }

    const Node operator[](const std::string& key) const {
        if (type_ == NodeType::Map) {
            auto it = map_.find(key);
            if (it != map_.end()) return it->second;
        }
        return Node();
    }

    Node operator[](const char* key) { return operator[](std::string(key)); }
    const Node operator[](const char* key) const { return operator[](std::string(key)); }

    // Forward declaration for iterator
    class iterator;
    class const_iterator;

    // Iterator pair for map iteration - yaml-cpp iterators dereference to a pair-like type
    // This type must be usable as both a pair (with first/second) and as a Node
    struct iterator_value {
        Node first;   // key
        Node second;  // value

        iterator_value() = default;
        iterator_value(const std::string& k, const Node& v) : first(k), second(v) {}
        iterator_value(const Node& v) : first(), second(v) {}

        // Allow treating the value like a Node for sequences
        template<typename T>
        T as() const { return second.as<T>(); }

        template<typename T>
        T as(const T& fallback) const { return second.as<T>(fallback); }

        // Implicit conversion to Node (delegates to second)
        operator Node() const { return second; }

        // Node-like interface (delegates to second)
        bool IsDefined() const { return second.IsDefined(); }
        bool IsNull() const { return second.IsNull(); }
        bool IsScalar() const { return second.IsScalar(); }
        bool IsSequence() const { return second.IsSequence(); }
        bool IsMap() const { return second.IsMap(); }
        std::size_t size() const { return second.size(); }
        explicit operator bool() const { return second.IsDefined(); }

        // Iteration support - delegates to second (Node)
        iterator begin();
        iterator end();
        const_iterator begin() const;
        const_iterator end() const;

        // Subscript access - delegates to second (Node)
        Node operator[](std::size_t index) { return second[index]; }
        const Node operator[](std::size_t index) const { return second[index]; }
        Node operator[](const std::string& key) { return second[key]; }
        const Node operator[](const std::string& key) const { return second[key]; }
        Node operator[](const char* key) { return second[key]; }
        const Node operator[](const char* key) const { return second[key]; }
    };

    // Sequence iteration
    class iterator {
    public:
        using value_type = iterator_value;
        using reference = iterator_value;
        using pointer = iterator_value*;
        using difference_type = std::ptrdiff_t;
        using iterator_category = std::forward_iterator_tag;

        iterator() = default;
        iterator(std::vector<Node>::iterator it) : seq_it_(it), is_seq_(true) {}
        iterator(std::map<std::string, Node>::iterator it) : map_it_(it), is_seq_(false) {}

        iterator_value operator*() {
            if (is_seq_) return iterator_value(*seq_it_);
            return iterator_value(map_it_->first, map_it_->second);
        }

        // Arrow operator returns proxy that provides first/second access
        struct arrow_proxy {
            iterator_value value;
            iterator_value* operator->() { return &value; }
        };
        arrow_proxy operator->() {
            if (is_seq_) return arrow_proxy{iterator_value(*seq_it_)};
            return arrow_proxy{iterator_value(map_it_->first, map_it_->second)};
        }

        iterator& operator++() {
            if (is_seq_) ++seq_it_;
            else ++map_it_;
            return *this;
        }
        iterator operator++(int) {
            iterator tmp(*this);
            ++(*this);
            return tmp;
        }
        bool operator==(const iterator& other) const {
            if (is_seq_ != other.is_seq_) return false;
            if (is_seq_) return seq_it_ == other.seq_it_;
            return map_it_ == other.map_it_;
        }
        bool operator!=(const iterator& other) const { return !(*this == other); }

        // For map iteration
        Node first() const {
            if (is_seq_) return Node();
            return Node(map_it_->first);
        }
        Node second() const {
            if (is_seq_) return *seq_it_;
            return map_it_->second;
        }

    private:
        std::vector<Node>::iterator seq_it_;
        std::map<std::string, Node>::iterator map_it_;
        bool is_seq_ = true;
    };

    class const_iterator {
    public:
        using value_type = const iterator_value;
        using reference = const iterator_value;
        using pointer = const iterator_value*;
        using difference_type = std::ptrdiff_t;
        using iterator_category = std::forward_iterator_tag;

        const_iterator() = default;
        const_iterator(std::vector<Node>::const_iterator it) : seq_it_(it), is_seq_(true) {}
        const_iterator(std::map<std::string, Node>::const_iterator it) : map_it_(it), is_seq_(false) {}

        iterator_value operator*() const {
            if (is_seq_) return iterator_value(*seq_it_);
            return iterator_value(map_it_->first, map_it_->second);
        }

        struct arrow_proxy {
            iterator_value value;
            const iterator_value* operator->() const { return &value; }
        };
        arrow_proxy operator->() const {
            if (is_seq_) return arrow_proxy{iterator_value(*seq_it_)};
            return arrow_proxy{iterator_value(map_it_->first, map_it_->second)};
        }

        const_iterator& operator++() {
            if (is_seq_) ++seq_it_;
            else ++map_it_;
            return *this;
        }
        const_iterator operator++(int) {
            const_iterator tmp(*this);
            ++(*this);
            return tmp;
        }
        bool operator==(const const_iterator& other) const {
            if (is_seq_ != other.is_seq_) return false;
            if (is_seq_) return seq_it_ == other.seq_it_;
            return map_it_ == other.map_it_;
        }
        bool operator!=(const const_iterator& other) const { return !(*this == other); }

        Node first() const {
            if (is_seq_) return Node();
            return Node(map_it_->first);
        }
        Node second() const {
            if (is_seq_) return *seq_it_;
            return map_it_->second;
        }

    private:
        std::vector<Node>::const_iterator seq_it_;
        std::map<std::string, Node>::const_iterator map_it_;
        bool is_seq_ = true;
    };

    iterator begin() {
        if (type_ == NodeType::Sequence) return iterator(sequence_.begin());
        if (type_ == NodeType::Map) return iterator(map_.begin());
        return iterator();
    }
    iterator end() {
        if (type_ == NodeType::Sequence) return iterator(sequence_.end());
        if (type_ == NodeType::Map) return iterator(map_.end());
        return iterator();
    }
    const_iterator begin() const {
        if (type_ == NodeType::Sequence) return const_iterator(sequence_.begin());
        if (type_ == NodeType::Map) return const_iterator(map_.begin());
        return const_iterator();
    }
    const_iterator end() const {
        if (type_ == NodeType::Sequence) return const_iterator(sequence_.end());
        if (type_ == NodeType::Map) return const_iterator(map_.end());
        return const_iterator();
    }

    // Push back for sequences
    void push_back(const Node& node) {
        if (type_ == NodeType::Undefined) type_ = NodeType::Sequence;
        if (type_ == NodeType::Sequence) {
            sequence_.push_back(node);
        }
    }

    // Reset
    void reset(const Node& other = Node()) {
        *this = other;
    }

    // Scalar value
    const std::string& Scalar() const { return scalar_value_; }

    // Tag
    std::string Tag() const { return tag_; }
    void SetTag(const std::string& tag) { tag_ = tag; }

private:
    NodeType type_ = NodeType::Undefined;
    std::string scalar_value_;
    std::string tag_;
    mutable std::vector<Node> sequence_;
    mutable std::map<std::string, Node> map_;
};

// Implement iterator_value methods that depend on iterator definitions
inline Node::iterator Node::iterator_value::begin() {
    return second.begin();
}

inline Node::iterator Node::iterator_value::end() {
    return second.end();
}

inline Node::const_iterator Node::iterator_value::begin() const {
    return second.begin();
}

inline Node::const_iterator Node::iterator_value::end() const {
    return second.end();
}

// Load functions
inline Node Load(const std::string& input) {
    return Node();
}

inline Node Load(std::istream& input) {
    return Node();
}

inline Node LoadFile(const std::string& filename) {
    return Node();
}

inline std::vector<Node> LoadAll(const std::string& input) {
    return {};
}

inline std::vector<Node> LoadAll(std::istream& input) {
    return {};
}

inline std::vector<Node> LoadAllFromFile(const std::string& filename) {
    return {};
}

// Emitter manipulators
struct EMITTER_MANIP {
    int value;
};

inline EMITTER_MANIP BeginSeq = {1};
inline EMITTER_MANIP EndSeq = {2};
inline EMITTER_MANIP BeginMap = {3};
inline EMITTER_MANIP EndMap = {4};
inline EMITTER_MANIP Key = {5};
inline EMITTER_MANIP Value = {6};
inline EMITTER_MANIP Newline = {7};
inline EMITTER_MANIP Flow = {8};
inline EMITTER_MANIP Block = {9};
inline EMITTER_MANIP Auto = {10};
inline EMITTER_MANIP SingleQuoted = {11};
inline EMITTER_MANIP DoubleQuoted = {12};
inline EMITTER_MANIP Literal = {13};
inline EMITTER_MANIP Comment = {14};
inline EMITTER_MANIP Alias = {15};
inline EMITTER_MANIP Anchor = {16};

// Emitter class
class Emitter {
public:
    Emitter() = default;
    ~Emitter() = default;

    // Output
    const char* c_str() const { return output_.str().c_str(); }
    std::string str() const { return output_.str(); }
    std::size_t size() const { return output_.str().size(); }

    // State
    bool good() const { return true; }

    // Emit operations
    Emitter& operator<<(const EMITTER_MANIP&) { return *this; }
    Emitter& operator<<(const Node&) { return *this; }
    Emitter& operator<<(const char* s) { output_ << s; return *this; }
    Emitter& operator<<(const std::string& s) { output_ << s; return *this; }

    template<typename T>
    Emitter& operator<<(const T& value) {
        output_ << value;
        return *this;
    }

    // Write to stream
    Emitter& Write(std::ostream& out) {
        out << output_.str();
        return *this;
    }

    // Set options
    Emitter& SetIndent(std::size_t) { return *this; }
    Emitter& SetPreCommentIndent(std::size_t) { return *this; }
    Emitter& SetPostCommentIndent(std::size_t) { return *this; }
    Emitter& SetFloatPrecision(std::size_t) { return *this; }
    Emitter& SetDoublePrecision(std::size_t) { return *this; }
    Emitter& SetStringFormat(EMITTER_MANIP) { return *this; }
    Emitter& SetBoolFormat(EMITTER_MANIP) { return *this; }
    Emitter& SetSeqFormat(EMITTER_MANIP) { return *this; }
    Emitter& SetMapFormat(EMITTER_MANIP) { return *this; }

private:
    std::ostringstream output_;
};

// Output operator for Node
inline std::ostream& operator<<(std::ostream& out, const Node& node) {
    if (node.IsScalar()) out << node.Scalar();
    return out;
}

// Clone function
inline Node Clone(const Node& node) {
    return node;
}

// Convert template
template<typename T>
struct convert {
    static Node encode(const T& rhs) {
        return Node(rhs);
    }
    static bool decode(const Node& node, T& rhs) {
        if (!node.IsDefined()) return false;
        try {
            rhs = node.as<T>();
            return true;
        } catch (...) {
            return false;
        }
    }
};

// Mark
struct Mark {
    int pos = 0;
    int line = 0;
    int column = 0;

    Mark() = default;
    Mark(int p, int l, int c) : pos(p), line(l), column(c) {}

    bool is_null() const { return pos < 0; }
    static Mark null_mark() { return Mark(-1, -1, -1); }
};

} // namespace YAML
