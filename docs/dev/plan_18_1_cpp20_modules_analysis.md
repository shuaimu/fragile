# C++20 Modules Support Analysis

## Summary

Task 18.1 (C++20 Module detection) requires significant work due to libclang limitations.

## Research Findings

### libclang Module Support Status

libclang has **minimal support** for C++20 modules:

1. **Supported**: `CXCursor_ModuleImportDecl` (value: 600) - for `import` declarations
2. **NOT supported** as dedicated cursor kinds:
   - Module declarations (`module foo;` or `export module foo;`)
   - Export declarations (`export` keyword)
   - Module partitions (private/implementation module units)

### Available libclang Functions

```rust
clang_Cursor_getModule()        // Get module from a cursor
clang_getModuleForFile()        // Get module associated with a file
clang_Module_getName()          // Retrieve module name
clang_Module_getFullName()      // Get fully qualified module name
clang_Module_getParent()        // Access parent module hierarchy
```

### Implementation Challenges

1. **For `import` declarations**: Can use `CXCursor_ModuleImportDecl` directly (straightforward)
2. **For `module` and `export module`**: Must use tokenization/source parsing since libclang doesn't expose dedicated cursors
3. **For module partitions**: Require custom source-level parsing
4. **For `export` visibility**: May need to parse source tokens or use `UnexposedDecl` analysis

### Clang Version Requirements

- Fragile uses Clang 17.0 (via `clang-sys` feature `clang_17_0`)
- Clang 17.0 has solid C++20 module compilation support
- Module import parsing is available
- Module declarations require custom parsing beyond libclang cursors

### Known Clang Limitations (as of Clang 17-23)

- `#include` and `import` order sensitivity
- Private module fragment introducer is ignored
- Some export scenarios fail validation
- Export syntax checking has issues

## Recommended Approach

### Phase 1: Import Support (Simpler)
- Add `CXCursor_ModuleImportDecl` handling in parse.rs
- Extract module name using `clang_Cursor_getModule()` and `clang_Module_getName()`
- Generate `use` statements or extern crate references

### Phase 2: Module/Export Declarations (Complex)
- Implement token-based parsing for `module`, `export module` keywords
- Parse source file to find module declarations at file scope
- Track export visibility at declaration level

### Phase 3: Module Partitions (Complex)
- Parse partition syntax (`:` in module names)
- Map to Rust submodule structure

## Estimated Effort

- Import support only: ~100 LOC, 1-2 hours
- Full module support: ~500+ LOC, significant complexity due to token parsing

## Decision

Given the complexity and low priority, this task should remain deferred until:
1. libclang adds better module support, OR
2. There's a specific need for C++20 module transpilation

## References

- [clang-sys Rust bindings](https://docs.rs/clang-sys/latest/clang_sys/)
- [Clang 17.0.1 Standard C++ Modules](https://releases.llvm.org/17.0.1/tools/clang/docs/StandardCPlusPlusModules.html)
- [Clang Module introspection](https://clang.llvm.org/doxygen/group__CINDEX__MODULE.html)
