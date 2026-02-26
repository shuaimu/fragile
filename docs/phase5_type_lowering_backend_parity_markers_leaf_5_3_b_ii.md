# Phase 5.3.b.ii: Backend parity markers for libc alias lowering and template fallback surfaces

Date: 2026-02-26

## Scope

Complete `5.3.b.ii` by extending the parser-backend parity fixture with deterministic
type-lowering markers for:

- libc alias lowering (`__FILE` family)
- template placeholder/dependent-type fallback surfaces that remain intentionally emitted

This leaf remains small (<500 LOC) and was implemented directly in the parity fixture test.

## Design notes

Updated `crates/fragile-clang/tests/parser_backend_parity_tests.rs` with:

- Fixture-owned libc alias surface:
  - `struct __FILE;`
  - `typedef __FILE FragileFileAlias;`
- New marker assertions:
  - `pub type FragileFileAlias = std::ffi::c_void`
  - `pub type value_type = std::ffi::c_void`
  - `pub struct _dependent_type;`

The fallback markers (`value_type`, `_dependent_type`) are intentional current-state
surfaces in generated output and are now parity-locked across `libclang`, `hybrid`,
and `libtooling`.

## Test updates

Extended `BackendReplayResult`, manifest serialization, and backend parity assertions with:

- `typedef_fragile_file_alias`
- `template_placeholder_value_type_alias`
- `dependent_type_placeholder_struct`

All markers are required in the `libclang` reference output and then asserted equal for
`hybrid` and `libtooling`.

## Validation

Executed and passing:

- `cargo test -p fragile-clang --test parser_backend_parity_tests test_parser_backend_parity_local_fixture_replay -- --nocapture`
