// Simple doctest test file for Fragile compiler
// Tests basic doctest functionality

#define DOCTEST_CONFIG_IMPLEMENT_WITH_MAIN
#include "../../vendor/doctest/doctest/doctest.h"

// Simple function to test
int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

// Test basic arithmetic
TEST_CASE("testing factorial") {
    CHECK(factorial(0) == 1);
    CHECK(factorial(1) == 1);
    CHECK(factorial(2) == 2);
    CHECK(factorial(3) == 6);
    CHECK(factorial(4) == 24);
    CHECK(factorial(5) == 120);
}

// Test basic comparisons
TEST_CASE("testing basic comparisons") {
    CHECK(1 < 2);
    CHECK(2 > 1);
    CHECK(1 == 1);
    CHECK(1 != 2);
}

// Test with subcases
TEST_CASE("testing with subcases") {
    int x = 5;

    SUBCASE("addition") {
        x += 3;
        CHECK(x == 8);
    }

    SUBCASE("multiplication") {
        x *= 2;
        CHECK(x == 10);
    }
}
