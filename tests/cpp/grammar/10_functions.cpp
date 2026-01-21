// Test: Function calls and parameters
// Expected: test_functions() returns 42

int add(int a, int b) {
    return a + b;
}

int multiply(int a, int b) {
    return a * b;
}

int square(int x) {
    return multiply(x, x);
}

int test_functions() {
    int a = add(10, 5);       // 15
    int b = multiply(3, 4);   // 12
    int c = square(3);        // 9
    return a + b + c + 6;     // 15 + 12 + 9 + 6 = 42
}
