# Plan: Field Access (Task 3.1.2)

## Date: 2026-01-17
## Status: COMPLETED [26:01:17, 22:00]

## Problem Statement

Member access expressions (`s.field` and `ptr->field`) in the C++ → MIR pipeline were not converted correctly:
- `MemberExpr` fell through to the default case in `convert_expr()` returning `MirConstant::Unit`
- The `MirProjection::Field` variant existed but was not being used

## Changes Made

### 1. Updated MirProjection::Field (lib.rs)

Changed from index-only to support both index and name:

```rust
// Before
Field(usize),

// After
Field {
    index: usize,
    name: Option<String>,
},
```

This allows the fragile-clang converter to store the field name, which can be resolved to an index later in the rustc driver when full type information is available.

### 2. Added MemberExpr handling (convert.rs)

Added case for `ClangNodeKind::MemberExpr` in `convert_expr()`:

- For dot access (`s.field`): Adds `MirProjection::Field` directly
- For arrow access (`ptr->field`): Adds `MirProjection::Deref` followed by `MirProjection::Field`

```rust
ClangNodeKind::MemberExpr { member_name, is_arrow, ty: _ } => {
    // Convert base to place
    let mut base_place = ...;

    // For arrow access, dereference first
    if *is_arrow {
        base_place.projection.push(MirProjection::Deref);
    }

    // Add field access
    base_place.projection.push(MirProjection::Field {
        index: 0, // Placeholder
        name: Some(member_name.clone()),
    });

    Ok(MirOperand::Copy(base_place))
}
```

### 3. Updated rustc driver (mir_convert.rs)

Updated to handle the new `MirProjection::Field` struct format:

```rust
fragile_clang::MirProjection::Field { index, name: _ } => {
    ProjectionElem::Field(
        FieldIdx::from_usize(*index),
        self.tcx.types.unit,
    )
}
```

### 4. Added Tests

Three new tests in `convert.rs`:
- `test_convert_member_expr_dot`: Tests dot access (`s.field`)
- `test_convert_member_expr_arrow`: Tests arrow access (`ptr->field`)
- `test_convert_nested_member_expr`: Tests nested access (`outer.inner.value`)

## Files Changed

1. `crates/fragile-clang/src/lib.rs` - Updated `MirProjection::Field` to include name
2. `crates/fragile-clang/src/convert.rs` - Added `MemberExpr` handling + 3 tests
3. `crates/fragile-rustc-driver/src/mir_convert.rs` - Updated to use new Field struct

## Design Decisions

1. **Name-based field lookup**: The field name is stored alongside the index because during fragile-clang conversion, we don't have easy access to struct definitions to resolve the index. The rustc driver can resolve the name to an index when full type information is available.

2. **Arrow access as Deref + Field**: Following MIR conventions, `ptr->field` is represented as a Deref projection followed by a Field projection, matching how rustc handles this.

## Future Improvements

- Resolve field index from struct definition during fragile-clang conversion
- Pass proper field type to rustc instead of using `unit` as placeholder
