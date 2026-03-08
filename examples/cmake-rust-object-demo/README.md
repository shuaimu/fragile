# CMake Rust Object Demo

Minimal demo showing how to compile a Rust source (`math.rs`) into an object and
link it into a C++ executable.

This variant keeps the original split-source layout:
- C++ entrypoint: `src/main.cpp`
- Rust implementation: `src/math.rs`
- Generated C++ mirror header from Rust structs: `build/generated/rust_math.hpp`
- C++ compilation is done with `fragilec`
- final executable link is driven by `fragilec` and delegated to `clang++`

## Build and run

```bash
cd examples/cmake-rust-object-demo
# Optional override:
# export FRAGILEC_BIN=/abs/path/to/fragilec
cmake -S . -B build
cmake --build build -j
./build/demo
```

Expected output with `fragilec` strict mode:

```text
initial total -> 10
scale -> 3
after manual bump -> 13
push(4) -> 25
push(2) -> 31
push(5) -> 46
final total -> 46
```

## Key CMake pattern

`CMAKE_CXX_COMPILER` is set to `fragilec` before `project(...)`. The link step
still goes through `fragilec`, but we explicitly force its underlying linker to
`clang++` via `FRAGILEC_LINKER`.

```cmake
set(CMAKE_CXX_COMPILER "${FRAGILEC_BIN}" CACHE FILEPATH "C++ compiler" FORCE)
set_property(TARGET demo PROPERTY
  RULE_LAUNCH_LINK "${CMAKE_COMMAND} -E env FRAGILEC_LINKER=${CLANGXX_BIN}"
)

add_custom_command(
  OUTPUT ${RUST_HEADER}
  COMMAND ${RUST_HEADER_GEN_SCRIPT} ${RUST_SRC} ${RUST_HEADER}
)

add_custom_command(
  OUTPUT ${RUST_OBJ}
  COMMAND ${RUSTC_BIN} --edition=2021 --crate-type=lib --emit=obj ...
)
```

## ABI note

This demo intentionally calls non-`extern "C"` Rust symbols from C++ by binding
to exact symbol names with `asm("...")`. It works in a single toolchain setup,
but Rust ABI is not a stable cross-toolchain ABI contract.

## Notes

`main.cpp` is intentionally kept to a supported C++ subset (`cstdio` + direct
free-function calls) so fragile's strict-mode lowering can preserve the entry
body end-to-end.

The demo avoids manual duplication of `RustAccumulator` in C++ by generating
`rust_math.hpp` from `math.rs` during CMake build. C++ includes that header and
can access fields directly (`acc->total`, `acc->scale`).

The same generated header also exports declarations for Rust `#[no_mangle] pub fn`
functions (with `asm("symbol")` binding), so `main.cpp` does not need separate
hand-written extern declarations.

Header generation is built into `fragilec`:

```bash
fragilec --emit-rust-cpp-header src/math.rs -o build/generated/rust_math.hpp
```
