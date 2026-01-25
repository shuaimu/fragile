# Plan: Task 25.9 - Update dynamic_cast with RTTI

## Problem

The current dynamic_cast implementation is a placeholder that just does a direct pointer cast without runtime type checking:

```rust
/* dynamic_cast: returns null on failure */ ptr as *mut Target
```

This is incorrect - dynamic_cast should check at runtime whether the object's actual type is compatible with the target type, and return nullptr if not.

## Solution: RTTI in VTable

Add type information to vtables that allows runtime type checking.

### Data Structures

1. **Type identifier in vtable**: Add a unique type ID to each vtable
2. **Base class info**: Store base class type IDs for upcast/downcast validation

### Implementation Steps

#### 25.9.1 Add type_info field to vtable struct

```rust
#[repr(C)]
pub struct Base_vtable {
    pub __type_id: u64,                    // Unique hash for this class
    pub __base_type_ids: &'static [u64],   // Array of ancestor type IDs
    pub method1: unsafe fn(*mut Base) -> i32,
    // ... other methods
    pub __destructor: unsafe fn(*mut Base),
}
```

Each vtable has:
- `__type_id`: A unique identifier for this class (hash of class name)
- `__base_type_ids`: Slice of ancestor class type IDs (for downcast checking)

#### 25.9.2 Generate type_info constants

For each polymorphic class, generate:

```rust
/// Type ID for class `Derived`
pub const DERIVED_TYPE_ID: u64 = 0x1234567890abcdef; // Hash of "Derived"

/// Base class type IDs for `Derived` (includes self and all ancestors)
pub static DERIVED_BASE_TYPE_IDS: [u64; 3] = [
    DERIVED_TYPE_ID,  // Self
    MIDDLE_TYPE_ID,   // Direct base
    BASE_TYPE_ID,     // Root
];
```

#### 25.9.3 Implement dynamic_cast with type checking

```rust
/// Dynamic cast from base pointer to derived pointer.
/// Returns null if the actual type is not derived from Target.
pub unsafe fn dynamic_cast<Target>(ptr: *mut Base) -> *mut Target {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    // Get the vtable's type_id array
    let vtable = (*ptr).__vtable;
    let base_type_ids = (*vtable).__base_type_ids;

    // Check if Target's type_id is in the base_type_ids array
    let target_type_id = TARGET_TYPE_ID; // Would be a const for each target type

    for &type_id in base_type_ids {
        if type_id == target_type_id {
            return ptr as *mut Target;
        }
    }

    std::ptr::null_mut()
}
```

### Complexity Estimate

- 25.9.1: ~100 LOC (vtable struct generation changes)
- 25.9.2: ~80 LOC (type_info constant generation)
- 25.9.3: ~100 LOC (dynamic_cast implementation)

Total: ~280 LOC

### Alternative Approach

Instead of runtime type checking, we could:
1. Use Rust's std::any::TypeId with Box<dyn Any> - but this doesn't work with raw pointers
2. Just trust the programmer and do unchecked casts - but this is unsafe and can cause UB

The vtable RTTI approach matches how C++ actually implements dynamic_cast.

### Testing

1. Test downcast success: `dynamic_cast<Derived*>(base_ptr)` when object is actually Derived
2. Test downcast failure: `dynamic_cast<Derived*>(base_ptr)` when object is actually Base
3. Test cross-cast failure: `dynamic_cast<SiblingA*>(sibling_b_ptr)`
4. Test reference dynamic_cast (throws std::bad_cast on failure)

## Decision

Proceed with vtable RTTI approach as it:
1. Matches C++ semantics exactly
2. Works with raw pointers (no boxing needed)
3. Has O(n) runtime where n = inheritance depth (typically small)
4. Is how real C++ compilers implement it
