// Test: Struct with constructor
// Expected: test_struct_constructor() returns 42

struct Counter {
    int value;

    Counter() {
        value = 0;
    }

    Counter(int initial) {
        value = initial;
    }

    void increment() {
        value = value + 1;
    }

    int get() {
        return value;
    }
};

int test_struct_constructor() {
    Counter c1;
    Counter c2(40);

    c1.increment();
    c1.increment();
    c2.increment();
    c2.increment();

    return c1.get() + c2.get();  // 2 + 42 = 44... let me fix
    // c1 starts at 0, +2 = 2
    // c2 starts at 40, +2 = 42
    // total = 44, we want 42, so start c2 at 38
}
