# Plan: Bit Field Offset and Packing (Task 14.1.2)

## Overview

Track bit field offset and packing within structs to enable proper Rust code generation.

## Background

C++ bit fields allow compact storage of multiple values within a single storage unit:

```cpp
struct Flags {
    unsigned int a : 3;  // bits 0-2
    unsigned int b : 5;  // bits 3-7
    unsigned int c : 8;  // bits 8-15
    // Total: 16 bits, fits in u16
};
```

## Design

### Data Structure

Add a `BitFieldGroup` struct to track consecutive bit fields:

```rust
/// Represents a group of consecutive bit fields packed into a single storage unit
struct BitFieldInfo {
    /// Name of the original field
    field_name: String,
    /// Original type (for return type in accessor)
    original_type: CppType,
    /// Width in bits
    width: u32,
    /// Offset within the storage unit
    offset: u32,
    /// Access specifier
    access: AccessSpecifier,
}

struct BitFieldGroup {
    /// Fields in this group
    fields: Vec<BitFieldInfo>,
    /// Total bits used
    total_bits: u32,
}
```

### Algorithm

1. When processing struct fields, identify consecutive bit fields
2. Group them together based on:
   - Same underlying type OR can be packed together
   - Total bits don't exceed storage unit size
3. Track offset for each field within the group
4. Determine the smallest storage type that can hold all bits

### Storage Type Selection

| Total Bits | Storage Type |
|------------|--------------|
| 1-8        | u8           |
| 9-16       | u16          |
| 17-32      | u32          |
| 33-64      | u64          |
| 65-128     | u128         |

### Implementation Steps

1. **Modify `collect_struct_fields` or field processing**:
   - Detect bit fields (where `bit_field_width.is_some()`)
   - Group consecutive bit fields
   - Calculate offsets

2. **Store bit field metadata**:
   - Add `bit_field_groups: HashMap<String, Vec<BitFieldGroup>>` to CodeGenerator
   - Key is struct name, value is list of bit field groups

3. **Generate packed storage**:
   - For each group, generate a single storage field `_bitfield_N: uN`
   - Track field name to bit field group mapping

## Files to Modify

- `crates/fragile-clang/src/ast_codegen.rs`:
  - Add `BitFieldInfo` and `BitFieldGroup` structs
  - Add `bit_field_groups` field to `CodeGenerator`
  - Modify struct field generation to pack bit fields

## Estimated LOC: ~80

- Data structures: ~30 LOC
- Field grouping logic: ~50 LOC

## Test Cases

```cpp
// Simple bit fields
struct Simple {
    unsigned a : 1;
    unsigned b : 2;
    unsigned c : 5;
};

// Bit fields crossing storage boundary
struct CrossBoundary {
    unsigned a : 20;
    unsigned b : 20;  // Total 40 bits, needs u64
};

// Mixed bit fields and regular fields
struct Mixed {
    int x;
    unsigned a : 4;
    unsigned b : 4;
    int y;
};
```
