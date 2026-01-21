// Test C++ classes - simple version

class Counter {
public:
    int value;

    int get() {
        return value;
    }

    int add(int n) {
        return value + n;
    }
};

int main() {
    // For now, skip constructor call
    return 0;
}
