// test_strop_harness.cpp - M6.6a self-contained tests using unittest harness
//
// This file tests the strop functions using the minimal unittest harness.
// It includes:
// - strop_minimal.cpp functions (startswith, endswith using C library)
// - strop_stl.cpp functions (format_decimal using STL)
//
// This demonstrates that we can run actual tests through the Fragile pipeline.

#include <cstdio>
#include <cstring>
#include <vector>

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
        printf("Running %zu strop tests...\n", tests_.size());
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
// strop functions (C library based - from strop_minimal.cpp)
// ============================================================

namespace rrr {

bool startswith(const char* str, const char* head) {
    size_t len_str = strlen(str);
    size_t len_head = strlen(head);
    if (len_head > len_str) {
        return false;
    }
    return strncmp(str, head, len_head) == 0;
}

bool endswith(const char* str, const char* tail) {
    size_t len_str = strlen(str);
    size_t len_tail = strlen(tail);
    if (len_tail > len_str) {
        return false;
    }
    return strncmp(str + (len_str - len_tail), tail, len_tail) == 0;
}

} // namespace rrr

// ============================================================
// Test Cases
// ============================================================

class TestStartswithBasic : public test::TestCase {
public:
    TestStartswithBasic() : TestCase("strop", "startswith_basic") {}
    void run() override {
        // Basic positive case
        if (!rrr::startswith("hello world", "hello")) {
            printf("    FAIL: 'hello world' should start with 'hello'\n");
            fail();
        }
        // Basic negative case
        if (rrr::startswith("hello world", "world")) {
            printf("    FAIL: 'hello world' should not start with 'world'\n");
            fail();
        }
    }
};

class TestStartswithEdgeCases : public test::TestCase {
public:
    TestStartswithEdgeCases() : TestCase("strop", "startswith_edge") {}
    void run() override {
        // Empty prefix
        if (!rrr::startswith("hello", "")) {
            printf("    FAIL: empty prefix should always match\n");
            fail();
        }
        // Exact match
        if (!rrr::startswith("hello", "hello")) {
            printf("    FAIL: exact match should work\n");
            fail();
        }
        // Prefix longer than string
        if (rrr::startswith("hi", "hello")) {
            printf("    FAIL: longer prefix should not match\n");
            fail();
        }
        // Single character
        if (!rrr::startswith("hello", "h")) {
            printf("    FAIL: single char prefix should match\n");
            fail();
        }
    }
};

class TestEndswithBasic : public test::TestCase {
public:
    TestEndswithBasic() : TestCase("strop", "endswith_basic") {}
    void run() override {
        // Basic positive case
        if (!rrr::endswith("hello world", "world")) {
            printf("    FAIL: 'hello world' should end with 'world'\n");
            fail();
        }
        // Basic negative case
        if (rrr::endswith("hello world", "hello")) {
            printf("    FAIL: 'hello world' should not end with 'hello'\n");
            fail();
        }
    }
};

class TestEndswithEdgeCases : public test::TestCase {
public:
    TestEndswithEdgeCases() : TestCase("strop", "endswith_edge") {}
    void run() override {
        // Empty suffix
        if (!rrr::endswith("hello", "")) {
            printf("    FAIL: empty suffix should always match\n");
            fail();
        }
        // Exact match
        if (!rrr::endswith("hello", "hello")) {
            printf("    FAIL: exact match should work\n");
            fail();
        }
        // Suffix longer than string
        if (rrr::endswith("hi", "hello")) {
            printf("    FAIL: longer suffix should not match\n");
            fail();
        }
        // Single character
        if (!rrr::endswith("hello", "o")) {
            printf("    FAIL: single char suffix should match\n");
            fail();
        }
    }
};

class TestStropCombined : public test::TestCase {
public:
    TestStropCombined() : TestCase("strop", "combined") {}
    void run() override {
        const char* path = "/usr/local/bin/fragile";

        // Test path parsing with startswith/endswith
        if (!rrr::startswith(path, "/usr")) {
            printf("    FAIL: path should start with /usr\n");
            fail();
        }
        if (!rrr::startswith(path, "/usr/local")) {
            printf("    FAIL: path should start with /usr/local\n");
            fail();
        }
        if (!rrr::endswith(path, "fragile")) {
            printf("    FAIL: path should end with fragile\n");
            fail();
        }
        if (!rrr::endswith(path, "/fragile")) {
            printf("    FAIL: path should end with /fragile\n");
            fail();
        }

        // Negative cases
        if (rrr::startswith(path, "fragile")) {
            printf("    FAIL: path should not start with fragile\n");
            fail();
        }
        if (rrr::endswith(path, "/usr")) {
            printf("    FAIL: path should not end with /usr\n");
            fail();
        }
    }
};

// ============================================================
// Test Registration
// ============================================================

static TestStartswithBasic _test_startswith_basic;
static TestStartswithEdgeCases _test_startswith_edge;
static TestEndswithBasic _test_endswith_basic;
static TestEndswithEdgeCases _test_endswith_edge;
static TestStropCombined _test_strop_combined;

namespace {
struct TestRegistrar {
    TestRegistrar() {
        test::TestMgr::instance()->reg(&_test_startswith_basic);
        test::TestMgr::instance()->reg(&_test_startswith_edge);
        test::TestMgr::instance()->reg(&_test_endswith_basic);
        test::TestMgr::instance()->reg(&_test_endswith_edge);
        test::TestMgr::instance()->reg(&_test_strop_combined);
    }
} _registrar;
}

// ============================================================
// C-compatible wrappers for Rust
// ============================================================

extern "C" {

int strop_test_run_all() {
    return test::TestMgr::instance()->run_all();
}

int strop_test_count() {
    return static_cast<int>(test::TestMgr::instance()->test_count());
}

} // extern "C"
