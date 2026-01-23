# Plan: Flatten Anonymous Struct Fields (Task 13.1.3)

## Problem

When C++ has an anonymous struct inside another struct, its fields should be accessible directly:

```cpp
struct Outer {
    int regular_field;
    struct {
        int x;
        int y;
    };
    int another_field;
};

int main() {
    Outer o;
    o.x = 10;  // Direct access - not o.anonymous.x
}
```

## Current Behavior

1. The anonymous struct is detected with name like "(anonymous struct at ...)"
2. The transpiler generates `Outer` without the anonymous struct's fields
3. Member access is generated as `o.__base.x` which doesn't work

## Solution

### Changes to ast_codegen.rs

1. **In `generate_struct()` and `generate_struct_stub()`**:
   - When iterating children for FieldDecl, also check for nested RecordDecl
   - If the RecordDecl name starts with "(anonymous" or is empty:
     - Don't generate a separate struct
     - Flatten its FieldDecl children into the parent struct

2. **In member expression generation**:
   - When `declaring_class` contains "(anonymous", access the field directly
   - Don't use `__base.fieldname`, just use `fieldname`

### Implementation Steps

1. Modify `generate_struct_stub()` to flatten anonymous struct fields (~30 LOC)
2. Modify `generate_struct()` to flatten anonymous struct fields (~30 LOC)
3. Modify `generate_member_expr()` to handle anonymous struct member access (~20 LOC)

Total estimated: ~80 LOC

## Test Case

```cpp
struct Outer {
    int a;
    struct { int x; int y; };
    int b;
};

int main() {
    Outer o;
    o.a = 1;
    o.x = 10;
    o.y = 20;
    o.b = 2;
    return o.x + o.y;  // Should return 30
}
```

Expected Rust output:
```rust
pub struct Outer {
    pub a: i32,
    pub x: i32,
    pub y: i32,
    pub b: i32,
}

fn main() {
    let mut o = Outer::new_0();
    o.a = 1;
    o.x = 10;
    o.y = 20;
    o.b = 2;
    return o.x + o.y;
}
```
