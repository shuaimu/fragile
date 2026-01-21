// Test: Struct with methods
// Expected: test_struct_methods() returns 42

struct Rectangle {
    int width;
    int height;

    int area() {
        return width * height;
    }

    int perimeter() {
        return 2 * (width + height);
    }
};

int test_struct_methods() {
    Rectangle r;
    r.width = 5;
    r.height = 6;
    int a = r.area();       // 30
    int p = r.perimeter();  // 22
    return a + p - 10;      // 30 + 22 - 10 = 42
}
