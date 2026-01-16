// Minimal lz4.h stub for fragile parsing
#ifndef _FRAGILE_LZ4_H_
#define _FRAGILE_LZ4_H_

#include "cstddef"
#include "cstdint"

#ifdef __cplusplus
extern "C" {
#endif

// LZ4 version (minimal stub)
#define LZ4_VERSION_MAJOR    1
#define LZ4_VERSION_MINOR    9
#define LZ4_VERSION_RELEASE  4

// LZ4 API visibility (no-op for stub)
#define LZ4LIB_API

// Maximum input size
#define LZ4_MAX_INPUT_SIZE        0x7E000000

// Compression level limits
#define LZ4_ACCELERATION_DEFAULT  1
#define LZ4_ACCELERATION_MAX      65537

/// Calculate maximum compressed size for input of given size
LZ4LIB_API int LZ4_compressBound(int inputSize);

/// Simple compression function
LZ4LIB_API int LZ4_compress_default(const char* src, char* dst, int srcSize, int dstCapacity);

/// Faster compression with acceleration parameter
LZ4LIB_API int LZ4_compress_fast(const char* src, char* dst, int srcSize, int dstCapacity, int acceleration);

/// Compression using pre-allocated context (heap-allocated hash table)
LZ4LIB_API int LZ4_compress_fast_extState(void* state, const char* src, char* dst, int srcSize, int dstCapacity, int acceleration);

/// Simple state-based compression (4 args: ctx, src, dst, srcSize)
LZ4LIB_API int LZ4_compress_heap(void* state, const char* src, char* dst, int srcSize);

/// Simple decompression function
LZ4LIB_API int LZ4_decompress_safe(const char* src, char* dst, int compressedSize, int dstCapacity);

/// Partial decompression (decompress up to targetOutputSize bytes)
LZ4LIB_API int LZ4_decompress_safe_partial(const char* src, char* dst, int srcSize, int targetOutputSize, int dstCapacity);

// State management for streaming/stateful compression

/// Get required size for state buffer
LZ4LIB_API int LZ4_sizeofState(void);

/// Create a new compression context
LZ4LIB_API void* LZ4_create(void);
/// Alternative name
#define LZ4_createStream LZ4_create

/// Free a compression context
LZ4LIB_API void LZ4_free(void* ctx);
/// Alternative name
#define LZ4_freeStream LZ4_free

/// Reset a compression context
LZ4LIB_API void LZ4_resetStream(void* ctx);

/// Stream compression
LZ4LIB_API int LZ4_compress_fast_continue(void* ctx, const char* src, char* dst, int srcSize, int dstCapacity, int acceleration);

/// Create decompression context
LZ4LIB_API void* LZ4_createStreamDecode(void);

/// Free decompression context
LZ4LIB_API void LZ4_freeStreamDecode(void* ctx);

/// Stream decompression
LZ4LIB_API int LZ4_decompress_safe_continue(void* ctx, const char* src, char* dst, int srcSize, int dstCapacity);

#ifdef __cplusplus
}
#endif

#endif // _FRAGILE_LZ4_H_
