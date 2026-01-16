# Plan: Dependent Type Representation (B.1.2.1)

**Status:** In Progress

## Design Rationale

### Problem
When a function template uses template parameters in its return type or parameter types, these types are "dependent" - they depend on the template arguments. For example:

```cpp
template<typename T>
T identity(T x) { return x; }
```

Here both the return type `T` and parameter type `T` are dependent types.

### Current State
The `CppType` enum uses `Named(String)` to represent template parameters:
- `T` becomes `CppType::Named("T")`
- This loses the information that `T` is a template parameter

### Solution
Add a new variant to `CppType` to explicitly represent template parameters:

```rust
CppType::TemplateParam {
    name: String,    // "T"
    depth: u32,      // Template nesting level
    index: u32,      // Index in template parameter list
}
```

### Design Decisions
1. **Explicit variant**: Use a dedicated variant instead of Named to distinguish template params
2. **Depth/Index tracking**: Store position info for nested templates and correct substitution
3. **Backward compatible**: Named continues to work for regular named types

## Implementation Plan

### Step 1: Extend CppType (types.rs)
Add TemplateParam variant with name, depth, and index.

### Step 2: Update Type Parsing (parse.rs)
When parsing types in template contexts, detect template parameter references and use the new variant.

### Step 3: Add Tests
Verify template parameters are correctly identified and stored.

## Estimated LOC
- types.rs: ~30 lines
- parse.rs: ~50 lines
- Tests: ~50 lines
- Total: ~130 lines
