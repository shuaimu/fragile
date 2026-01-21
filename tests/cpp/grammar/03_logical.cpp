// Test: Logical operations
// Expected: test_logical() returns 3

int test_logical() {
    bool t = true;
    bool f = false;

    int count = 0;

    if (t && t) count = count + 1;  // true
    if (t || f) count = count + 1;  // true
    if (!f) count = count + 1;      // true
    if (t && f) count = count + 1;  // false
    if (f || f) count = count + 1;  // false

    return count;  // 3
}
