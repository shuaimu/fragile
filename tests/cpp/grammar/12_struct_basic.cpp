// Test: Basic struct
// Expected: test_struct_basic() returns 42

struct Point {
    int x;
    int y;
};

int test_struct_basic() {
    Point p;
    p.x = 10;
    p.y = 32;
    return p.x + p.y;  // 42
}
