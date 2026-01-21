// Test C++ constructors
struct Point {
    int x;
    int y;

    // Default constructor
    Point() : x(0), y(0) {}

    // Parameterized constructor
    Point(int a, int b) : x(a), y(b) {}

    // Getter methods
    int get_x() { return x; }
    int get_y() { return y; }
};

int main() {
    Point p1;
    Point p2(10, 20);
    return p2.get_x() + p2.get_y();
}
