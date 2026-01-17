// Simple factorial test without doctest
// This file tests the MIR pipeline without the complexity of doctest.h

int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

int main() {
    // Test factorial function
    int result = factorial(5);
    // Expected: 120
    return (result == 120) ? 0 : 1;
}
