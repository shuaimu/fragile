# cargo-fragile-cpp-demo

Minimal standalone example showing a typical Cargo project that compiles C++
with Fragile via `build.rs`, without CMake.

## Run

```bash
cargo test --manifest-path examples/cargo-fragile-cpp-demo/Cargo.toml
```

Optional explicit compiler path:

```bash
FRAGILEC_BIN=/home/shuai/workspace/fragile/target/release/fragilec \
  cargo test --manifest-path examples/cargo-fragile-cpp-demo/Cargo.toml
```
