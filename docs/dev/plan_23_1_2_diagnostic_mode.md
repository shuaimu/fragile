# Plan: Diagnostic Mode for AST Node Debugging (23.1.2)

## Goal
Add optional diagnostic logging to help debug issues with:
1. Unknown AST nodes during transpilation
2. Type conversion failures

## Design

### 1. Add diagnostic flag to CodeGenerator
- Add `diagnostic_mode: bool` field to `CodeGenerator` struct
- When enabled, log additional information about problematic nodes

### 2. Log Unknown AST nodes
- In `expr_to_string()` when handling `ClangNodeKind::Unknown`
- Log: cursor kind string, source location if available
- Example: `[DIAG] Unknown node: CXCursor_XXX at file.cpp:123`

### 3. Log type conversion issues
- In `to_rust_type_str()` when falling back to c_void or sanitized identifier
- Log: original C++ type spelling, what it was converted to
- Example: `[DIAG] Type fallback: 'std::__1::__compressed_pair<T, A>' -> c_void`

### 4. Enable via environment variable
- Check `FRAGILE_DIAGNOSTIC=1` environment variable
- No need to change API or CLI for now

## Implementation

### Files to modify
1. `crates/fragile-clang/src/ast_codegen.rs`
   - Add `diagnostic_mode` field
   - Initialize from env var in `new()`
   - Add `log_diagnostic()` helper method
   - Add logging in `expr_to_string()` for Unknown nodes

2. `crates/fragile-clang/src/types.rs`
   - Add diagnostic logging for type fallbacks (optional, can be done via eprintln for now)

### Estimated LOC: ~40 lines

## Testing
- Run transpilation with `FRAGILE_DIAGNOSTIC=1` to see output
- Verify Unknown nodes and type issues are logged
