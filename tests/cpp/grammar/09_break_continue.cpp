// Test: Break and continue
// Expected: test_break() returns 15, test_continue() returns 25

int test_break() {
    int sum = 0;
    for (int i = 1; i <= 10; i = i + 1) {
        if (i > 5) {
            break;
        }
        sum = sum + i;
    }
    return sum;  // 1+2+3+4+5 = 15
}

int test_continue() {
    int sum = 0;
    for (int i = 1; i <= 10; i = i + 1) {
        if (i % 2 == 0) {
            continue;  // skip even numbers
        }
        sum = sum + i;
    }
    return sum;  // 1+3+5+7+9 = 25
}

int test_break_continue() {
    return test_break() + test_continue();  // 15 + 25 = 40
}
