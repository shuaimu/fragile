// Simple add function for transpiler testing

int add(int a, int b) {
    return a + b;
}

struct Point {
    double x;
    double y;
};

double distance_from_origin(Point p) {
    return p.x * p.x + p.y * p.y;
}
