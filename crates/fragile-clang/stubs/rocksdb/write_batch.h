// Stub header for rocksdb/write_batch.h
// Provides minimal type declarations for C++ parsing

#ifndef ROCKSDB_WRITE_BATCH_H_
#define ROCKSDB_WRITE_BATCH_H_

#include <string>
#include <cstdint>
#include <cstddef>

namespace rocksdb {

// Forward declarations
class ColumnFamilyHandle;
class Slice;
class Status;

// Slice - reference to a portion of memory
class Slice {
public:
    Slice() : data_(""), size_(0) {}
    Slice(const char* d, size_t n) : data_(d), size_(n) {}
    Slice(const std::string& s) : data_(s.data()), size_(s.size()) {}
    Slice(const char* s) : data_(s), size_(strlen(s)) {}

    const char* data() const { return data_; }
    size_t size() const { return size_; }
    bool empty() const { return size_ == 0; }

    std::string ToString() const { return std::string(data_, size_); }

private:
    const char* data_;
    size_t size_;
};

// Status - result of an operation
class Status {
public:
    Status() : code_(kOk) {}

    static Status OK() { return Status(); }
    static Status NotFound(const Slice& msg = Slice()) { return Status(kNotFound); }
    static Status Corruption(const Slice& msg = Slice()) { return Status(kCorruption); }
    static Status NotSupported(const Slice& msg = Slice()) { return Status(kNotSupported); }
    static Status InvalidArgument(const Slice& msg = Slice()) { return Status(kInvalidArgument); }
    static Status IOError(const Slice& msg = Slice()) { return Status(kIOError); }

    bool ok() const { return code_ == kOk; }
    bool IsNotFound() const { return code_ == kNotFound; }
    bool IsCorruption() const { return code_ == kCorruption; }
    bool IsIOError() const { return code_ == kIOError; }
    bool IsNotSupported() const { return code_ == kNotSupported; }
    bool IsInvalidArgument() const { return code_ == kInvalidArgument; }

    std::string ToString() const { return ""; }

private:
    enum Code {
        kOk = 0,
        kNotFound = 1,
        kCorruption = 2,
        kNotSupported = 3,
        kInvalidArgument = 4,
        kIOError = 5,
    };

    Status(Code c) : code_(c) {}
    Code code_;
};

// ColumnFamilyHandle - reference to a column family
class ColumnFamilyHandle {
public:
    virtual ~ColumnFamilyHandle() = default;
    virtual const std::string& GetName() const = 0;
    virtual uint32_t GetID() const = 0;
};

// WriteBatch - batch of writes to be atomically applied
class WriteBatch {
public:
    WriteBatch() = default;
    explicit WriteBatch(size_t reserved_bytes) {}
    ~WriteBatch() = default;

    // Store key-value pair
    Status Put(const Slice& key, const Slice& value) { return Status::OK(); }
    Status Put(ColumnFamilyHandle* column_family, const Slice& key, const Slice& value) { return Status::OK(); }

    // Delete a key
    Status Delete(const Slice& key) { return Status::OK(); }
    Status Delete(ColumnFamilyHandle* column_family, const Slice& key) { return Status::OK(); }

    // Delete a range of keys [begin, end)
    Status DeleteRange(const Slice& begin_key, const Slice& end_key) { return Status::OK(); }
    Status DeleteRange(ColumnFamilyHandle* column_family, const Slice& begin_key, const Slice& end_key) { return Status::OK(); }

    // Merge a value with existing value
    Status Merge(const Slice& key, const Slice& value) { return Status::OK(); }
    Status Merge(ColumnFamilyHandle* column_family, const Slice& key, const Slice& value) { return Status::OK(); }

    // Clear all updates
    void Clear() {}

    // Get the number of updates in the batch
    int Count() const { return 0; }

    // Get the size of the batch in bytes
    size_t GetDataSize() const { return 0; }

    // Check if batch contains no updates
    bool HasPut() const { return false; }
    bool HasDelete() const { return false; }
    bool HasMerge() const { return false; }

    // Handler interface for iterating over batch contents
    class Handler {
    public:
        virtual ~Handler() = default;
        virtual Status PutCF(uint32_t column_family_id, const Slice& key, const Slice& value) { return Status::OK(); }
        virtual Status DeleteCF(uint32_t column_family_id, const Slice& key) { return Status::OK(); }
        virtual Status MergeCF(uint32_t column_family_id, const Slice& key, const Slice& value) { return Status::OK(); }
        virtual void LogData(const Slice& blob) {}
        virtual bool Continue() { return true; }
    };

    // Iterate over batch contents
    Status Iterate(Handler* handler) const { return Status::OK(); }
};

// WriteBatchWithIndex - WriteBatch with an index for queries
class WriteBatchWithIndex {
public:
    WriteBatchWithIndex() = default;
    ~WriteBatchWithIndex() = default;

    WriteBatch* GetWriteBatch() { return &batch_; }

    Status Put(const Slice& key, const Slice& value) { return batch_.Put(key, value); }
    Status Put(ColumnFamilyHandle* cf, const Slice& key, const Slice& value) { return batch_.Put(cf, key, value); }

    Status Delete(const Slice& key) { return batch_.Delete(key); }
    Status Delete(ColumnFamilyHandle* cf, const Slice& key) { return batch_.Delete(cf, key); }

    void Clear() { batch_.Clear(); }

private:
    WriteBatch batch_;
};

} // namespace rocksdb

#endif // ROCKSDB_WRITE_BATCH_H_
