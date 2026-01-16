// Stub header for gflags/gflags.h
// Provides minimal type declarations for C++ parsing

#ifndef GFLAGS_GFLAGS_H_
#define GFLAGS_GFLAGS_H_

#include <string>
#include <cstdint>

namespace google {

// Command line flag validators
typedef bool (*FlagValidator)(const char*, bool);
typedef bool (*FlagValidatorInt32)(const char*, int32_t);
typedef bool (*FlagValidatorInt64)(const char*, int64_t);
typedef bool (*FlagValidatorUInt32)(const char*, uint32_t);
typedef bool (*FlagValidatorUInt64)(const char*, uint64_t);
typedef bool (*FlagValidatorDouble)(const char*, double);
typedef bool (*FlagValidatorString)(const char*, const std::string&);

// Flag registration
bool RegisterFlagValidator(const bool*, FlagValidator) { return true; }
bool RegisterFlagValidator(const int32_t*, FlagValidatorInt32) { return true; }
bool RegisterFlagValidator(const int64_t*, FlagValidatorInt64) { return true; }
bool RegisterFlagValidator(const uint32_t*, FlagValidatorUInt32) { return true; }
bool RegisterFlagValidator(const uint64_t*, FlagValidatorUInt64) { return true; }
bool RegisterFlagValidator(const double*, FlagValidatorDouble) { return true; }
bool RegisterFlagValidator(const std::string*, FlagValidatorString) { return true; }

// Parse command line flags
uint32_t ParseCommandLineFlags(int* argc, char*** argv, bool remove_flags = true) { return 0; }
void ParseCommandLineNonHelpFlags(int* argc, char*** argv, bool remove_flags = true) {}
void HandleCommandLineHelpFlags() {}

// Flag accessor functions
void SetCommandLineOption(const char* name, const char* value) {}
std::string GetCommandLineOption(const char* name) { return ""; }
bool GetCommandLineFlagInfo(const char* name, void* info) { return false; }

// Shutdown
void ShutDownCommandLineFlags() {}

// Flag info structure
struct CommandLineFlagInfo {
    std::string name;
    std::string type;
    std::string description;
    std::string current_value;
    std::string default_value;
    std::string filename;
    bool is_default;
    bool has_validator_fn;
    int flag_ptr;
};

} // namespace google

// Import into global namespace (common usage pattern)
using google::ParseCommandLineFlags;
using google::SetCommandLineOption;

// Macros for defining flags
#define DEFINE_bool(name, val, txt) \
    namespace fLB { static const bool FLAGS_no##name = !(val); static const bool FLAGS_##name = (val); } \
    bool FLAGS_##name = (val);

#define DEFINE_int32(name, val, txt) \
    int32_t FLAGS_##name = (val);

#define DEFINE_int64(name, val, txt) \
    int64_t FLAGS_##name = (val);

#define DEFINE_uint32(name, val, txt) \
    uint32_t FLAGS_##name = (val);

#define DEFINE_uint64(name, val, txt) \
    uint64_t FLAGS_##name = (val);

#define DEFINE_double(name, val, txt) \
    double FLAGS_##name = (val);

#define DEFINE_string(name, val, txt) \
    std::string FLAGS_##name = (val);

// Macros for declaring flags (extern)
#define DECLARE_bool(name) \
    extern bool FLAGS_##name;

#define DECLARE_int32(name) \
    extern int32_t FLAGS_##name;

#define DECLARE_int64(name) \
    extern int64_t FLAGS_##name;

#define DECLARE_uint32(name) \
    extern uint32_t FLAGS_##name;

#define DECLARE_uint64(name) \
    extern uint64_t FLAGS_##name;

#define DECLARE_double(name) \
    extern double FLAGS_##name;

#define DECLARE_string(name) \
    extern std::string FLAGS_##name;

#endif // GFLAGS_GFLAGS_H_
