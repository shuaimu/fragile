// Test: Bitwise operations
// Expected: test_bitwise() returns 42

int test_bitwise() {
    int a = 0b1010;  // 10
    int b = 0b1100;  // 12

    int and_result = a & b;   // 0b1000 = 8
    int or_result = a | b;    // 0b1110 = 14
    int xor_result = a ^ b;   // 0b0110 = 6
    int not_result = ~0;      // -1
    int left = 1 << 3;        // 8
    int right = 16 >> 2;      // 4

    // 8 + 14 + 6 + 8 + 4 = 40, need 42
    return and_result + or_result + xor_result + left + right + 2;
}
