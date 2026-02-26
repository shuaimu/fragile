# Phase 5.3.b.i: Backend parity markers for pointer/ref, decltype-scalar, and array surfaces

Date: 2026-02-26

## Scope

Implement the first `5.3.b` leaf by extending parser-backend parity fixture gates so
`libclang`, `hybrid`, and `libtooling` are checked for deterministic marker parity on:

- pointer/ref qualifier signatures
- decltype-backed scalar signatures
- array-type surface emission

## Design notes

The parity fixture (`crates/fragile-clang/tests/parser_backend_parity_tests.rs`) was
extended with additional C++ source surfaces and marker assertions.

Added fixture surfaces:

- `read_const_ptr(const int* p)` for const pointer param lowering
- `bump_ref(int& value)` for mutable reference param lowering
- `using DecltypeScalar = decltype(1);` plus
  `decltype_scalar_identity(DecltypeScalar v)` for decltype-backed scalar signature
- `typedef int IntArray4[4];` for array-type alias surface
- `array_decay_head(int* data)` for deterministic pointer call-shape marker

A direct decltype-in-signature shape (`decltype(...)` directly in function param/return)
was intentionally not used in this parity leaf because it currently destabilizes compile
in the libclang replay path; the alias-backed decltype shape keeps this gate deterministic
while still asserting decltype scalar lowering parity.

## Test updates

Extended marker set and parity manifest fields in `parser_backend_parity_tests` with:

- `const_ptr_fn_sig`
- `mut_ref_fn_sig`
- `decltype_scalar_fn_sig`
- `typedef_intarray4`
- `array_decay_fn_sig`

All markers are required in the libclang reference output, then enforced equal for
hybrid and libtooling.

## Validation

Executed and passing:

- `cargo test -p fragile-clang --test parser_backend_parity_tests -- --nocapture`
- `cargo test` (full workspace)
