// Test C++ namespaces

namespace math {

int add(int a, int b) {
    return a + b;
}

int multiply(int a, int b) {
    return a * b;
}

} // namespace math

namespace nested {
namespace inner {

int value() {
    return 42;
}

} // namespace inner
} // namespace nested

int main() {
    int x = math::add(10, 20);
    int y = nested::inner::value();
    return x + y;
}
