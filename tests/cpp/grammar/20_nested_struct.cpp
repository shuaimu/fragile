// Test: Nested structs
// Expected: test_nested_struct() returns 42

struct Inner {
    int value;
};

struct Outer {
    Inner a;
    Inner b;
    int extra;
};

int test_nested_struct() {
    Outer o;
    o.a.value = 10;
    o.b.value = 20;
    o.extra = 12;

    return o.a.value + o.b.value + o.extra;  // 42
}
