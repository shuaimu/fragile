# Plan: Task 25.3 - Generate VTable Structs

## Overview

Generate `{ClassName}_vtable` structs with function pointer fields for each virtual method.

## Target Output

For a class like:
```cpp
class exception {
    virtual const char* what() const noexcept;
    virtual ~exception();
};
```

Generate:
```rust
#[repr(C)]
pub struct exception_vtable {
    pub what: unsafe fn(*const exception) -> *const i8,
    pub __destructor: unsafe fn(*mut exception),
}
```

## Implementation Steps

### 25.3.1 Generate vtable struct

Add `generate_vtable_struct()` function:
```rust
fn generate_vtable_struct(&mut self, class_name: &str, vtable_info: &ClassVTableInfo) {
    let sanitized = sanitize_identifier(class_name);
    self.writeln(&format!("#[repr(C)]"));
    self.writeln(&format!("pub struct {}_vtable {{", sanitized));
    self.indent += 1;

    for entry in &vtable_info.entries {
        // Generate function pointer field
    }

    self.indent -= 1;
    self.writeln("}");
}
```

### 25.3.2 Function pointer signature

Each virtual method `fn foo(&self, x: T) -> R` becomes:
```rust
pub foo: unsafe fn(*const Self, x: T) -> R
```

For non-const methods (mutating self):
```rust
pub foo: unsafe fn(*mut Self, x: T) -> R
```

### 25.3.3 Add destructor entry

Every vtable gets a `__destructor` entry:
```rust
pub __destructor: unsafe fn(*mut Self),
```

### 25.3.4 Covariant return types

If derived class returns `*Derived` where base returns `*Base`, use the base type in vtable.
This is handled by using the declaring_class in VTableEntry.

## Call Site

Generate vtable structs before class structs, at top of output.

## Estimated LOC

- generate_vtable_struct function: ~50 LOC
- Integration into generate(): ~10 LOC
- Total: ~60 LOC
