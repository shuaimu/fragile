# Phase 5.3.a.i: LibTooling resolve_type parity for high-impact tags

Date: 2026-02-26

## Scope

Close immediate `libtooling::resolve_type` gaps where exported AST type tags were
still downgraded to `Named("Unknown...")`/fallbacks instead of concrete `CppType`
shapes that already exist in the libclang path.

## Why this leaf

`5.3` is a broad backend type-parity effort. This leaf targets the smallest
high-impact set that can be completed safely in one step:

- builtin char-family tags used in real headers (`TagWChar`, `TagChar16`, `TagChar32`)
- array shape tags (`TagConstantArrayType`, `TagIncompleteArrayType`)
- wrapper tags that should transparently forward (`TagDecayedType`, `TagAttributedType`, `TagParenType`)
- function signature shape tag (`TagFunctionProtoType`)

These were selected because they are common in strict transpilation paths and can
be validated with deterministic synthetic unit tests.

## Implementation

Updated `crates/fragile-clang/src/libtooling.rs` `resolve_type` mapping:

- Added direct mappings:
  - `TagWChar` -> `CppType::Int { signed: true }`
  - `TagChar16` -> `CppType::Short { signed: false }`
  - `TagChar32` -> `CppType::Int { signed: false }`
- Added array mappings:
  - `TagConstantArrayType` -> `CppType::Array { size: Some(n) }`
  - `TagIncompleteArrayType`/`TagVariableArrayType`/`TagDependentSizedArrayType` -> `CppType::Array { size: None }`
- Added wrapper forwarding:
  - `TagDecayedType`/`TagAttributedType`/`TagParenType` now resolve their inner type IDs.
- Added function-prototype mapping:
  - `TagFunctionProtoType` now resolves return/parameter type IDs and variadic flag into `CppType::Function`.

## Tests

Added focused regressions in `libtooling.rs` tests:

- `test_resolve_type_maps_wrapper_array_and_extended_builtin_tags`
- `test_resolve_type_maps_function_proto_with_const_shapes`

Both pass and lock:

- builtin char-family parity
- array/wrapper resolution behavior
- const-qualified pointer/reference behavior in function signatures
- variadic function shape preservation

Validation command:

- `cargo test -p fragile-clang libtooling::tests -- --nocapture`
