#include <cstdint>
#include <iostream>

extern "C" std::int32_t rust_add(std::int32_t a, std::int32_t b);
extern "C" std::int32_t rust_mul(std::int32_t a, std::int32_t b);

int main() {
  const std::int32_t x = 7;
  const std::int32_t y = 5;

  const std::int32_t sum = rust_add(x, y);
  const std::int32_t product = rust_mul(x, y);

  std::cout << "rust_add(" << x << ", " << y << ") = " << sum << "\n";
  std::cout << "rust_mul(" << x << ", " << y << ") = " << product << "\n";
  return 0;
}
