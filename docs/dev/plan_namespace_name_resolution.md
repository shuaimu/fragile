# Plan: Namespace Name Resolution (A.1.4)

**Status:** Completed [26:01:16, 00:53]

## Design Rationale

### Problem
When C++ code references a name (e.g., function call, variable), the unqualified name must be resolved to its fully qualified form. This requires:
1. Looking up names in the current scope
2. Searching parent scopes (enclosing namespaces)
3. Consulting `using namespace` directives to find imported names
4. Handling `using` declarations for specific imported names

### Solution
Implemented a `NameResolver` that:
1. Builds an index of all declarations (functions, structs, externs) by their qualified names
2. Provides a `resolve_function()` method for function lookup
3. Provides a `resolve_type()` method for type lookup
4. Searches following C++ lookup rules:
   - Current scope (namespace or class)
   - Enclosing scopes (walking up the namespace hierarchy)
   - Imported namespaces (via `using namespace`)
   - Specific imported names (via `using`)

### Design Decisions
1. **Two-phase approach**: First index all declarations, then resolve as post-processing
2. **Simple lookup**: No overload resolution or ADL in this phase (future work)
3. **Store qualified names in MIR**: Function calls use fully qualified names (e.g., "foo::helper")
4. **Post-processing resolution**: `CppModule::resolve_names()` applies resolution after conversion

## Implementation Summary

### Files Changed

1. **crates/fragile-clang/src/resolve.rs** (NEW - ~200 lines)
   - `NameResolver` struct with function/type indexes
   - `resolve_function()` for function name lookup
   - `resolve_type()` for type name lookup
   - `is_scope_visible()` for using directive scope checking
   - `format_qualified_name()` helper
   - 8 unit tests covering all lookup scenarios

2. **crates/fragile-clang/src/lib.rs** (~100 lines added)
   - Added `mod resolve` and `pub use resolve::NameResolver`
   - Added `CppModule::resolve_names()` method for post-processing
   - Added `CppModule::collect_mir_resolutions()` helper

3. **crates/fragile-clang/src/convert.rs** (~25 lines added)
   - Added `extract_function_name()` to unwrap Clang AST wrappers
   - Fixed function name extraction from `CallExpr` nodes
   - Now handles `Unknown("UnexposedExpr")` wrapper nodes

4. **crates/fragile-clang/tests/integration_test.rs** (~110 lines added)
   - `test_name_resolution_same_namespace`
   - `test_name_resolution_using_namespace`
   - `test_name_resolution_global_from_namespace`

5. **tests/clang_integration/namespace_resolution.cpp** (NEW)
   - Test file for namespace name resolution scenarios

### Test Coverage
- 8 unit tests in resolve.rs
- 3 integration tests in integration_test.rs
- All 83 tests passing

## User Guide

### Basic Usage
Name resolution is automatically applied when using `compile_cpp_file()`:

```rust
use fragile_clang::compile_cpp_file;
use std::path::Path;

let module = compile_cpp_file(Path::new("source.cpp"))?;
// Function calls are now fully qualified (e.g., "foo::helper")
```

### Manual Resolution
For more control, use the converter directly:

```rust
use fragile_clang::{ClangParser, MirConverter, NameResolver};

let parser = ClangParser::new()?;
let ast = parser.parse_file(path)?;
let converter = MirConverter::new();
let mut module = converter.convert(ast)?;

// Apply name resolution separately
module.resolve_names();
```

### Using the NameResolver Directly
For custom resolution needs:

```rust
use fragile_clang::NameResolver;

let resolver = NameResolver::new(&module);

// Resolve a function name from a specific scope
let qualified = resolver.resolve_function("helper", &["foo".into()]);
// Returns Some(["foo", "helper"]) if found

// Format as qualified name string
if let Some(q) = qualified {
    let name_str = NameResolver::format_qualified_name(&q);  // "foo::helper"
}
```

## Limitations

- **No overload resolution**: Multiple functions with the same name are not differentiated
- **No ADL (Argument-Dependent Lookup)**: Future enhancement
- **No template argument deduction**: Templates handled separately
- **Scope limited to namespaces**: Class member lookup not yet implemented
