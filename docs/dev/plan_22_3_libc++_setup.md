# Plan: Task 22.3 - Vendor libc++ Source Code

## Objective

Vendor libc++ (LLVM's C++ standard library) for transpilation. The goal is to transpile
libc++ source code to Rust, rather than using special-case STL type mappings.

## Status: Complete ✅

All subtasks have been completed:

- **22.3.1** ✅ libc++ added via llvm-project git submodule at `vendor/llvm-project/libcxx`
- **22.3.2** ✅ Both `include/` (headers) and `src/` (library source) included via sparse checkout
- **22.3.3** ✅ Submodule tracks LLVM main branch (commit f091be6d5, Jan 2026)
- **22.3.4** ✅ License documented below

## Vendored libc++ Structure

```
vendor/llvm-project/
├── libcxx/
│   ├── include/    # Header files (templates, inline functions)
│   ├── src/        # Source files (compiled library components)
│   └── LICENSE.TXT # Apache 2.0 with LLVM Exception
└── LICENSE.TXT     # Apache 2.0 with LLVM Exception
```

The submodule uses sparse checkout to only fetch the `libcxx` directory.

## License

**libc++ is licensed under the Apache License v2.0 with LLVM Exceptions.**

This is a permissive license that allows:
- Commercial use
- Modification
- Distribution
- Patent use
- Private use

The LLVM Exception allows linking against compiled LLVM code without the
requirement to release source code under certain conditions.

Full license text: `vendor/llvm-project/libcxx/LICENSE.TXT`

## System libc++ Support (Separate Feature)

In addition to the vendored source, the transpiler also supports using system-installed
libc++ via the `--use-libcxx` CLI flag. This:

- Adds `-stdlib=libc++` to Clang invocation
- Auto-detects libc++ include paths
- Requires `apt install libc++-dev libc++abi-dev` on Debian/Ubuntu

This feature is useful for parsing C++ code that uses STL, but the long-term goal
is to transpile libc++ itself so no system dependencies are needed.

## Next Steps

The vendored libc++ source will be used in Phase 4 (Task 22.7+) to:
1. Transpile libc++ implementation to Rust
2. Replace special-case STL mappings with actual transpiled code
3. Achieve full STL compatibility through transpilation
