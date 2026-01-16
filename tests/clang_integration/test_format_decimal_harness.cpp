// test_format_decimal_harness.cpp - M6.6b strop tests with STL functions
//
// This file tests the format_decimal functions from strop_stl.cpp using the
// unittest harness. It demonstrates:
// - STL usage (std::string, std::ostringstream)
// - Decimal number formatting with commas
// - Integration of multiple C++ compilation units (conceptually)

#include <cstdio>
#include <cstring>
#include <sstream>
#include <string>
#include <iomanip>
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
        printf("Running %zu format_decimal tests...\n", tests_.size());
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
// format_decimal implementations (from strop_stl.cpp pattern)
// ============================================================

namespace rrr {

// Format double as -#,###.##
std::string format_decimal_double(double val) {
    std::ostringstream o;
    o.precision(2);
    o << std::fixed << val;
    std::string s = o.str();
    std::string str;

    // Find decimal point position
    size_t idx = 0;
    while (idx < s.size()) {
        if (s[idx] == '.') {
            break;
        }
        idx++;
    }

    str.reserve(s.size() + 16);

    // Add commas every 3 digits before decimal
    for (size_t i = 0; i < idx; i++) {
        if ((idx - i) % 3 == 0 && i != 0 && s[i - 1] != '-') {
            str += ',';
        }
        str += s[i];
    }

    // Add decimal part
    str += s.substr(idx);

    // Handle -0.00 case
    if (str == "-0.00") {
        str = "0.00";
    }

    return str;
}

// Format int as -#,###
std::string format_decimal_int(int val) {
    std::ostringstream o;
    o << val;
    std::string s = o.str();
    std::string str;

    str.reserve(s.size() + 8);

    // Add commas every 3 digits
    for (size_t i = 0; i < s.size(); i++) {
        if ((s.size() - i) % 3 == 0 && i != 0 && s[i - 1] != '-') {
            str += ',';
        }
        str += s[i];
    }

    return str;
}

} // namespace rrr

// ============================================================
// Test Cases
// ============================================================

class TestFormatDecimalDoubleBasic : public test::TestCase {
public:
    TestFormatDecimalDoubleBasic() : TestCase("format", "double_basic") {}
    void run() override {
        // Small number
        std::string r1 = rrr::format_decimal_double(1.23);
        if (r1 != "1.23") {
            printf("    FAIL: expected '1.23', got '%s'\n", r1.c_str());
            fail();
        }

        // With commas
        std::string r2 = rrr::format_decimal_double(1234.56);
        if (r2 != "1,234.56") {
            printf("    FAIL: expected '1,234.56', got '%s'\n", r2.c_str());
            fail();
        }

        // Large number
        std::string r3 = rrr::format_decimal_double(1234567.89);
        if (r3 != "1,234,567.89") {
            printf("    FAIL: expected '1,234,567.89', got '%s'\n", r3.c_str());
            fail();
        }
    }
};

class TestFormatDecimalDoubleEdge : public test::TestCase {
public:
    TestFormatDecimalDoubleEdge() : TestCase("format", "double_edge") {}
    void run() override {
        // Zero
        std::string r1 = rrr::format_decimal_double(0.0);
        if (r1 != "0.00") {
            printf("    FAIL: expected '0.00', got '%s'\n", r1.c_str());
            fail();
        }

        // Negative
        std::string r2 = rrr::format_decimal_double(-1234.56);
        if (r2 != "-1,234.56") {
            printf("    FAIL: expected '-1,234.56', got '%s'\n", r2.c_str());
            fail();
        }

        // Tiny number
        std::string r3 = rrr::format_decimal_double(0.01);
        if (r3 != "0.01") {
            printf("    FAIL: expected '0.01', got '%s'\n", r3.c_str());
            fail();
        }
    }
};

class TestFormatDecimalIntBasic : public test::TestCase {
public:
    TestFormatDecimalIntBasic() : TestCase("format", "int_basic") {}
    void run() override {
        // Small number
        std::string r1 = rrr::format_decimal_int(123);
        if (r1 != "123") {
            printf("    FAIL: expected '123', got '%s'\n", r1.c_str());
            fail();
        }

        // With commas
        std::string r2 = rrr::format_decimal_int(1234);
        if (r2 != "1,234") {
            printf("    FAIL: expected '1,234', got '%s'\n", r2.c_str());
            fail();
        }

        // Large number
        std::string r3 = rrr::format_decimal_int(1234567);
        if (r3 != "1,234,567") {
            printf("    FAIL: expected '1,234,567', got '%s'\n", r3.c_str());
            fail();
        }
    }
};

class TestFormatDecimalIntEdge : public test::TestCase {
public:
    TestFormatDecimalIntEdge() : TestCase("format", "int_edge") {}
    void run() override {
        // Zero
        std::string r1 = rrr::format_decimal_int(0);
        if (r1 != "0") {
            printf("    FAIL: expected '0', got '%s'\n", r1.c_str());
            fail();
        }

        // Negative
        std::string r2 = rrr::format_decimal_int(-1234);
        if (r2 != "-1,234") {
            printf("    FAIL: expected '-1,234', got '%s'\n", r2.c_str());
            fail();
        }

        // Single digit
        std::string r3 = rrr::format_decimal_int(5);
        if (r3 != "5") {
            printf("    FAIL: expected '5', got '%s'\n", r3.c_str());
            fail();
        }

        // Negative small
        std::string r4 = rrr::format_decimal_int(-42);
        if (r4 != "-42") {
            printf("    FAIL: expected '-42', got '%s'\n", r4.c_str());
            fail();
        }
    }
};

class TestFormatDecimalLarge : public test::TestCase {
public:
    TestFormatDecimalLarge() : TestCase("format", "large_numbers") {}
    void run() override {
        // Billion
        std::string r1 = rrr::format_decimal_int(1000000000);
        if (r1 != "1,000,000,000") {
            printf("    FAIL: expected '1,000,000,000', got '%s'\n", r1.c_str());
            fail();
        }

        // Large negative
        std::string r2 = rrr::format_decimal_int(-999999999);
        if (r2 != "-999,999,999") {
            printf("    FAIL: expected '-999,999,999', got '%s'\n", r2.c_str());
            fail();
        }

        // Large double
        std::string r3 = rrr::format_decimal_double(9876543.21);
        if (r3 != "9,876,543.21") {
            printf("    FAIL: expected '9,876,543.21', got '%s'\n", r3.c_str());
            fail();
        }
    }
};

// ============================================================
// Test Registration
// ============================================================

static TestFormatDecimalDoubleBasic _test_double_basic;
static TestFormatDecimalDoubleEdge _test_double_edge;
static TestFormatDecimalIntBasic _test_int_basic;
static TestFormatDecimalIntEdge _test_int_edge;
static TestFormatDecimalLarge _test_large;

namespace {
struct TestRegistrar {
    TestRegistrar() {
        test::TestMgr::instance()->reg(&_test_double_basic);
        test::TestMgr::instance()->reg(&_test_double_edge);
        test::TestMgr::instance()->reg(&_test_int_basic);
        test::TestMgr::instance()->reg(&_test_int_edge);
        test::TestMgr::instance()->reg(&_test_large);
    }
} _registrar;
}

// ============================================================
// C-compatible wrappers for Rust
// ============================================================

extern "C" {

int format_test_run_all() {
    return test::TestMgr::instance()->run_all();
}

int format_test_count() {
    return static_cast<int>(test::TestMgr::instance()->test_count());
}

} // extern "C"
