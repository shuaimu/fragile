# Plan: M5.7.2 - Build C++ Object Files

## Overview

This task adds the capability to compile C++ source files into object files using clang. This is a critical step toward linking C++ and Rust code together.

## Current State

- M5.7.1 added a compilation test that demonstrates the Rust → rustc pipeline works
- The test currently fails at link time because there are no C++ object files
- We need to generate `.o` files from `.cpp` sources using clang

## Design

### New Module: `cpp_compiler.rs`

Add a new module to `fragile-rustc-driver` that handles C++ compilation:

```rust
/// C++ compiler configuration
pub struct CppCompiler {
    /// Path to clang executable (defaults to "clang++")
    compiler: PathBuf,
    /// Include directories (-I flags)
    include_dirs: Vec<PathBuf>,
    /// System include directories (-isystem flags)
    system_include_dirs: Vec<PathBuf>,
    /// Preprocessor defines (-D flags)
    defines: Vec<String>,
    /// C++ standard (default: c++20)
    std_version: String,
    /// Optimization level (0-3)
    opt_level: u8,
    /// Generate debug info
    debug_info: bool,
    /// Position-independent code for shared libraries
    pic: bool,
}
```

### Key Functions

1. **`compile_to_object`**: Compile a single `.cpp` file to `.o`
2. **`compile_all`**: Compile multiple files, returning list of object paths
3. **`find_compiler`**: Locate clang++ or g++ on the system

### Integration with FragileDriver

Update `FragileDriver` to optionally compile C++ files:

```rust
impl FragileDriver {
    /// Compile C++ files to object files
    pub fn compile_cpp_objects(
        &self,
        cpp_files: &[&Path],
        output_dir: &Path,
        config: &CppCompilerConfig,
    ) -> Result<Vec<PathBuf>>;
}
```

### Stub Headers Path

The compilation needs access to our stub headers. We'll use the path:
`crates/fragile-clang/stubs/`

## Implementation Steps

### Step 1: Add CppCompiler struct (~100 LOC)

- Create `cpp_compiler.rs` module
- Implement `CppCompiler` struct with configuration
- Add `find_compiler()` to locate clang++

### Step 2: Implement compile_to_object (~80 LOC)

- Build command line arguments for clang
- Execute clang via `std::process::Command`
- Handle errors and return object file path

### Step 3: Add FragileDriver integration (~50 LOC)

- Add `compile_cpp_objects` method
- Update `compile` function to optionally build C++ objects

### Step 4: Add tests (~100 LOC)

- Test compiling add.cpp to add.o
- Test error handling for missing files
- Test include path handling

## Testing Strategy

1. **Unit test**: Compile add.cpp → add.o, verify .o file exists
2. **Integration test**: Compile multiple files
3. **Error test**: Handle missing files gracefully

## Estimated LOC

- cpp_compiler.rs: ~200 LOC
- Driver updates: ~50 LOC
- Tests: ~100 LOC
- **Total: ~350 LOC**

## Success Criteria

1. `cargo test --package fragile-rustc-driver test_compile_cpp_to_object` passes
2. Object files are generated in specified output directory
3. Object files can be used for linking (verified in M5.7.3)
