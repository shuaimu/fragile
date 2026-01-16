# Plan: Constructors (A.2.2)

**Status:** Completed [26:01:15]

## Task
Implement constructor parsing for C++ classes (default, copy, move constructors).

## Analysis

### Current State
- `CppStruct` has fields and methods
- No constructor support
- No destructor support

### libclang Support
- `CXCursor_Constructor` (24) - cursor kind for constructors
- `clang_CXXConstructor_isDefaultConstructor()` - check if default constructor
- `clang_CXXConstructor_isCopyConstructor()` - check if copy constructor
- `clang_CXXConstructor_isMoveConstructor()` - check if move constructor

## Required Changes

### 1. AST Layer (ast.rs)
Add constructor node:
```rust
ConstructorDecl {
    class_name: String,
    params: Vec<(String, CppType)>,
    is_definition: bool,
    ctor_kind: ConstructorKind,
    access: AccessSpecifier,
},

#[derive(Debug, Clone, Copy)]
pub enum ConstructorKind {
    Default,
    Copy,
    Move,
    Other,
}
```

### 2. Parser Layer (parse.rs)
- Add handling for `CXCursor_Constructor`
- Query constructor type with `clang_CXXConstructor_is*` functions
- Extract parameters and access specifier

### 3. Data Structures (lib.rs)
Add to `CppStruct`:
```rust
pub constructors: Vec<CppConstructor>,

pub struct CppConstructor {
    pub params: Vec<(String, CppType)>,
    pub kind: ConstructorKind,
    pub access: AccessSpecifier,
    pub mir_body: Option<MirBody>,
}
```

### 4. Converter Layer (convert.rs)
- Convert constructor body to MIR (like functions)
- Store in struct's constructors list

## Estimated Changes
- ast.rs: ~20 lines
- parse.rs: ~40 lines
- lib.rs: ~20 lines
- convert.rs: ~40 lines
- tests: ~60 lines
Total: ~180 lines (well under 500 LOC)

## Test Cases
1. Default constructor (no params)
2. Copy constructor (const T&)
3. Move constructor (T&&)
4. Parameterized constructor
5. Multiple constructors in one class

## Implementation Summary

### Changes Made

1. **ast.rs**: Added `ConstructorKind` enum and `ConstructorDecl`/`DestructorDecl` to `ClangNodeKind`
2. **parse.rs**:
   - Added `CXCursor_Constructor` and `CXCursor_Destructor` handling
   - Added helper functions: `get_constructor_kind()`, `get_parent_class_name()`, `extract_params()`
3. **lib.rs**:
   - Added `CppConstructor` and `CppDestructor` structs
   - Added `constructors: Vec<CppConstructor>` and `destructor: Option<CppDestructor>` to `CppStruct`
   - Exported `ConstructorKind`
4. **convert.rs**:
   - Updated `convert_struct` to handle constructor and destructor nodes
   - Added `convert_constructor_body` and `convert_destructor_body` functions
5. **stubs.rs**: Updated tests to include new fields

### Test Coverage
- `test_default_constructor` - Default constructor parsing
- `test_copy_constructor` - Copy constructor detection
- `test_move_constructor` - Move constructor detection
- `test_parameterized_constructor` - Other (parameterized) constructor
- `test_multiple_constructors` - Multiple constructors in one class
- `test_destructor` - Destructor parsing
- `test_private_constructor` - Private constructor access specifier
