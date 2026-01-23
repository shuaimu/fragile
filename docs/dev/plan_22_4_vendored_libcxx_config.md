# Plan: Task 22.4 - Configure build to use vendored libc++

## Objective

Allow the transpiler to use the vendored libc++ source code from `vendor/llvm-project/libcxx/`
instead of system-installed libc++.

## Task Breakdown

### 22.4.1 Point Clang to vendored `include/` directory (~60 LOC)

1. Add `detect_vendored_libcxx_path()` function to find vendored libc++ relative to project root
2. Add `with_vendored_libcxx()` constructor that uses vendored headers
3. Add `--use-vendored-libcxx` CLI flag to enable this mode
4. Fall back to system libc++ if vendored not found

**Implementation:**
- Detect vendored path by looking for `vendor/llvm-project/libcxx/include/` relative to:
  - Current working directory
  - Executable location
  - Environment variable `FRAGILE_ROOT`

### 22.4.2 Transpile `src/*.cpp` files as part of STL support (future)

This is a larger task that involves:
- Compiling libc++ source files to Rust
- Handling internal implementation details
- This will be done incrementally as we need specific STL features

**Not implemented in this task - deferred to when we need specific STL features.**

### 22.4.3 Handle libc++ build configuration macros (~20 LOC)

Common macros to define for libc++ compatibility:
- `_LIBCPP_HAS_NO_PRAGMA_SYSTEM_HEADER` - Disable pragma system_header
- `_LIBCPP_DISABLE_DEPRECATION_WARNINGS` - Suppress deprecation warnings
- `_LIBCPP_NO_EXCEPTIONS` - Optional, if we want to disable exception support

**Implementation:**
- Add default defines to `build_compiler_args()` when using vendored libc++

## Estimated LOC: ~80 for 22.4.1 + 22.4.3

## Implementation Details

### Changes to parse.rs

```rust
/// Detect vendored libc++ include path.
/// Looks for vendor/llvm-project/libcxx/include/ relative to:
/// 1. Environment variable FRAGILE_ROOT
/// 2. Current working directory
/// 3. Executable's parent directories
pub fn detect_vendored_libcxx_path() -> Option<String> {
    let vendored_subpath = "vendor/llvm-project/libcxx/include";

    // Try FRAGILE_ROOT environment variable
    if let Ok(root) = std::env::var("FRAGILE_ROOT") {
        let path = Path::new(&root).join(vendored_subpath);
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    // Try current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let path = cwd.join(vendored_subpath);
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    // Try executable's location
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        while let Some(parent) = dir {
            let path = parent.join(vendored_subpath);
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
            dir = parent.parent();
        }
    }

    None
}

/// Create a Clang parser configured to use vendored libc++.
pub fn with_vendored_libcxx() -> Result<Self> {
    let vendored_path = Self::detect_vendored_libcxx_path()
        .ok_or_else(|| miette!("Vendored libc++ not found"))?;

    let system_paths = vec![vendored_path];
    let defines = vec![
        "_LIBCPP_HAS_NO_PRAGMA_SYSTEM_HEADER".to_string(),
    ];

    Self::with_full_options(Vec::new(), system_paths, defines, Vec::new(), true)
}
```

### Changes to CLI

Add `--use-vendored-libcxx` flag that uses `with_vendored_libcxx()`.
