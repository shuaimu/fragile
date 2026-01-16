// Minimal cxxabi.h stub for fragile parsing
// Provides C++ ABI runtime support declarations

#ifndef _CXXABI_H
#define _CXXABI_H

#include "cstddef"
#include "typeinfo"  // Required for typeid operator

namespace __cxxabiv1 {

// Type info classes for RTTI
class __class_type_info;
class __si_class_type_info;
class __vmi_class_type_info;

// Exception handling
struct __cxa_exception;
struct __cxa_eh_globals;

// Get thread-local exception globals
__cxa_eh_globals* __cxa_get_globals() noexcept;
__cxa_eh_globals* __cxa_get_globals_fast() noexcept;

// Exception allocation and throwing
void* __cxa_allocate_exception(size_t thrown_size) noexcept;
void __cxa_free_exception(void* thrown_exception) noexcept;
void __cxa_throw(void* thrown_exception, void* tinfo, void (*dest)(void*));
void* __cxa_begin_catch(void* exceptionObject) noexcept;
void __cxa_end_catch();
void __cxa_rethrow();

// Guard variables for static initialization
int __cxa_guard_acquire(long long* guard_object);
void __cxa_guard_release(long long* guard_object) noexcept;
void __cxa_guard_abort(long long* guard_object) noexcept;

// Pure virtual call handler
void __cxa_pure_virtual();

// Demangling (the main reason mako includes this)
char* __cxa_demangle(const char* mangled_name, char* output_buffer,
                     size_t* length, int* status);

// atexit handlers
int __cxa_atexit(void (*destructor)(void*), void* arg, void* dso_handle);
void __cxa_finalize(void* dso_handle);

// Thread-safe static initialization
int __cxa_thread_atexit(void (*destructor)(void*), void* arg, void* dso_handle);

} // namespace __cxxabiv1

// Expose in abi namespace as well
namespace abi = __cxxabiv1;

// C interface
extern "C" {
    char* __cxa_demangle(const char* mangled_name, char* output_buffer,
                         size_t* length, int* status);
}

#endif // _CXXABI_H
