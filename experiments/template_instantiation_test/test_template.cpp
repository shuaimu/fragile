// Test file to verify LibTooling can access instantiated template method bodies
// Compile with: clang++ -Xclang -ast-dump -fsyntax-only test_template.cpp

template<typename T>
class Container {
    T value;
public:
    Container(T v) : value(v) {}

    T get() const {
        return value;  // We want to see this body with T=int substituted
    }

    void set(T v) {
        value = v;  // We want to see this body with T=int substituted
    }

    T add(T other) {
        return value + other;  // Expression with template parameter
    }
};

int main() {
    Container<int> c(42);      // Implicit instantiation of Container<int>
    int x = c.get();           // Should instantiate get() method
    c.set(10);                 // Should instantiate set() method
    int y = c.add(5);          // Should instantiate add() method
    return x + y;
}
