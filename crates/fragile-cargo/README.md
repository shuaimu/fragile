# fragile-cargo

Cargo `build.rs` helpers for building C/C++ targets described in `fragile.toml`.

Current scope:
- Supports `static_library`, `shared_library`, and `executable` targets
- Builds internal target dependencies
- Compiles C/C++ units through `fragilec` (via `fragile-driver`)
- Emits linked artifacts into Cargo `OUT_DIR` (`.a`, `.so`, executable)
- Emits `cargo:rustc-link-*` directives

Link directive behavior by root target type:
- `static_library`: emits root static lib and internal library deps in link order
- `shared_library`: emits root shared lib (and shared deps if any)
- `executable`: builds executable artifact and emits a Cargo warning with output path (no rustc link flags)

## Usage

Add as a build dependency:

```toml
[build-dependencies]
fragile-cargo = { path = "../path/to/fragile/crates/fragile-cargo" }
```

Use from `build.rs`:

```rust
fn main() {
    fragile_cargo::build_target("fragile.toml", "cppcore")
        .expect("failed to build C++ static library with Fragile");
}
```

Environment notes:
- `OUT_DIR` is provided by Cargo.
- `FRAGILEC_BIN` can be set to an explicit `fragilec` path.
- If `FRAGILEC_MODE` is unset, strict mode is used by the underlying driver.
