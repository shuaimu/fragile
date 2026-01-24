# Plan: Two-Pass Namespace Merging (23.8.2)

## Problem Statement

C++ allows reopening namespaces - the same namespace can be declared multiple times, each time adding new items:

```cpp
namespace foo { struct A {}; }
namespace foo { struct B {}; }  // Reopens foo, adds B
```

Rust modules cannot be reopened. Once a `mod foo {}` is declared, you can't add more items to it later.

Current approach: Skip duplicate namespace occurrences entirely (line 1593-1596 in ast_codegen.rs). This loses items defined in subsequent namespace occurrences.

## Solution: Two-Pass Namespace Merging

### Pass 1: Collect Namespace Contents

Before generating any code, traverse the entire AST and collect:
- For each namespace path (e.g., "std::__1::vector"), collect all child nodes from all occurrences
- Merge children from multiple namespace declarations with the same path

Data structure:
```rust
struct MergedNamespace {
    children: Vec<ClangNode>,
    is_anonymous: bool,
}
HashMap<String, MergedNamespace>  // namespace_path -> merged contents
```

### Pass 2: Generate Modules with Merged Contents

When generating a namespace:
1. Look up the merged children for this namespace path
2. Generate the module with all merged children at once
3. Mark as generated to skip subsequent occurrences

### Implementation Steps

1. **Add namespace collection data structure** (~20 LOC)
   - Add `merged_namespaces: HashMap<String, Vec<ClangNode>>` field
   - Add helper to compute namespace path

2. **Add first pass collection function** (~80 LOC)
   - `collect_namespace_contents(node: &ClangNode, current_path: Vec<String>)`
   - Recursively traverse AST
   - For NamespaceDecl nodes, extend merged_namespaces map
   - Don't generate any code in this pass

3. **Modify second pass generation** (~60 LOC)
   - When hitting NamespaceDecl, check if already generated
   - If not, look up merged children and generate all at once
   - Mark as generated

4. **Handle edge cases** (~40 LOC)
   - Anonymous namespaces get unique synthetic names
   - Inline namespaces (std::__1) are flattened
   - Nested namespaces need proper path computation

### Estimated LOC: ~200 lines

### Files to Modify
- `crates/fragile-clang/src/ast_codegen.rs` - main changes

### Testing
- Existing tests should pass
- Add test for namespace reopening pattern
- Verify std::vector error count decreases

## Alternative Considered: Collect All Then Sort

Could collect all top-level items, sort by namespace, then generate. But this would require significant restructuring of the entire codegen approach and is more invasive.

The two-pass approach is more surgical and preserves existing structure.
