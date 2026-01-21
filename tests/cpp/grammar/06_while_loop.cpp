// Test: While loop
// Expected: sum_to_n(9) returns 45 (1+2+3+4+5+6+7+8+9)

int sum_to_n(int n) {
    int sum = 0;
    int i = 1;
    while (i <= n) {
        sum = sum + i;
        i = i + 1;
    }
    return sum;
}

int test_while_loop() {
    return sum_to_n(9);  // 45
}
