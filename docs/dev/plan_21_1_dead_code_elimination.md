# Plan: Dead Code Elimination (Task 21.1)

## Overview

Track unused functions and types during transpilation, with an option to omit them from output.

## Design

### Approach 1: Simple Reference Tracking (Chosen)

Track function/type declarations and their references in two passes:
1. First pass: Collect all declarations
2. Second pass: Track references during code generation
3. Post-process: Mark unreferenced items

### Implementation

1. Add tracking data structures to AstCodeGen:
   - `declared_functions: HashSet<String>` - all function names
   - `referenced_functions: HashSet<String>` - functions that are called
   - `entry_points: HashSet<String>` - main, exported functions

2. During expression processing, track CallExpr targets

3. After generation, compute unused = declared - referenced - entry_points

4. Add option `--eliminate-dead-code` to CLI for optional omission

## Simplification for 21.1.1

For task 21.1.1, just implement basic tracking:
- Track declared function names
- Track function calls/references
- Add `#[allow(dead_code)]` warnings as comments for unreferenced functions
- Do NOT omit code (that's task 21.1.3)

## Files to Modify

- `crates/fragile-clang/src/ast_codegen.rs`:
  - Add `declared_functions` and `referenced_functions` HashSets
  - Track function declarations in generate_function
  - Track function calls in expr_to_string for CallExpr

## Estimated LOC: ~80
