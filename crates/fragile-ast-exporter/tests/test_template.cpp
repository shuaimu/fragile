// Test file for template instantiation export

template<typename T>
class Container {
public:
    T value;

    Container(T v) : value(v) {}

    T get() const {
        return value;
    }

    void set(T v) {
        value = v;
    }

    T double_value() {
        return value + value;
    }
};

// Explicit instantiation with int
template class Container<int>;

// Use with double (implicit instantiation)
void use_double() {
    Container<double> c(3.14);
    double x = c.get();
    c.set(2.71);
    double y = c.double_value();
}

int main() {
    Container<int> ci(42);
    int x = ci.get();
    ci.set(100);
    int y = ci.double_value();

    return 0;
}
