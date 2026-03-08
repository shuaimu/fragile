#include <cstddef>
#include <cstdint>
#include <iostream>
#include <new>

struct RustAccumulator;

// C++ declarations without `extern "C"`, pinned to exported Rust symbol names.
extern std::size_t rust_accumulator_size() asm("rust_accumulator_size");
extern std::size_t rust_accumulator_align() asm("rust_accumulator_align");
extern bool rust_accumulator_init(RustAccumulator *obj, std::int64_t seed,
                                  std::int64_t scale)
    asm("rust_accumulator_init");
extern std::int64_t rust_accumulator_push(RustAccumulator *obj,
                                          std::int64_t value)
    asm("rust_accumulator_push");
extern std::int64_t rust_accumulator_get(const RustAccumulator *obj)
    asm("rust_accumulator_get");
extern void rust_accumulator_drop(RustAccumulator *obj) asm("rust_accumulator_drop");

class Accumulator {
 public:
  Accumulator(std::int64_t seed, std::int64_t scale) {
    align_ = rust_accumulator_align();
    storage_ = ::operator new(rust_accumulator_size(), std::align_val_t(align_));
    obj_ = static_cast<RustAccumulator *>(storage_);
    valid_ = rust_accumulator_init(obj_, seed, scale);
  }

  ~Accumulator() {
    if (obj_ != nullptr && valid_) {
      rust_accumulator_drop(obj_);
    }
    if (storage_ != nullptr) {
      ::operator delete(storage_, std::align_val_t(align_));
    }
  }

  Accumulator(const Accumulator &) = delete;
  Accumulator &operator=(const Accumulator &) = delete;

  std::int64_t Push(std::int64_t value) { return rust_accumulator_push(obj_, value); }
  std::int64_t Total() const { return rust_accumulator_get(obj_); }
  bool IsValid() const { return obj_ != nullptr && valid_; }

 private:
  std::size_t align_ = alignof(std::max_align_t);
  void *storage_ = nullptr;
  RustAccumulator *obj_ = nullptr;
  bool valid_ = false;
};

int main() {
  Accumulator acc(/*seed=*/10, /*scale=*/3);
  if (!acc.IsValid()) {
    std::cerr << "failed to create rust accumulator\n";
    return 1;
  }

  const std::int64_t step1 = acc.Push(4);
  const std::int64_t step2 = acc.Push(-2);
  const std::int64_t step3 = acc.Push(5);

  std::cout << "push(4) -> " << step1 << "\n";
  std::cout << "push(-2) -> " << step2 << "\n";
  std::cout << "push(5) -> " << step3 << "\n";
  std::cout << "final total -> " << acc.Total() << "\n";
  return 0;
}
