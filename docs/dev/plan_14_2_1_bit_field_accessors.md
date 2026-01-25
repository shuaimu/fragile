# Plan: Bit Field Getter/Setter Methods (Task 14.2.1)

## Overview

Generate getter and setter methods for bit field access to provide type-safe access to packed bit fields.

## Background

With bit field packing (14.1.2) implemented, we now have:
- `_bitfield_N` storage fields
- `BitFieldGroup` with `BitFieldInfo` metadata

We need accessor methods like:
```rust
impl Flags {
    pub fn a(&self) -> u32 { ... }
    pub fn set_a(&mut self, v: u32) { ... }
}
```

## Design

### Getter Method

```rust
pub fn field_name(&self) -> original_type {
    let mask = (1 << width) - 1;
    ((self._bitfield_N >> offset) & mask) as original_type
}
```

### Setter Method

```rust
pub fn set_field_name(&mut self, v: original_type) {
    let mask = (1 << width) - 1;
    let shifted_mask = mask << offset;
    self._bitfield_N = (self._bitfield_N & !shifted_mask) | (((v as storage_type) & mask) << offset);
}
```

## Implementation Steps

1. After struct definition, if struct has bit field groups, generate impl block
2. For each BitFieldInfo in each BitFieldGroup:
   - Generate getter method
   - Generate setter method
3. Handle visibility based on access specifier

## Files to Modify

- `crates/fragile-clang/src/ast_codegen.rs`:
  - Add `generate_bit_field_accessors` method
  - Call after `generate_struct` completes

## Example Output

```cpp
// C++ input:
struct Flags {
    unsigned a : 3;
    unsigned b : 5;
};
```

```rust
// Rust output:
#[repr(C)]
pub struct Flags {
    pub _bitfield_0: u8,
}

impl Flags {
    pub fn a(&self) -> u32 {
        (self._bitfield_0 & 0x7) as u32
    }
    pub fn set_a(&mut self, v: u32) {
        self._bitfield_0 = (self._bitfield_0 & !0x7) | ((v as u8) & 0x7);
    }
    pub fn b(&self) -> u32 {
        ((self._bitfield_0 >> 3) & 0x1F) as u32
    }
    pub fn set_b(&mut self, v: u32) {
        self._bitfield_0 = (self._bitfield_0 & !(0x1F << 3)) | (((v as u8) & 0x1F) << 3);
    }
}
```

## Estimated LOC: ~100
