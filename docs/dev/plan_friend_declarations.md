# Plan: Friend Declarations (A.2)

**Status**: ✅ Completed [26:01:16]

## Task
Implement parsing of friend declarations in C++ classes.

## Analysis

### Current State
- Class members (fields, methods, constructors, destructors) are parsed
- No support for friend declarations

### libclang Support
- `CXCursor_FriendDecl` (603) - cursor kind for friend declarations
- Children of FriendDecl can be:
  - `CXCursor_ClassDecl` / `CXCursor_CXXRecordDecl` - for friend classes
  - `CXCursor_FunctionDecl` - for friend functions

### Friend Types
1. **Friend class**: `friend class Bar;`
2. **Friend function declaration**: `friend void helper(Foo& f);`
3. **Friend function definition**: `friend int get_value(const Foo& f) { ... }`

## Required Changes

### 1. AST Layer (ast.rs)
- Add `FriendDecl` variant with kind (class vs function)

### 2. Parser Layer (parse.rs)
- Handle `CXCursor_FriendDecl`
- Extract child cursor to determine friend type

### 3. Data Structures (lib.rs)
- Add `CppFriend` enum for friend types
- Add `friends: Vec<CppFriend>` to `CppStruct`

### 4. Converter Layer (convert.rs)
- Pass friends through conversion

## Estimated Changes
- ast.rs: ~10 lines
- parse.rs: ~25 lines
- lib.rs: ~15 lines
- convert.rs: ~15 lines
- tests: ~50 lines
Total: ~115 lines (well under 500 LOC)

## Test Cases
1. Friend class `friend class Bar;`
2. Friend function declaration `friend void helper(Foo& f);`
3. Friend function definition `friend int get_value(const Foo&) { ... }`

## Implementation Summary

### Changes Made

1. **ast.rs**:
   - Added `FriendDecl` variant with `friend_class: Option<String>` and `friend_function: Option<String>` fields

2. **parse.rs**:
   - Added `CXCursor_FriendDecl` handler
   - Added `get_friend_info()` helper function to extract friend type from children
   - Handles `CXCursor_ClassDecl`, `CXCursor_StructDecl`, `CXCursor_ClassTemplate` for friend classes
   - Handles `CXCursor_TypeRef` for forward-declared friend classes (strips "class "/"struct " prefix)
   - Handles `CXCursor_FunctionDecl` for friend functions

3. **lib.rs**:
   - Added `CppFriend` enum with `Class { name }` and `Function { name }` variants
   - Added `friends: Vec<CppFriend>` field to `CppStruct`

4. **convert.rs**:
   - Updated `convert_struct()` to handle `FriendDecl` nodes
   - Passes friends through to the returned `CppStruct`

5. **stubs.rs**:
   - Updated test structs to include `friends: vec![]`

### Tests Added
- `test_friend_class` - tests `friend class Bar;`
- `test_friend_function` - tests `friend void helper(Foo& f);`
- `test_multiple_friends` - tests combination of friend classes and functions

All 35 tests pass (29 fragile-clang + 6 fragile-rustc-driver).
