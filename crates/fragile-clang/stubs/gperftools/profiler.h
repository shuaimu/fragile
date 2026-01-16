// Stub header for gperftools/profiler.h
// Provides minimal type declarations for C++ parsing

#ifndef GPERFTOOLS_PROFILER_H_
#define GPERFTOOLS_PROFILER_H_

#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif

// Options for ProfilerStart
struct ProfilerOptions {
    // Filter function: return true to include, false to exclude
    int (*filter_in_thread)(void* arg);
    void* filter_in_thread_arg;
};

// Start profiler, writing samples to filename
// Returns non-zero on success
int ProfilerStart(const char* filename);

// Start profiler with options
int ProfilerStartWithOptions(const char* filename, const struct ProfilerOptions* options);

// Stop profiler
void ProfilerStop(void);

// Flush profile data (call during long-running programs)
void ProfilerFlush(void);

// Enable/disable profiling for current thread
void ProfilerEnable(void);
void ProfilerDisable(void);

// Check if profiling is enabled
int ProfilingIsEnabledForAllThreads(void);

// Register thread
void ProfilerRegisterThread(void);

// Get current profiler state
struct ProfilerState {
    int enabled;           // Is profiling enabled?
    int start_time;        // Start time
    char* profile_name;    // Profile output filename
    int samples_gathered;  // Number of samples collected
};

void ProfilerGetCurrentState(struct ProfilerState* state);

#ifdef __cplusplus
}
#endif

// C++ wrapper class
#ifdef __cplusplus
class ProfilerScope {
public:
    ProfilerScope(const char* filename) { ProfilerStart(filename); }
    ~ProfilerScope() { ProfilerStop(); }
};
#endif

#endif // GPERFTOOLS_PROFILER_H_
