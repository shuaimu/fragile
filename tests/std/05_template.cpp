// Test 05: Template function

template<typename T>
T identity(T x) {
    return x;
}

int main() {
    return identity(42);
}
