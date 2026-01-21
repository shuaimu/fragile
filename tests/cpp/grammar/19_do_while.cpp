// Test: Do-while loop
// Expected: test_do_while() returns 42

int test_do_while() {
    int sum = 0;
    int i = 1;

    do {
        sum = sum + i;
        i = i + 1;
    } while (i <= 6);

    // 1+2+3+4+5+6 = 21
    return sum * 2;  // 42
}
