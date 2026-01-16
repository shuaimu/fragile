// unittest_minimal.cpp - Minimal unit test harness for M6.5
//
// This is a simplified version of mako's unittest framework that demonstrates:
// - Class inheritance with virtual functions
// - Singleton pattern
// - std::vector with class pointers
// - C-compatible wrapper functions
//
// The harness provides:
// - TestCase base class with virtual run()
// - TestMgr singleton for test registration and execution
// - C wrappers for integration with Rust

#include <cstdio>
#include <vector>

namespace test {

// Base class for all test cases
class TestCase {
    const char* group_;
    const char* name_;
    int failures_;
public:
    TestCase(const char* group, const char* name)
        : group_(group), name_(name), failures_(0) {}
    virtual ~TestCase() {}

    // Override this to implement the test
    virtual void run() = 0;

    const char* group() const { return group_; }
    const char* name() const { return name_; }
    void fail() { failures_++; }
    void reset() { failures_ = 0; }
    int failures() const { return failures_; }
};

// Singleton test manager
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

    void reg(TestCase* t) {
        tests_.push_back(t);
    }

    int run_all() {
        int total_failures = 0;
        int passed = 0;

        printf("Running %zu tests...\n", tests_.size());

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

    size_t test_count() const {
        return tests_.size();
    }
};

// Static initialization
TestMgr* TestMgr::instance_ = nullptr;

} // namespace test

// C-compatible wrapper functions for calling from Rust
extern "C" {

// Register a test case (not typically called from Rust, but available)
void test_register(test::TestCase* t) {
    test::TestMgr::instance()->reg(t);
}

// Run all registered tests
// Returns: number of failures (0 = all passed)
int test_run_all() {
    return test::TestMgr::instance()->run_all();
}

// Get number of registered tests
int test_count() {
    return static_cast<int>(test::TestMgr::instance()->test_count());
}

} // extern "C"

// Test cases using the harness
// These demonstrate how the framework works

// Test case for startswith functionality
class TestStartswith : public test::TestCase {
public:
    TestStartswith() : TestCase("strop", "startswith") {}

    void run() override {
        // Test: "hello" starts with "hel"
        const char* str = "hello world";
        if (str[0] != 'h' || str[1] != 'e' || str[2] != 'l') {
            fail();
        }

        // Test: empty prefix always matches
        if (str[0] == '\0') {
            fail(); // This should not fail - str is not empty
        }
    }
};

// Test case for endswith functionality
class TestEndswith : public test::TestCase {
public:
    TestEndswith() : TestCase("strop", "endswith") {}

    void run() override {
        // Test: "hello world" ends with "world"
        const char* str = "hello world";
        const char* suffix = "world";

        // Simple check - str length is 11, suffix length is 5
        // str + 6 should be "world"
        const char* end_part = str + 6;
        bool matches = true;
        for (int i = 0; suffix[i] != '\0'; i++) {
            if (end_part[i] != suffix[i]) {
                matches = false;
                break;
            }
        }
        if (!matches) {
            fail();
        }
    }
};

// Test case for integer operations
class TestIntOps : public test::TestCase {
public:
    TestIntOps() : TestCase("math", "int_ops") {}

    void run() override {
        // Test min
        int a = 5, b = 10;
        int min_val = (a < b) ? a : b;
        if (min_val != 5) fail();

        // Test max
        int max_val = (a > b) ? a : b;
        if (max_val != 10) fail();

        // Test clamp
        int value = 15;
        int clamped = value;
        if (clamped < 0) clamped = 0;
        if (clamped > 10) clamped = 10;
        if (clamped != 10) fail();
    }
};

// Auto-register test cases
static TestStartswith _test_startswith;
static TestEndswith _test_endswith;
static TestIntOps _test_int_ops;

// Registration happens at static initialization time
namespace {
struct TestRegistrar {
    TestRegistrar() {
        test::TestMgr::instance()->reg(&_test_startswith);
        test::TestMgr::instance()->reg(&_test_endswith);
        test::TestMgr::instance()->reg(&_test_int_ops);
    }
} _registrar;
}
