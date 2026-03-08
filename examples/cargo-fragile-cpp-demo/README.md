# cargo-fragile-cpp-demo

Minimal standalone example showing a Cargo-native flow:

1. `build.rs` transpiles `cpp/demo.cc` to Rust source using `fragile-clang`.
2. The crate `include!`s the generated Rust and builds it as normal crate code.

This demo does not use `.o` mode or C/C++ linker boundaries.

## Run

```bash
cargo test --manifest-path examples/cargo-fragile-cpp-demo/Cargo.toml
```
