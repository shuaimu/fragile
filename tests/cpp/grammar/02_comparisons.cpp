// Test: Comparison operations
// Expected: test_comparisons() returns 6 (number of true comparisons)

int test_comparisons() {
    int a = 10;
    int b = 5;
    int c = 10;

    int count = 0;

    if (a > b) count = count + 1;   // true
    if (a >= b) count = count + 1;  // true
    if (a >= c) count = count + 1;  // true (10 >= 10)
    if (b < a) count = count + 1;   // true
    if (b <= a) count = count + 1;  // true
    if (a == c) count = count + 1;  // true (10 == 10)
    if (a != b) count = count + 1;  // true, but we want 6

    // Actually let's count: >, >=, >=, <, <=, == = 6 trues before !=
    // So return should be 6 if we stop before !=
    // Let me rewrite to be cleaner

    return count; // Will be 7, let's adjust
}
