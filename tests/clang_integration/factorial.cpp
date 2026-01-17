// Factorial test file for Fragile Clang integration
// Phase 5.2: Tests recursion and control flow via MIR pipeline

// Factorial function with recursion
// Tests: if/else branching, recursion (function calls), return statements
int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
