// test_threading_harness.cpp - M6.6d basic threading tests
//
// This file tests C++11 standard library threading primitives:
// - std::thread for thread creation and joining
// - std::mutex for mutual exclusion
// - std::lock_guard for RAII-based locking
// - std::atomic for lock-free operations
// - Lambda functions with captures

#include <cstdio>
#include <thread>
#include <mutex>
#include <atomic>
#include <vector>
#include <functional>

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
        printf("Running %zu threading tests...\n", tests_.size());
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
// Test Cases
// ============================================================

class TestThreadBasic : public test::TestCase {
public:
    TestThreadBasic() : TestCase("threading", "basic_thread") {}
    void run() override {
        // Test basic thread creation with a simple flag
        bool thread_ran = false;

        std::thread t([&thread_ran]() {
            thread_ran = true;
        });

        t.join();

        if (!thread_ran) {
            printf("    FAIL: thread did not run\n");
            fail();
        }
    }
};

class TestMutexProtect : public test::TestCase {
public:
    TestMutexProtect() : TestCase("threading", "mutex_protect") {}
    void run() override {
        // Test mutex protection of shared counter
        const int num_threads = 4;
        const int increments_per_thread = 1000;
        int counter = 0;
        std::mutex mtx;

        std::vector<std::thread> threads;
        threads.reserve(num_threads);

        for (int i = 0; i < num_threads; i++) {
            threads.emplace_back([&counter, &mtx, increments_per_thread]() {
                for (int j = 0; j < increments_per_thread; j++) {
                    mtx.lock();
                    counter++;
                    mtx.unlock();
                }
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        int expected = num_threads * increments_per_thread;
        if (counter != expected) {
            printf("    FAIL: expected counter=%d, got counter=%d\n", expected, counter);
            fail();
        }
    }
};

class TestLockGuard : public test::TestCase {
public:
    TestLockGuard() : TestCase("threading", "lock_guard") {}
    void run() override {
        // Test RAII-based locking with lock_guard
        const int num_threads = 4;
        const int increments_per_thread = 1000;
        int counter = 0;
        std::mutex mtx;

        std::vector<std::thread> threads;
        threads.reserve(num_threads);

        for (int i = 0; i < num_threads; i++) {
            threads.emplace_back([&counter, &mtx, increments_per_thread]() {
                for (int j = 0; j < increments_per_thread; j++) {
                    std::lock_guard<std::mutex> guard(mtx);
                    counter++;
                    // Lock automatically released when guard goes out of scope
                }
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        int expected = num_threads * increments_per_thread;
        if (counter != expected) {
            printf("    FAIL: expected counter=%d, got counter=%d\n", expected, counter);
            fail();
        }
    }
};

class TestAtomic : public test::TestCase {
public:
    TestAtomic() : TestCase("threading", "atomic_ops") {}
    void run() override {
        // Test atomic operations without mutex
        const int num_threads = 4;
        const int increments_per_thread = 1000;
        std::atomic<int> counter{0};

        std::vector<std::thread> threads;
        threads.reserve(num_threads);

        for (int i = 0; i < num_threads; i++) {
            threads.emplace_back([&counter, increments_per_thread]() {
                for (int j = 0; j < increments_per_thread; j++) {
                    counter.fetch_add(1, std::memory_order_relaxed);
                }
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        int expected = num_threads * increments_per_thread;
        int actual = counter.load();
        if (actual != expected) {
            printf("    FAIL: expected counter=%d, got counter=%d\n", expected, actual);
            fail();
        }
    }
};

class TestThreadLambdaCapture : public test::TestCase {
public:
    TestThreadLambdaCapture() : TestCase("threading", "lambda_capture") {}
    void run() override {
        // Test threads with value and reference captures
        int sum = 0;
        std::mutex mtx;
        const int num_threads = 4;

        std::vector<std::thread> threads;
        threads.reserve(num_threads);

        for (int i = 0; i < num_threads; i++) {
            // Capture i by value, sum and mtx by reference
            threads.emplace_back([i, &sum, &mtx]() {
                std::lock_guard<std::mutex> guard(mtx);
                sum += i;  // Add thread index: 0 + 1 + 2 + 3 = 6
            });
        }

        for (auto& t : threads) {
            t.join();
        }

        // Sum should be 0 + 1 + 2 + 3 = 6
        int expected = (num_threads * (num_threads - 1)) / 2;
        if (sum != expected) {
            printf("    FAIL: expected sum=%d, got sum=%d\n", expected, sum);
            fail();
        }
    }
};

// ============================================================
// Test Registration
// ============================================================

static TestThreadBasic _test_thread_basic;
static TestMutexProtect _test_mutex_protect;
static TestLockGuard _test_lock_guard;
static TestAtomic _test_atomic;
static TestThreadLambdaCapture _test_lambda_capture;

namespace {
struct TestRegistrar {
    TestRegistrar() {
        test::TestMgr::instance()->reg(&_test_thread_basic);
        test::TestMgr::instance()->reg(&_test_mutex_protect);
        test::TestMgr::instance()->reg(&_test_lock_guard);
        test::TestMgr::instance()->reg(&_test_atomic);
        test::TestMgr::instance()->reg(&_test_lambda_capture);
    }
} _registrar;
}

// ============================================================
// C-compatible wrappers for Rust
// ============================================================

extern "C" {

int threading_test_run_all() {
    return test::TestMgr::instance()->run_all();
}

int threading_test_count() {
    return static_cast<int>(test::TestMgr::instance()->test_count());
}

} // extern "C"
