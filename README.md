# Fragile

A unified programming language with three syntaxes: **Rust**, **C++20**, and **Go**.

All code compiles to the same IR with identical semantics — no FFI, no marshalling, zero overhead.

## Example

```rust
// math.rs
pub fn add(a: i32, b: i32) -> i32 { a + b }
```

```go
// main.go
import "math.rs"

func main() {
    result := add(2, 3)  // Direct call to Rust function
}
```

## Building

Prerequisites: LLVM 19, Rust 1.75+

```bash
cargo build --release
```

## Usage

```bash
fragile main.go math.rs -o program
./program
```

## Documentation

See [docs/fragile-book.md](docs/fragile-book.md) for the full language specification.

## License

MIT
