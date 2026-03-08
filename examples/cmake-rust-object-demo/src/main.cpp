#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include "rust_math.hpp"

int main() {
  RustAccumulator* acc =
      static_cast<RustAccumulator*>(std::malloc(sizeof(RustAccumulator)));
  if (acc == nullptr) {
    std::fprintf(stderr, "failed to allocate rust accumulator storage\n");
    return 1;
  }

  if (!rust_accumulator_init(acc, /*seed=*/10, /*scale=*/3)) {
    std::fprintf(stderr, "failed to create rust accumulator\n");
    std::free(acc);
    return 1;
  }

  std::printf("initial total -> %lld\n", static_cast<long long>(acc->total));
  std::printf("scale -> %lld\n", static_cast<long long>(acc->scale));
  acc->total += acc->scale;
  std::printf("after manual bump -> %lld\n", static_cast<long long>(acc->total));

  const std::int64_t step1 = rust_accumulator_push(acc, 4);
  const std::int64_t step2 = rust_accumulator_push(acc, 2);
  const std::int64_t step3 = rust_accumulator_push(acc, 5);
  const std::int64_t total = rust_accumulator_get(acc);

  std::printf("push(4) -> %lld\n", static_cast<long long>(step1));
  std::printf("push(2) -> %lld\n", static_cast<long long>(step2));
  std::printf("push(5) -> %lld\n", static_cast<long long>(step3));
  std::printf("final total -> %lld\n", static_cast<long long>(total));

  rust_accumulator_drop(acc);
  std::free(acc);
  return 0;
}
