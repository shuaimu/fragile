// Test: References
// Expected: test_references() returns 42

void increment_ref(int& x) {
    x = x + 1;
}

int test_references() {
    int a = 40;
    int& ref = a;

    increment_ref(a);   // a becomes 41
    increment_ref(ref); // a becomes 42

    return a;  // 42
}
