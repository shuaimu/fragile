# CMake Rust Object Demo

Minimal demo showing how to compile a `.rs` file into a `.o` with `rustc --emit=obj`
from CMake and link that object into a C++ executable.

This variant demonstrates Rust functions without `extern "C"` on the Rust side.
The C++ declarations bind to exact symbol names via `asm("...")`.

## Build and run

```bash
cd examples/cmake-rust-object-demo
cmake -S . -B build
cmake --build build -j
./build/demo
```

Expected output:

```text
rust_add(7, 5) = 12
rust_mul(7, 5) = 35
```

## Key CMake pattern

The Rust compilation is done with a custom command:

```cmake
add_custom_command(
  OUTPUT ${RUST_OBJ}
  COMMAND ${RUSTC_BIN} --edition=2021 --crate-type=lib --emit=obj ...
)
```

Then `${RUST_OBJ}` is marked as an external generated object and attached to a normal
`add_executable(...)` target.

## ABI note

This demo intentionally calls non-`extern "C"` Rust functions from C++ and works
for this local single-toolchain build. Rust ABI is not a stable cross-toolchain ABI.
