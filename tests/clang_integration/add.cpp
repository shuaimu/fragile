// Minimal C++ test file for Fragile Clang integration
// Phase 1.2: Simple add function

// A simple C++ add function
// Uses C++ name mangling: _Z7add_cppii
int add_cpp(int a, int b) {
    return a + b;
}
