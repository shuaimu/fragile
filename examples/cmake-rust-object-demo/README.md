# CMake Rust Object Demo

Minimal demo showing how to compile a `.rs` file into a `.o` with `rustc --emit=obj`
from CMake and link that object into a C++ executable.

This variant demonstrates a Rust-owned object being created and used from C++
without `extern "C"` on the Rust side. C++ declarations bind to exact Rust
symbol names via `asm("...")`.

To keep `rustc --emit=obj` linking simple, the object lives in C++-allocated
aligned storage. Rust provides `size/align/init/push/get/drop` functions, and
C++ keeps the type opaque.

## Build and run

```bash
cd examples/cmake-rust-object-demo
cmake -S . -B build
cmake --build build -j
./build/demo
```

Expected output:

```text
push(4) -> 22
push(-2) -> 16
push(5) -> 31
final total -> 31
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
The object is passed as an opaque pointer and managed by Rust lifecycle functions
(`size`/`align`/`init`/`drop`).
