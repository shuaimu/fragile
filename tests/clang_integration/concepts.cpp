// C++20 Concepts test file

// Simple concept using type trait
template<typename T>
concept Integral = __is_integral(T);

// Concept using multiple type traits
template<typename T>
concept Numeric = __is_arithmetic(T);

// Function template with requires clause
template<typename T>
    requires Integral<T>
T twice(T x) {
    return x + x;
}

// Function template without requires clause (for comparison)
template<typename T>
T identity(T x) {
    return x;
}
