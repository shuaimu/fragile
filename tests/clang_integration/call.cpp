// Test file for C++ function calls via MIR injection
// This tests Task 1.2.5: Function call resolution

// Simple helper function
int helper(int x) {
    return x * 2;
}

// Function that calls another function
int double_and_add(int a, int b) {
    // This should generate a Call terminator in MIR
    int doubled_a = helper(a);
    int doubled_b = helper(b);
    return doubled_a + doubled_b;
}
