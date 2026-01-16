# Plan: M5.7.3 - Link Rust + C++ Objects

## Overview

This task adds the capability to link Rust binaries with C++ object files, completing the compilation pipeline from C++ source to executable.

## Current State

- M5.7.1: rustc driver integration working
- M5.7.2: C++ object file compilation working (CppCompiler module)
- Need to connect the two: pass C++ objects to rustc linker

## Design

### Approach: Use rustc's `-C link-arg` flag

Rustc can pass arbitrary arguments to the linker via `-C link-arg=<arg>`. For object files, we simply pass the path to each .o file.

### Implementation

Extend `run_rustc` in `rustc_integration.rs` to accept object file paths:

```rust
pub fn run_rustc(
    rust_files: &[&Path],
    cpp_stubs: &str,
    output: &Path,
    mir_registry: Arc<CppMirRegistry>,
    cpp_objects: &[PathBuf],  // NEW: C++ object files to link
) -> Result<()>
```

For each C++ object file, add:
```
args.push("-C".to_string());
args.push(format!("link-arg={}", obj.display()));
```

### C++ Runtime Linking

C++ code may require the C++ standard library. We need to link with:
- `-lstdc++` (GNU libstdc++) or
- `-lc++` (LLVM libc++)

Add this as a linker flag:
```
args.push("-C".to_string());
args.push("link-arg=-lstdc++".to_string());
```

### FragileDriver Updates

Add a method to perform full compilation with linking:

```rust
impl FragileDriver {
    /// Full compilation: parse C++, generate stubs, compile, link
    pub fn compile_with_cpp(
        &self,
        rust_files: &[&Path],
        cpp_files: &[&Path],
        output: &Path,
        config: Option<CppCompilerConfig>,
    ) -> Result<()> {
        // 1. Parse C++ files
        // 2. Generate stubs
        // 3. Compile C++ to objects
        // 4. Compile Rust with objects linked
    }
}
```

## Implementation Steps

### Step 1: Update run_rustc (~30 LOC)

- Add `cpp_objects` parameter
- Add `-C link-arg=<obj>` for each object
- Add `-C link-arg=-lstdc++` for C++ runtime

### Step 2: Update FragileDriver::compile (~20 LOC)

- Add optional `cpp_objects` parameter
- Pass objects through to run_rustc

### Step 3: Add compile_with_cpp method (~50 LOC)

- Full pipeline: parse → stubs → compile objects → link

### Step 4: Add integration test (~50 LOC)

- Parse add.cpp
- Generate stubs
- Compile C++ to object
- Link Rust main with C++ object
- Verify executable runs

## Testing Strategy

1. **Unit test**: Verify linker args are generated correctly
2. **Integration test**: Full pipeline from C++ source to executable
3. **Run test**: Execute generated binary and verify C++ function is called

## Estimated LOC

- rustc_integration.rs updates: ~30 LOC
- driver.rs updates: ~70 LOC
- Tests: ~80 LOC
- **Total: ~180 LOC**

## Success Criteria

1. Generate executable from Rust main + C++ add function
2. Execute binary and verify `add(2, 3)` returns 5
3. Tests pass on CI

## Known Issues

1. **Symbol mangling**: C++ function names may be mangled. Need to use `extern "C"` in C++ or handle mangled names.
2. **C++ runtime**: May need to detect which C++ runtime to link (libstdc++ vs libc++)
