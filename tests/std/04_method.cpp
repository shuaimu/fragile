// Test 04: Struct with method

struct Point {
    int x;

    int get_x() const {
        return x;
    }
};

int main() {
    Point p{42};
    return p.get_x();
}
