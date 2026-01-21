// Test: Nested loops
// Expected: test_nested_loops() returns 30 (sum of multiplication table 1-3 x 1-3)

int test_nested_loops() {
    int sum = 0;
    for (int i = 1; i <= 3; i = i + 1) {
        for (int j = 1; j <= 3; j = j + 1) {
            sum = sum + i * j;
        }
    }
    // 1*1 + 1*2 + 1*3 + 2*1 + 2*2 + 2*3 + 3*1 + 3*2 + 3*3
    // = 1 + 2 + 3 + 2 + 4 + 6 + 3 + 6 + 9 = 36
    return sum;
}
