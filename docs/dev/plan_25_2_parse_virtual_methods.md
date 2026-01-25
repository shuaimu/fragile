# Plan: Task 25.2 - Parse Virtual Method Information

## Overview

Implement vtable construction by collecting virtual methods from class hierarchies, merging overrides, and building complete ClassVTableInfo structures.

## Current State

The `analyze_class()` function already collects virtual methods per class in `virtual_methods` HashMap. However:
1. It doesn't merge inherited virtual methods
2. It doesn't track which methods override which
3. It doesn't build `ClassVTableInfo` structures
4. Pure virtual methods are tracked but not used for vtable construction

## Implementation Steps

### 25.2.1 Collect all virtual methods from class and bases (merge overrides)

Create a new function `build_vtable_for_class()` that:
1. Gets the base class (if any) from `class_bases`
2. Recursively gets base's vtable entries
3. Appends own virtual methods, replacing overridden entries

```rust
fn build_vtable_for_class(&mut self, class_name: &str) -> ClassVTableInfo {
    // Get base class vtable first
    let base_entries = if let Some(bases) = self.class_bases.get(class_name) {
        if let Some(primary_base) = bases.first() {
            // Recursively build base vtable if not already done
            self.build_vtable_for_class(&primary_base.name).entries.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // ... merge with own methods
}
```

### 25.2.2 Track which methods are overridden vs inherited

Update `method_overrides` HashMap when an override is detected:
- Key: (derived_class, method_name)
- Value: base class where method was originally declared

### 25.2.3 Handle pure virtual methods (= 0)

Mark `is_abstract = true` on ClassVTableInfo if any entry has `is_pure_virtual = true`.

### 25.2.4 Handle final methods

Final methods cannot be overridden. Track in VTableEntry if needed (already have `is_final` in CXXMethodDecl).

## Testing

- Existing E2E tests for virtual methods should pass
- Test that vtables HashMap is populated correctly
- Test inheritance chain vtable merging

## Estimated LOC

- build_vtable_for_class function: ~80 LOC
- Call from analyze pass: ~10 LOC
- Tests: ~50 LOC
- Total: ~140 LOC (well under 500)
