# Plan: Task 25.1 - VTable Data Structures Design

## Overview

Replace Rust trait-based polymorphism with explicit vtable structs that match C++ vtable implementation.

## Current State

The current implementation uses Rust traits for virtual dispatch:
```rust
pub trait exceptionTrait {
    fn what(&self) -> *const i8;
}
impl exceptionTrait for bad_alloc { ... }
```

This breaks for intermediate polymorphic classes (e.g., `bad_alloc` inherits from `exception` but is also a base for `bad_array_new_length`).

## Target Design

### VTableEntry (25.1.1)

```rust
/// Represents a single entry in a vtable (one virtual method)
#[derive(Clone, Debug)]
struct VTableEntry {
    /// Method name (e.g., "what", "__destructor")
    name: String,
    /// Return type
    return_type: CppType,
    /// Parameter types (excluding implicit self)
    params: Vec<(String, CppType)>,
    /// True if method is pure virtual (= 0)
    is_pure_virtual: bool,
    /// True if method is const (affects self mutability)
    is_const: bool,
    /// Class where this method was originally declared
    declaring_class: String,
    /// Index in vtable (assigned during vtable construction)
    vtable_index: usize,
}
```

### ClassVTableInfo (25.1.2)

```rust
/// Complete vtable information for a polymorphic class
#[derive(Clone, Debug)]
struct ClassVTableInfo {
    /// Class name this vtable is for
    class_name: String,
    /// All vtable entries (inherited + own)
    entries: Vec<VTableEntry>,
    /// Base class vtable (if single inheritance)
    /// For multiple inheritance, each polymorphic base has its own vtable
    base_vtable: Option<String>,
    /// True if class is abstract (has pure virtual methods)
    is_abstract: bool,
}
```

### Inheritance Tracking (25.1.3)

Already have `class_bases` HashMap tracking direct bases. Need to add:

```rust
/// Track which methods are overridden in derived classes
/// Key: (class_name, method_name)
/// Value: Original declaring class
method_overrides: HashMap<(String, String), String>,

/// Complete vtable info per polymorphic class
vtables: HashMap<String, ClassVTableInfo>,
```

### Multiple Inheritance (25.1.4)

For multiple inheritance with multiple polymorphic bases:

```rust
#[repr(C)]
struct derived {
    __vtable_base1: *const base1_vtable,
    __base1: base1_storage,  // fields only, no vtable ptr
    __vtable_base2: *const base2_vtable,
    __base2: base2_storage,
    // derived's own fields
}
```

Each polymorphic base gets its own vtable pointer at the start of its section.

## Implementation Steps

1. **25.1.1**: Add `VTableEntry` struct - extend existing `VirtualMethodInfo`
2. **25.1.2**: Add `ClassVTableInfo` struct
3. **25.1.3**: Add `method_overrides` and `vtables` to `RustCodeGen`
4. **25.1.4**: Update struct generation to handle multiple vtable pointers

## Estimated LOC

- VTableEntry struct: ~30 LOC
- ClassVTableInfo struct: ~20 LOC
- HashMap additions: ~10 LOC
- Total: ~60 LOC (well under 500)

## Testing

- Existing E2E tests for virtual methods should continue to pass
- Add test for intermediate polymorphic class (class that inherits and is inherited from)
