// Stub header for google/protobuf/message.h
// Provides minimal type declarations for C++ parsing

#ifndef GOOGLE_PROTOBUF_MESSAGE_H_
#define GOOGLE_PROTOBUF_MESSAGE_H_

#include <string>
#include <cstdint>

namespace google {
namespace protobuf {

// Forward declarations
class Descriptor;
class Reflection;
class MessageFactory;
class Arena;

// MessageLite - base class for lightweight message types
class MessageLite {
public:
    MessageLite() = default;
    virtual ~MessageLite() = default;

    // Serialization
    virtual bool SerializeToString(std::string* output) const { return true; }
    virtual bool SerializePartialToString(std::string* output) const { return true; }
    virtual bool ParseFromString(const std::string& data) { return true; }
    virtual bool ParsePartialFromString(const std::string& data) { return true; }

    virtual std::string SerializeAsString() const { return ""; }
    virtual std::string SerializePartialAsString() const { return ""; }

    // Array serialization
    virtual bool SerializeToArray(void* data, int size) const { return true; }
    virtual bool ParseFromArray(const void* data, int size) { return true; }

    // Size calculation
    virtual size_t ByteSizeLong() const { return 0; }
    int ByteSize() const { return static_cast<int>(ByteSizeLong()); }

    // Type information
    virtual std::string GetTypeName() const { return ""; }

    // Cloning
    virtual MessageLite* New() const { return nullptr; }
    virtual MessageLite* New(Arena* arena) const { return nullptr; }

    // Clear all fields
    virtual void Clear() {}

    // Check if initialized
    virtual bool IsInitialized() const { return true; }

    // Debug string
    virtual std::string InitializationErrorString() const { return ""; }
};

// Message - full-featured message class with reflection
class Message : public MessageLite {
public:
    Message() = default;
    virtual ~Message() override = default;

    // Reflection support
    virtual const Descriptor* GetDescriptor() const { return nullptr; }
    virtual const Reflection* GetReflection() const { return nullptr; }

    // Copy/merge operations
    virtual void CopyFrom(const Message& from) {}
    virtual void MergeFrom(const Message& from) {}

    // Debug output
    virtual std::string DebugString() const { return ""; }
    virtual std::string ShortDebugString() const { return ""; }
    virtual std::string Utf8DebugString() const { return ""; }

    // Serialization with output stream
    bool SerializeToOstream(std::ostream* output) const { return true; }
    bool ParseFromIstream(std::istream* input) { return true; }

    // Check for unknown fields
    virtual int SpaceUsedLong() const { return 0; }
    int SpaceUsed() const { return static_cast<int>(SpaceUsedLong()); }

    // Clone
    Message* New() const override { return nullptr; }
    Message* New(Arena* arena) const override { return nullptr; }
};

// Descriptor - describes a message type
class Descriptor {
public:
    const char* name() const { return ""; }
    const char* full_name() const { return ""; }
    int field_count() const { return 0; }
    int nested_type_count() const { return 0; }
    int enum_type_count() const { return 0; }
};

// Reflection - provides dynamic access to message fields
class Reflection {
public:
    bool HasField(const Message& message, const void* field) const { return false; }
    int FieldSize(const Message& message, const void* field) const { return 0; }
    void ClearField(Message* message, const void* field) const {}
};

// Arena - memory pool for message allocation
class Arena {
public:
    Arena() = default;
    ~Arena() = default;

    template<typename T>
    T* CreateMessage() { return new T(); }

    void Reset() {}
    uint64_t SpaceUsed() const { return 0; }
};

// MessageFactory - creates messages by type name
class MessageFactory {
public:
    virtual ~MessageFactory() = default;
    virtual const Message* GetPrototype(const Descriptor* type) { return nullptr; }
    static MessageFactory* generated_factory() { return nullptr; }
};

} // namespace protobuf
} // namespace google

#endif // GOOGLE_PROTOBUF_MESSAGE_H_
