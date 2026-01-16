// Minimal eRPC stub header for Fragile compiler
// This provides type declarations used by mako's eRPC integration

#ifndef _FRAGILE_ERPC_RPC_H
#define _FRAGILE_ERPC_RPC_H

#include <cstdint>
#include <cstddef>
#include <cstring>
#include <climits>
#include <string>
#include <functional>

namespace erpc {

// Forward declarations
class ReqHandle;
class Nexus;

// Buffer type for messages
struct Buffer {
    uint8_t* buf_{nullptr};
};

// Message buffer for request/response data
class MsgBuffer {
public:
    MsgBuffer() = default;
    MsgBuffer(Buffer buffer, size_t max_data_size, size_t max_num_pkts) {}

    uint8_t* buf_{nullptr};
    size_t max_data_size_{0};
    size_t num_pkts_{0};
    Buffer buffer_;

    void resize(size_t new_data_size, size_t new_num_pkts) {}
    uint8_t* get_buf() const { return buf_; }
    size_t get_data_size() const { return max_data_size_; }
};

// Session management event types
enum class SmEventType {
    kConnected,
    kConnectFailed,
    kDisconnected,
    kDisconnectFailed
};

// Session management error types
enum class SmErrType {
    kNoError,
    kTimeout,
    kInvalidArg
};

// Request function types
enum class ReqFuncType {
    kForeground,
    kBackground
};

// String conversions for event/error types
inline std::string sm_event_type_str(SmEventType type) {
    switch (type) {
        case SmEventType::kConnected: return "kConnected";
        case SmEventType::kConnectFailed: return "kConnectFailed";
        case SmEventType::kDisconnected: return "kDisconnected";
        case SmEventType::kDisconnectFailed: return "kDisconnectFailed";
        default: return "Unknown";
    }
}

inline std::string sm_err_type_str(SmErrType type) {
    switch (type) {
        case SmErrType::kNoError: return "kNoError";
        case SmErrType::kTimeout: return "kTimeout";
        case SmErrType::kInvalidArg: return "kInvalidArg";
        default: return "Unknown";
    }
}

// Session management handler type
using sm_handler_t = std::function<void(int, SmEventType, SmErrType, void*)>;

// Request function type (called when request received)
using erpc_req_func_t = void (*)(ReqHandle*, void*);

// Request handle - represents an incoming RPC request
class ReqHandle {
public:
    ReqHandle() = default;

    // Pre-allocated response message buffers
    MsgBuffer pre_resp_msgbuf_;
    MsgBuffer dyn_resp_msgbuf_;

    MsgBuffer* get_req_msgbuf() { return &pre_resp_msgbuf_; }
    MsgBuffer* get_pre_resp_msgbuf() { return &pre_resp_msgbuf_; }
    MsgBuffer* get_dyn_resp_msgbuf() { return &dyn_resp_msgbuf_; }
};

// Nexus - global context for eRPC
class Nexus {
public:
    Nexus(const std::string& local_uri, size_t numa_node, size_t num_bg_threads) {}
    ~Nexus() = default;

    void register_req_func(uint8_t req_type, erpc_req_func_t req_func, ReqFuncType type) {}

    static constexpr size_t kMaxRpcId = 256;
};

// Transport types
struct CTransport {
    static constexpr size_t kMaxDataPerPkt = 8192;
};

struct DpdkTransport {
    static constexpr size_t kMaxDataPerPkt = 8192;
};

struct RawTransport {
    static constexpr size_t kMaxDataPerPkt = 8192;
};

// Main Rpc class - templated on transport type
template <class TTr>
class Rpc {
public:
    static constexpr size_t kMaxMsgSize = 1 << 20;  // 1 MB

    Rpc(Nexus* nexus, void* context, uint8_t rpc_id, sm_handler_t sm_handler, uint8_t phy_port = 0) {}
    ~Rpc() = default;

    // Message buffer management
    MsgBuffer alloc_msg_buffer(size_t max_data_size) { return MsgBuffer(); }
    static void resize_msg_buffer(MsgBuffer* msg_buffer, size_t new_data_size) {}
    void free_msg_buffer(MsgBuffer msg_buffer) {}

    // Session management
    int create_session(const std::string& remote_uri, uint8_t rem_rpc_id) { return 0; }
    int destroy_session(int session_num) { return 0; }
    bool is_connected(int session_num) const { return true; }

    // Request/response handling
    void enqueue_request(int session_num, uint8_t req_type, MsgBuffer* req_msgbuf,
                        MsgBuffer* resp_msgbuf, void (*cont_func)(void*, void*), void* tag) {}
    void enqueue_response(ReqHandle* req_handle, MsgBuffer* resp_msgbuf) {}

    // Event loop
    void run_event_loop(size_t timeout_ms) {}
    void run_event_loop_once() {}

    // Getters
    uint8_t get_rpc_id() const { return 0; }
    void* get_context() const { return nullptr; }
};

}  // namespace erpc

#endif  // _FRAGILE_ERPC_RPC_H
