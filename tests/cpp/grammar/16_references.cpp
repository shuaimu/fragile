// Test: References
// Expected: test_references() returns 42

void increment_ref(int& x) {
    x = x + 1;
}

int test_references() {
    int a = 40;
    int& ref = a;

    // Use only ref after creating it (Rust borrow rules)
    increment_ref(ref); // a becomes 41 via ref
    increment_ref(ref); // a becomes 42 via ref

    return ref;  // return via ref = 42
}
