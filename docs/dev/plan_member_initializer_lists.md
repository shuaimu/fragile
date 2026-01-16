# Plan: Member Initializer Lists (A.2)

**Status:** Completed [26:01:16]

## Task
Implement parsing of C++ member initializer lists in constructors.

## Analysis

### Current State
- Constructors are parsed with params, kind, and access specifier
- No support for member initializer lists like `MyClass(int x) : member(x) { }`

### libclang Support
- `CXCursor_MemberRef` - cursor kind for member references in initializer lists
- Appears as children of `CXCursor_Constructor`
- Contains member name

## Required Changes

### 1. AST Layer (ast.rs)
No changes needed - `CXCursor_CXXMemInitializer` children will be converted to ClangNode.

### 2. Data Structures (lib.rs)
Add to support member initializers:
```rust
pub struct MemberInitializer {
    pub member_name: String,
    pub init_value: MirOperand,  // The initialization value (simplified)
}

// Update CppConstructor
pub struct CppConstructor {
    pub params: Vec<(String, CppType)>,
    pub kind: ConstructorKind,
    pub access: AccessSpecifier,
    pub member_initializers: Vec<MemberInitializer>,
    pub mir_body: Option<MirBody>,
}
```

### 3. Parser Layer (parse.rs)
- Add handling for `CXCursor_CXXMemInitializer` in convert_cursor_kind

### 4. Converter Layer (convert.rs)
- Extract member initializers from constructor children
- Convert initialization expressions to MirOperand

## Estimated Changes
- ast.rs: ~10 lines (new node kind)
- lib.rs: ~15 lines
- parse.rs: ~15 lines
- convert.rs: ~30 lines
- tests: ~50 lines
Total: ~120 lines (well under 500 LOC)

## Test Cases
1. Simple member initializer `Foo() : x(0) { }`
2. Multiple member initializers `Foo() : x(1), y(2) { }`
3. Initializer with parameter `Foo(int a) : x(a) { }`
4. Base class initializer (for future inheritance support)

## Implementation Summary

### Changes Made

1. **ast.rs**: Added `MemberRef { name: String }` variant to `ClangNodeKind`
2. **parse.rs**: Added handling for `CXCursor_MemberRef` to extract member names
3. **lib.rs**:
   - Added `MemberInitializer` struct with `member_name` and `has_init` fields
   - Added `member_initializers: Vec<MemberInitializer>` to `CppConstructor`
4. **convert.rs**: Added `extract_member_initializers()` method to parse MemberRef children

### Test Coverage
- `test_parse_constructor_with_initializer_list` - Unit test for AST parsing
- `test_member_initializer_list` - Multiple member initializers
- `test_single_member_initializer` - Single member initializer
- `test_no_member_initializer` - Constructor without initializer list
