// Test: Ternary operator
// Expected: test_ternary() returns 42

int max(int a, int b) {
    return a > b ? a : b;
}

int min(int a, int b) {
    return a < b ? a : b;
}

int test_ternary() {
    int a = max(30, 20);  // 30
    int b = min(15, 12);  // 12
    return a + b;         // 42
}
