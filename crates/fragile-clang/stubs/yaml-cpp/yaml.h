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

// Internal storage for Node to allow self-referential types
struct NodeData {
    NodeType type = NodeType::Undefined;
    std::string scalar_value;
    std::string tag;
    std::shared_ptr<std::vector<std::shared_ptr<NodeData>>> sequence;
    std::shared_ptr<std::map<std::string, std::shared_ptr<NodeData>>> map;

    NodeData() = default;
    NodeData(NodeType t) : type(t) {}
    NodeData(const std::string& s) : type(NodeType::Scalar), scalar_value(s) {}
};

// YAML node class
class Node {
public:
    // Constructors
    Node() : data_(std::make_shared<NodeData>()) {}
    Node(const Node& other) = default;
    Node(Node&& other) noexcept = default;

    template<typename T>
    Node(const T& value) : data_(std::make_shared<NodeData>(std::to_string(value))) {}

    Node(const char* value) : data_(std::make_shared<NodeData>(value)) {}
    Node(const std::string& value) : data_(std::make_shared<NodeData>(value)) {}
    Node(bool value) : data_(std::make_shared<NodeData>(value ? "true" : "false")) {}
    Node(std::nullptr_t) : data_(std::make_shared<NodeData>(NodeType::Null)) {}

    // Assignment
    Node& operator=(const Node& other) = default;
    Node& operator=(Node&& other) noexcept = default;

    template<typename T>
    Node& operator=(const T& value) {
        data_->type = NodeType::Scalar;
        data_->scalar_value = std::to_string(value);
        return *this;
    }

    Node& operator=(const std::string& value) {
        data_->type = NodeType::Scalar;
        data_->scalar_value = value;
        return *this;
    }

    Node& operator=(const char* value) {
        data_->type = NodeType::Scalar;
        data_->scalar_value = value;
        return *this;
    }

    // Node type queries
    NodeType Type() const { return data_->type; }
    bool IsDefined() const { return data_->type != NodeType::Undefined; }
    bool IsNull() const { return data_->type == NodeType::Null; }
    bool IsScalar() const { return data_->type == NodeType::Scalar; }
    bool IsSequence() const { return data_->type == NodeType::Sequence; }
    bool IsMap() const { return data_->type == NodeType::Map; }

    // Explicit bool conversion
    explicit operator bool() const { return IsDefined(); }

    // Size for sequences and maps
    std::size_t size() const {
        if (data_->type == NodeType::Sequence && data_->sequence)
            return data_->sequence->size();
        if (data_->type == NodeType::Map && data_->map)
            return data_->map->size();
        return 0;
    }

    // Scalar conversion
    template<typename T>
    T as() const {
        if constexpr (std::is_same_v<T, std::string>) {
            return data_->scalar_value;
        } else if constexpr (std::is_same_v<T, bool>) {
            return data_->scalar_value == "true" || data_->scalar_value == "1" || data_->scalar_value == "yes";
        } else if constexpr (std::is_integral_v<T>) {
            return static_cast<T>(std::stoll(data_->scalar_value));
        } else if constexpr (std::is_floating_point_v<T>) {
            return static_cast<T>(std::stod(data_->scalar_value));
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
        if (data_->type == NodeType::Undefined) {
            data_->type = NodeType::Sequence;
            data_->sequence = std::make_shared<std::vector<std::shared_ptr<NodeData>>>();
        }
        if (data_->type == NodeType::Sequence && data_->sequence) {
            while (data_->sequence->size() <= index) {
                data_->sequence->push_back(std::make_shared<NodeData>());
            }
            Node result;
            result.data_ = (*data_->sequence)[index];
            return result;
        }
        return Node();
    }

    const Node operator[](std::size_t index) const {
        if (data_->type == NodeType::Sequence && data_->sequence && index < data_->sequence->size()) {
            Node result;
            result.data_ = (*data_->sequence)[index];
            return result;
        }
        return Node();
    }

    // Map access
    Node operator[](const std::string& key) {
        if (data_->type == NodeType::Undefined) {
            data_->type = NodeType::Map;
            data_->map = std::make_shared<std::map<std::string, std::shared_ptr<NodeData>>>();
        }
        if (data_->type == NodeType::Map && data_->map) {
            auto& entry = (*data_->map)[key];
            if (!entry) entry = std::make_shared<NodeData>();
            Node result;
            result.data_ = entry;
            return result;
        }
        return Node();
    }

    const Node operator[](const std::string& key) const {
        if (data_->type == NodeType::Map && data_->map) {
            auto it = data_->map->find(key);
            if (it != data_->map->end()) {
                Node result;
                result.data_ = it->second;
                return result;
            }
        }
        return Node();
    }

    Node operator[](const char* key) { return operator[](std::string(key)); }
    const Node operator[](const char* key) const { return operator[](std::string(key)); }

    // Forward declare iterator types for iterator_value
    class iterator;
    class const_iterator;

    // Iterator value type for dereferencing iterators
    struct iterator_value {
        Node first;   // key
        Node second;  // value

        iterator_value() = default;
        iterator_value(const Node& k, const Node& v) : first(k), second(v) {}
        iterator_value(const std::string& k, const Node& v) : first(k), second(v) {}
        explicit iterator_value(const Node& v) : first(), second(v) {}

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
        iterator begin() { return second.begin(); }
        iterator end() { return second.end(); }
        const_iterator begin() const { return second.begin(); }
        const_iterator end() const { return second.end(); }

        // Subscript access - delegates to second (Node)
        Node operator[](std::size_t index) { return second[index]; }
        const Node operator[](std::size_t index) const { return second[index]; }
        Node operator[](const std::string& key) { return second[key]; }
        const Node operator[](const std::string& key) const { return second[key]; }
        Node operator[](const char* key) { return second[key]; }
        const Node operator[](const char* key) const { return second[key]; }
    };

    // Sequence iteration - simplified using indices
    class iterator {
    public:
        using value_type = iterator_value;
        using reference = iterator_value;
        using pointer = iterator_value*;
        using difference_type = std::ptrdiff_t;
        using iterator_category = std::forward_iterator_tag;

        iterator() = default;
        iterator(Node* node, std::size_t index, bool is_map)
            : node_(node), index_(index), is_map_(is_map) {}

        iterator_value operator*();

        // Arrow operator returns proxy that provides first/second access
        struct arrow_proxy {
            iterator_value value;
            iterator_value* operator->() { return &value; }
        };
        arrow_proxy operator->() { return arrow_proxy{operator*()}; }

        iterator& operator++() { ++index_; return *this; }
        iterator operator++(int) {
            iterator tmp(*this);
            ++(*this);
            return tmp;
        }
        bool operator==(const iterator& other) const {
            return node_ == other.node_ && index_ == other.index_;
        }
        bool operator!=(const iterator& other) const { return !(*this == other); }

    private:
        Node* node_ = nullptr;
        std::size_t index_ = 0;
        bool is_map_ = false;
    };

    class const_iterator {
    public:
        using value_type = const iterator_value;
        using reference = const iterator_value;
        using pointer = const iterator_value*;
        using difference_type = std::ptrdiff_t;
        using iterator_category = std::forward_iterator_tag;

        const_iterator() = default;
        const_iterator(const Node* node, std::size_t index, bool is_map)
            : node_(node), index_(index), is_map_(is_map) {}

        iterator_value operator*() const;

        struct arrow_proxy {
            iterator_value value;
            const iterator_value* operator->() const { return &value; }
        };
        arrow_proxy operator->() const { return arrow_proxy{operator*()}; }

        const_iterator& operator++() { ++index_; return *this; }
        const_iterator operator++(int) {
            const_iterator tmp(*this);
            ++(*this);
            return tmp;
        }
        bool operator==(const const_iterator& other) const {
            return node_ == other.node_ && index_ == other.index_;
        }
        bool operator!=(const const_iterator& other) const { return !(*this == other); }

    private:
        const Node* node_ = nullptr;
        std::size_t index_ = 0;
        bool is_map_ = false;
    };

    iterator begin() {
        if (data_->type == NodeType::Sequence || data_->type == NodeType::Map)
            return iterator(this, 0, data_->type == NodeType::Map);
        return iterator(this, 0, false);
    }
    iterator end() {
        return iterator(this, size(), data_->type == NodeType::Map);
    }
    const_iterator begin() const {
        if (data_->type == NodeType::Sequence || data_->type == NodeType::Map)
            return const_iterator(this, 0, data_->type == NodeType::Map);
        return const_iterator(this, 0, false);
    }
    const_iterator end() const {
        return const_iterator(this, size(), data_->type == NodeType::Map);
    }

    // Push back for sequences
    void push_back(const Node& node) {
        if (data_->type == NodeType::Undefined) {
            data_->type = NodeType::Sequence;
            data_->sequence = std::make_shared<std::vector<std::shared_ptr<NodeData>>>();
        }
        if (data_->type == NodeType::Sequence && data_->sequence) {
            data_->sequence->push_back(node.data_);
        }
    }

    // Reset
    void reset(const Node& other = Node()) {
        *this = other;
    }

    // Scalar value
    const std::string& Scalar() const { return data_->scalar_value; }

    // Tag
    std::string Tag() const { return data_->tag; }
    void SetTag(const std::string& tag) { data_->tag = tag; }

    // Get map keys (for iteration support)
    std::vector<std::string> getMapKeys() const {
        std::vector<std::string> keys;
        if (data_->type == NodeType::Map && data_->map) {
            for (const auto& pair : *data_->map) {
                keys.push_back(pair.first);
            }
        }
        return keys;
    }

private:
    std::shared_ptr<NodeData> data_;

    friend class iterator;
    friend class const_iterator;
};

// Implement iterator operator* (needs complete Node type)
inline Node::iterator_value Node::iterator::operator*() {
    if (!node_) return iterator_value();
    if (is_map_) {
        auto keys = node_->getMapKeys();
        if (index_ < keys.size()) {
            return iterator_value(keys[index_], (*node_)[keys[index_]]);
        }
    } else {
        return iterator_value((*node_)[index_]);
    }
    return iterator_value();
}

inline Node::iterator_value Node::const_iterator::operator*() const {
    if (!node_) return iterator_value();
    if (is_map_) {
        auto keys = node_->getMapKeys();
        if (index_ < keys.size()) {
            return iterator_value(keys[index_], (*node_)[keys[index_]]);
        }
    } else {
        return iterator_value((*node_)[index_]);
    }
    return iterator_value();
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
