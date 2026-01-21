// Test: Pointers
// Expected: test_pointers() returns 42

void swap(int* a, int* b) {
    int temp = *a;
    *a = *b;
    *b = temp;
}

int test_pointers() {
    int x = 10;
    int y = 32;

    int* px = &x;
    int* py = &y;

    // Swap them
    swap(px, py);

    // Now x=32, y=10
    return x + y;  // 42
}
