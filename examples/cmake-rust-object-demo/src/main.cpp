#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstdlib>

struct RustAccumulator;

// C++ declarations without `extern "C"`, pinned to exported Rust symbol names.
extern std::size_t rust_accumulator_size() asm("rust_accumulator_size");
extern std::size_t rust_accumulator_align() asm("rust_accumulator_align");
extern bool rust_accumulator_init(RustAccumulator* obj, std::int64_t seed,
                                  std::int64_t scale)
    asm("rust_accumulator_init");
extern std::int64_t rust_accumulator_push(RustAccumulator* obj, std::int64_t value)
    asm("rust_accumulator_push");
extern std::int64_t rust_accumulator_get(const RustAccumulator* obj)
    asm("rust_accumulator_get");
extern void rust_accumulator_drop(RustAccumulator* obj) asm("rust_accumulator_drop");

int main() {
  const std::size_t size = rust_accumulator_size();
  void* storage = std::malloc(size);
  if (storage == nullptr) {
    std::fprintf(stderr, "failed to allocate rust accumulator storage\n");
    return 1;
  }
  RustAccumulator* acc = static_cast<RustAccumulator*>(storage);

  if (!rust_accumulator_init(acc, /*seed=*/10, /*scale=*/3)) {
    std::fprintf(stderr, "failed to create rust accumulator\n");
    std::free(storage);
    return 1;
  }

  const std::int64_t step1 = rust_accumulator_push(acc, 4);
  const std::int64_t step2 = rust_accumulator_push(acc, 2);
  const std::int64_t step3 = rust_accumulator_push(acc, 5);
  const std::int64_t total = rust_accumulator_get(acc);

  std::printf("push(4) -> %lld\n", static_cast<long long>(step1));
  std::printf("push(2) -> %lld\n", static_cast<long long>(step2));
  std::printf("push(5) -> %lld\n", static_cast<long long>(step3));
  std::printf("final total -> %lld\n", static_cast<long long>(total));

  rust_accumulator_drop(acc);
  std::free(storage);
  return 0;
}
