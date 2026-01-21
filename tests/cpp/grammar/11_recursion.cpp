// Test: Recursion
// Expected: factorial(5) returns 120, fibonacci(10) returns 55

int factorial(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

int fibonacci(int n) {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

int test_recursion() {
    int fact = factorial(5);   // 120
    int fib = fibonacci(10);   // 55
    return fact + fib;         // 175
}
