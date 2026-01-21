// Test: If-else control flow
// Expected: test_if_else(10) returns 1, test_if_else(5) returns -1, test_if_else(7) returns 0

int test_if_else(int x) {
    if (x > 7) {
        return 1;
    } else if (x < 7) {
        return -1;
    } else {
        return 0;
    }
}

int test_if_else_main() {
    int a = test_if_else(10);  // 1
    int b = test_if_else(5);   // -1
    int c = test_if_else(7);   // 0
    return a + b + c + 42;     // 1 - 1 + 0 + 42 = 42
}
