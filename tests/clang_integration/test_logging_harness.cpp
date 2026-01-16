// test_logging_harness.cpp - M6.6c logging framework tests
//
// This file tests a simplified logging framework inspired by mako's rrr::Log.
// It demonstrates:
// - Variadic functions (va_list, va_start, va_end)
// - pthread_mutex_t for thread safety
// - Log levels (FATAL, ERROR, WARN, INFO, DEBUG)
// - sprintf/vsprintf for formatting
// - Static class members

#include <cstdio>
#include <cstdarg>
#include <cstring>
#include <vector>
#include <pthread.h>

// ============================================================
// Test framework (inline version of unittest_minimal)
// ============================================================

namespace test {

class TestCase {
    const char* group_;
    const char* name_;
    int failures_;
public:
    TestCase(const char* group, const char* name)
        : group_(group), name_(name), failures_(0) {}
    virtual ~TestCase() {}
    virtual void run() = 0;
    const char* group() const { return group_; }
    const char* name() const { return name_; }
    void fail() { failures_++; }
    void reset() { failures_ = 0; }
    int failures() const { return failures_; }
};

class TestMgr {
    std::vector<TestCase*> tests_;
    static TestMgr* instance_;
    TestMgr() : tests_() {}
public:
    static TestMgr* instance() {
        if (instance_ == nullptr) {
            instance_ = new TestMgr();
        }
        return instance_;
    }
    void reg(TestCase* t) { tests_.push_back(t); }
    int run_all() {
        int total_failures = 0;
        int passed = 0;
        printf("Running %zu logging tests...\n", tests_.size());
        for (auto& t : tests_) {
            t->reset();
            printf("  [RUN] %s/%s\n", t->group(), t->name());
            t->run();
            if (t->failures() == 0) {
                printf("  [PASS] %s/%s\n", t->group(), t->name());
                passed++;
            } else {
                printf("  [FAIL] %s/%s (%d failures)\n",
                    t->group(), t->name(), t->failures());
                total_failures += t->failures();
            }
        }
        printf("\nResults: %d/%zu passed", passed, tests_.size());
        if (total_failures > 0) {
            printf(", %d failures\n", total_failures);
        } else {
            printf("\n");
        }
        return total_failures;
    }
    size_t test_count() const { return tests_.size(); }
};

TestMgr* TestMgr::instance_ = nullptr;

} // namespace test

// ============================================================
// Simplified Log class (inspired by rrr::Log)
// ============================================================

namespace rrr {

class Log {
    static int level_s;
    static pthread_mutex_t mutex_s;

    // Buffer for capturing output (for testing)
    static char output_buf_[4096];
    static size_t output_len_;

    // Internal variadic helper
    static void log_v(int level, const char* fmt, va_list args) {
        static const char* level_names[] = { "FATAL", "ERROR", "WARN", "INFO", "DEBUG" };

        if (level > level_s) {
            return;  // Filter by level
        }

        pthread_mutex_lock(&mutex_s);

        // Format the message
        char msg[1024];
        int msg_len = vsprintf(msg, fmt, args);

        // Build full log line: [LEVEL] message
        char line[1100];
        int line_len = sprintf(line, "[%s] %s\n", level_names[level], msg);

        // Append to output buffer
        if (output_len_ + line_len < sizeof(output_buf_)) {
            strcpy(output_buf_ + output_len_, line);
            output_len_ += line_len;
        }

        pthread_mutex_unlock(&mutex_s);
    }

public:
    enum {
        FATAL = 0, ERROR = 1, WARN = 2, INFO = 3, DEBUG = 4
    };

    // Set log level (thread-safe)
    static void set_level(int level) {
        pthread_mutex_lock(&mutex_s);
        level_s = level;
        pthread_mutex_unlock(&mutex_s);
    }

    // Get current level
    static int get_level() {
        pthread_mutex_lock(&mutex_s);
        int level = level_s;
        pthread_mutex_unlock(&mutex_s);
        return level;
    }

    // Clear output buffer
    static void clear_output() {
        pthread_mutex_lock(&mutex_s);
        output_buf_[0] = '\0';
        output_len_ = 0;
        pthread_mutex_unlock(&mutex_s);
    }

    // Get output buffer
    static const char* get_output() {
        return output_buf_;
    }

    // Check if output contains a string
    static bool output_contains(const char* str) {
        return strstr(output_buf_, str) != nullptr;
    }

    // Variadic logging functions
    static void fatal(const char* fmt, ...) {
        va_list args;
        va_start(args, fmt);
        log_v(FATAL, fmt, args);
        va_end(args);
    }

    static void error(const char* fmt, ...) {
        va_list args;
        va_start(args, fmt);
        log_v(ERROR, fmt, args);
        va_end(args);
    }

    static void warn(const char* fmt, ...) {
        va_list args;
        va_start(args, fmt);
        log_v(WARN, fmt, args);
        va_end(args);
    }

    static void info(const char* fmt, ...) {
        va_list args;
        va_start(args, fmt);
        log_v(INFO, fmt, args);
        va_end(args);
    }

    static void debug(const char* fmt, ...) {
        va_list args;
        va_start(args, fmt);
        log_v(DEBUG, fmt, args);
        va_end(args);
    }

    // Generic log with level
    static void log(int level, const char* fmt, ...) {
        va_list args;
        va_start(args, fmt);
        log_v(level, fmt, args);
        va_end(args);
    }
};

// Static member initialization
int Log::level_s = Log::DEBUG;
pthread_mutex_t Log::mutex_s = PTHREAD_MUTEX_INITIALIZER;
char Log::output_buf_[4096] = {0};
size_t Log::output_len_ = 0;

} // namespace rrr

// ============================================================
// Test Cases
// ============================================================

class TestLogBasicLevels : public test::TestCase {
public:
    TestLogBasicLevels() : TestCase("logging", "basic_levels") {}
    void run() override {
        rrr::Log::set_level(rrr::Log::DEBUG);
        rrr::Log::clear_output();

        // Test all log levels
        rrr::Log::fatal("fatal message");
        rrr::Log::error("error message");
        rrr::Log::warn("warn message");
        rrr::Log::info("info message");
        rrr::Log::debug("debug message");

        // Verify all messages appear
        if (!rrr::Log::output_contains("[FATAL] fatal message")) {
            printf("    FAIL: FATAL message not found\n");
            fail();
        }
        if (!rrr::Log::output_contains("[ERROR] error message")) {
            printf("    FAIL: ERROR message not found\n");
            fail();
        }
        if (!rrr::Log::output_contains("[WARN] warn message")) {
            printf("    FAIL: WARN message not found\n");
            fail();
        }
        if (!rrr::Log::output_contains("[INFO] info message")) {
            printf("    FAIL: INFO message not found\n");
            fail();
        }
        if (!rrr::Log::output_contains("[DEBUG] debug message")) {
            printf("    FAIL: DEBUG message not found\n");
            fail();
        }
    }
};

class TestLogFiltering : public test::TestCase {
public:
    TestLogFiltering() : TestCase("logging", "filtering") {}
    void run() override {
        // Set level to WARN - should only see FATAL, ERROR, WARN
        rrr::Log::set_level(rrr::Log::WARN);
        rrr::Log::clear_output();

        rrr::Log::fatal("should appear");
        rrr::Log::error("should appear");
        rrr::Log::warn("should appear");
        rrr::Log::info("should NOT appear");
        rrr::Log::debug("should NOT appear");

        // Check that FATAL, ERROR, WARN appear
        if (!rrr::Log::output_contains("[FATAL]")) {
            printf("    FAIL: FATAL should appear at WARN level\n");
            fail();
        }
        if (!rrr::Log::output_contains("[ERROR]")) {
            printf("    FAIL: ERROR should appear at WARN level\n");
            fail();
        }
        if (!rrr::Log::output_contains("[WARN]")) {
            printf("    FAIL: WARN should appear at WARN level\n");
            fail();
        }

        // Check that INFO and DEBUG are filtered out
        if (rrr::Log::output_contains("[INFO]")) {
            printf("    FAIL: INFO should be filtered at WARN level\n");
            fail();
        }
        if (rrr::Log::output_contains("[DEBUG]")) {
            printf("    FAIL: DEBUG should be filtered at WARN level\n");
            fail();
        }

        // Reset level
        rrr::Log::set_level(rrr::Log::DEBUG);
    }
};

class TestLogFormat : public test::TestCase {
public:
    TestLogFormat() : TestCase("logging", "format") {}
    void run() override {
        rrr::Log::set_level(rrr::Log::DEBUG);
        rrr::Log::clear_output();

        // Test format strings with various types
        rrr::Log::info("integer: %d", 42);
        rrr::Log::info("string: %s", "hello");
        rrr::Log::info("multiple: %d %s %d", 1, "two", 3);

        if (!rrr::Log::output_contains("integer: 42")) {
            printf("    FAIL: integer format failed\n");
            fail();
        }
        if (!rrr::Log::output_contains("string: hello")) {
            printf("    FAIL: string format failed\n");
            fail();
        }
        if (!rrr::Log::output_contains("multiple: 1 two 3")) {
            printf("    FAIL: multiple format failed\n");
            fail();
        }
    }
};

class TestLogGeneric : public test::TestCase {
public:
    TestLogGeneric() : TestCase("logging", "generic_log") {}
    void run() override {
        rrr::Log::set_level(rrr::Log::DEBUG);
        rrr::Log::clear_output();

        // Test generic log function with level parameter
        rrr::Log::log(rrr::Log::INFO, "generic info %d", 100);
        rrr::Log::log(rrr::Log::ERROR, "generic error %s", "test");

        if (!rrr::Log::output_contains("[INFO] generic info 100")) {
            printf("    FAIL: generic INFO log failed\n");
            fail();
        }
        if (!rrr::Log::output_contains("[ERROR] generic error test")) {
            printf("    FAIL: generic ERROR log failed\n");
            fail();
        }
    }
};

class TestLogLevelConfig : public test::TestCase {
public:
    TestLogLevelConfig() : TestCase("logging", "level_config") {}
    void run() override {
        // Test set_level and get_level
        int original = rrr::Log::get_level();

        rrr::Log::set_level(rrr::Log::ERROR);
        if (rrr::Log::get_level() != rrr::Log::ERROR) {
            printf("    FAIL: set_level(ERROR) didn't work\n");
            fail();
        }

        rrr::Log::set_level(rrr::Log::FATAL);
        if (rrr::Log::get_level() != rrr::Log::FATAL) {
            printf("    FAIL: set_level(FATAL) didn't work\n");
            fail();
        }

        // Only FATAL should appear
        rrr::Log::clear_output();
        rrr::Log::fatal("only this");
        rrr::Log::error("not this");

        if (!rrr::Log::output_contains("[FATAL]")) {
            printf("    FAIL: FATAL should appear\n");
            fail();
        }
        if (rrr::Log::output_contains("[ERROR]")) {
            printf("    FAIL: ERROR should be filtered at FATAL level\n");
            fail();
        }

        // Restore original level
        rrr::Log::set_level(original);
    }
};

// ============================================================
// Test Registration
// ============================================================

static TestLogBasicLevels _test_basic_levels;
static TestLogFiltering _test_filtering;
static TestLogFormat _test_format;
static TestLogGeneric _test_generic;
static TestLogLevelConfig _test_level_config;

namespace {
struct TestRegistrar {
    TestRegistrar() {
        test::TestMgr::instance()->reg(&_test_basic_levels);
        test::TestMgr::instance()->reg(&_test_filtering);
        test::TestMgr::instance()->reg(&_test_format);
        test::TestMgr::instance()->reg(&_test_generic);
        test::TestMgr::instance()->reg(&_test_level_config);
    }
} _registrar;
}

// ============================================================
// C-compatible wrappers for Rust
// ============================================================

extern "C" {

int logging_test_run_all() {
    return test::TestMgr::instance()->run_all();
}

int logging_test_count() {
    return static_cast<int>(test::TestMgr::instance()->test_count());
}

} // extern "C"
