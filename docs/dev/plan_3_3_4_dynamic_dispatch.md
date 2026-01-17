# Plan: Task 3.3.4 - Dynamic Dispatch

## Overview

Implement dynamic dispatch for virtual method calls in C++. When calling a virtual method,
the actual function called is determined at runtime via vtable lookup.

## Current Status

Already implemented:
- `MirTerminator::VirtualCall` with receiver, vtable_index, args
- Helper functions: `try_extract_member_call()`, `unwrap_casts()`, `extract_class_name()`
- `MemberCallInfo` struct for holding class_name, method_name, is_arrow info
- Vtable generation for polymorphic classes (`CppVtable`, `VtableEntry`)

## Subtasks

### 3.3.4a - Add vtable index lookup helper

**Goal**: Add a method to find the vtable index for a given virtual method.

**Location**: `convert.rs`

**Implementation**:
```rust
/// Find the vtable index for a virtual method call.
/// Returns None if the method is not virtual or class not found.
fn find_vtable_index(&self, class_name: &str, method_name: &str) -> Option<usize> {
    // 1. Find the vtable for this class in self.module.vtables
    // 2. Search entries for matching method_name
    // 3. Return the index if found
}
```

**Considerations**:
- Method overloading: For now, match by name only (TODO: signature matching)
- Base class methods: Currently vtable only has direct methods (TODO: inherited virtuals)

### 3.3.4b - Integrate VirtualCall in CallExpr handling

**Goal**: Generate `MirTerminator::VirtualCall` instead of regular `Call` for virtual method calls.

**Location**: `convert.rs`, around line 1002 in `CallExpr` handling

**Implementation**:
1. After extracting `func_name`, call `try_extract_member_call()`
2. If it returns `Some(MemberCallInfo)`:
   a. Look up the class in `self.module.structs`
   b. Find the method in `class.methods`
   c. If `method.is_virtual`:
      - Call `find_vtable_index()` to get the index
      - Generate `MirTerminator::VirtualCall`
   d. Else: Generate regular `MirTerminator::Call`
3. If `None`: Generate regular `MirTerminator::Call`

**Code Structure**:
```rust
ClangNodeKind::CallExpr { ty } => {
    if let Some(func_ref) = node.children.first() {
        // Check for member call (obj.method() or ptr->method())
        if let Some(member_info) = Self::try_extract_member_call(func_ref) {
            // Look up class and check if method is virtual
            if let Some(vtable_index) = self.find_vtable_index(&member_info.class_name, &member_info.method_name) {
                // Generate VirtualCall
                ...
            }
        }

        // Fall through to regular call handling
        ...
    }
}
```

### 3.3.4c - rustc-driver VirtualCall translation

**Goal**: Translate `MirTerminator::VirtualCall` to rustc MIR in `mir_convert.rs`.

**Location**: `fragile-rustc-driver/src/mir_convert.rs`

**Implementation Options**:

Option 1: Direct vtable lookup (complex)
- Load vtable pointer from receiver's first field
- Index into vtable to get function pointer
- Generate indirect call through function pointer

Option 2: Generate call to known function name (simpler, for now)
- Since we have the vtable_index and class name, we can reconstruct the function
- Generate a regular call to the mangled method name
- This loses dynamic dispatch but proves the pipeline works

**Recommendation**: Start with Option 2 for correctness testing, then implement Option 1.

### 3.3.4d - Add comprehensive tests

**Tests to add**:
1. `test_virtual_call_generation` - Verify VirtualCall is generated for virtual method calls
2. `test_non_virtual_call_generation` - Verify regular Call for non-virtual methods
3. `test_vtable_index_lookup` - Verify correct vtable index resolution
4. `test_virtual_call_with_args` - Virtual call with multiple arguments
5. `test_virtual_call_override` - Call to overridden method

## Test Case

```cpp
// tests/cpp/virtual_dispatch.cpp
class Animal {
public:
    virtual int speak() { return 1; }
};

class Dog : public Animal {
public:
    int speak() override { return 2; }
};

int call_speak(Animal* a) {
    return a->speak();  // Should generate VirtualCall
}
```

## Risk Assessment

- **Low risk**: The vtable infrastructure already exists
- **Medium risk**: Method signature matching for overloaded methods
- **Low priority**: Base class inherited virtual methods (can be deferred)

## Implementation Order

1. 3.3.4a - Vtable index lookup (prerequisite for 3.3.4b)
2. 3.3.4b - CallExpr integration (core functionality)
3. 3.3.4d - Tests (verify correctness)
4. 3.3.4c - rustc translation (can be deferred if needed)
