# Plan: Task 22.3 - Set up libc++ for transpilation

## Objective
Add support for using libc++ (LLVM's C++ standard library) instead of libstdc++ for transpilation.

## Tasks

### 22.3.1 Add `-stdlib=libc++` flag to Clang invocation (~40 LOC)

1. Add `use_libcxx: bool` field to `ClangParser` struct
2. Update `build_compiler_args()` to add `-stdlib=libc++` when enabled
3. Add constructor variant `with_libcxx()` or similar

### 22.3.2 Document libc++ installation requirements (~30 LOC)

1. Update README or CLAUDE.md with installation instructions
2. For Ubuntu/Debian: `sudo apt install libc++-dev libc++abi-dev`

### 22.3.3 Handle libc++ include paths (~50 LOC)

1. Add `detect_libcxx_include_paths()` function
2. Search for libc++ headers at common locations:
   - `/usr/include/c++/v1/` (standard location)
   - `/usr/lib/llvm-{18,19}/include/c++/v1/` (versioned)
3. Use `-isystem` to include libc++ paths

## Implementation Details

### Changes to parse.rs

```rust
// Add field to ClangParser
pub struct ClangParser {
    // ... existing fields ...
    use_libcxx: bool,
}

// Add libc++ path detection
fn detect_libcxx_include_paths() -> Vec<String> {
    let candidates = [
        "/usr/include/c++/v1",
        "/usr/lib/llvm-19/include/c++/v1",
        "/usr/lib/llvm-18/include/c++/v1",
    ];
    candidates.iter()
        .filter(|p| Path::new(p).exists())
        .map(|s| s.to_string())
        .collect()
}

// Update build_compiler_args
fn build_compiler_args(&self) -> Vec<CString> {
    let mut args = vec![/* ... existing ... */];

    if self.use_libcxx {
        args.push("-stdlib=libc++");
    }
    // ...
}
```

### Changes to CLI

Add `--use-libcxx` flag to enable libc++ mode.

## Testing

1. Test that libc++ headers are found when installed
2. Test transpilation with libc++ for simple STL usage
3. Verify error handling when libc++ is not installed

## Estimated LOC: ~100-120
