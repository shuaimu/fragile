// Test: For loop
// Expected: test_for_loop() returns 55 (1+2+...+10)

int test_for_loop() {
    int sum = 0;
    for (int i = 1; i <= 10; i = i + 1) {
        sum = sum + i;
    }
    return sum;  // 55
}
