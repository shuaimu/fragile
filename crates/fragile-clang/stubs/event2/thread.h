// Stub header for event2/thread.h (libevent threading)
// Provides minimal type declarations for C++ parsing

#ifndef EVENT2_THREAD_H_
#define EVENT2_THREAD_H_

#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif

// Threading support flags
#define EVTHREAD_USE_PTHREADS_IMPLEMENTED 1
#define EVTHREAD_USE_WINDOWS_THREADS_IMPLEMENTED 0

// Lock types
#define EVTHREAD_LOCK_API_VERSION 1
#define EVTHREAD_LOCKTYPE_RECURSIVE 1
#define EVTHREAD_LOCKTYPE_READWRITE 2

// Condition types
#define EVTHREAD_CONDITION_API_VERSION 1

// Enable threading support (returns 0 on success, -1 on failure)
int evthread_use_pthreads(void);
int evthread_use_windows_threads(void);

// Lock callbacks structure
struct evthread_lock_callbacks {
    int lock_api_version;
    unsigned supported_locktypes;
    void* (*alloc)(unsigned locktype);
    void (*free)(void* lock, unsigned locktype);
    int (*lock)(unsigned mode, void* lock);
    int (*unlock)(unsigned mode, void* lock);
};

// Condition callbacks structure
struct evthread_condition_callbacks {
    int condition_api_version;
    void* (*alloc_condition)(unsigned condtype);
    void (*free_condition)(void* cond);
    int (*signal_condition)(void* cond, int broadcast);
    int (*wait_condition)(void* cond, void* lock, const struct timeval* timeout);
};

// Set custom lock/condition callbacks
int evthread_set_lock_callbacks(const struct evthread_lock_callbacks* cbs);
int evthread_set_condition_callbacks(const struct evthread_condition_callbacks* cbs);

// Set thread ID callback
void evthread_set_id_callback(unsigned long (*id_fn)(void));

// Enable lock debugging
void evthread_enable_lock_debugging(void);

// Check if threading is enabled
int evthread_is_enabled(void);

// Make an event base thread-safe
int evthread_make_base_notifiable(struct event_base* base);

#ifdef __cplusplus
}
#endif

#endif // EVENT2_THREAD_H_
