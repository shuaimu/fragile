# Phase 5.3.c.ii: Cross-backend CppType snapshot coverage for qualifier preservation and array decay boundaries

Date: 2026-02-26

## Scope

Implement `5.3.c.ii` by adding focused parse-roundtrip snapshot assertions on direct parser
outputs for:

- pointer/reference qualifier preservation (`const` pointer, mutable pointer, `const` reference, mutable reference)
- array decay boundary shapes (typedef/incomplete array surfaces, function-parameter array forms, and reference-to-array non-decay boundary)

This leaf stayed small (<500 LOC), implemented in
`crates/fragile-clang/tests/parser_backend_parity_tests.rs`.

## Design

Added a dedicated fixture and snapshot gate:

- New test:
  `test_parser_backend_cpp_type_snapshot_pointer_ref_qualifiers_and_array_decay_boundaries`
- Fixture source includes:
  - `read_const_ptr(const int*)`
  - `read_mut_ptr(int*)`
  - `read_const_ref(const int&)`
  - `bump_mut_ref(int&)`
  - `typedef int SizedArrayAlias[4];`
  - `typedef int UnsizedArrayAlias[];`
  - `decay_sized_array_param(SizedArrayAlias value)`
  - `decay_unsized_array_param(int value[])`
  - `preserve_array_ref_boundary(int (&value)[4])`
- Snapshot capture is taken from direct parser outputs per backend:
  - `libclang`: `ClangParser::parse_file`
  - `hybrid`: direct parse shape from `ClangParser::parse_file`
  - `libtooling`: `LibToolingParser::parse_file` + `convert_to_clang_node`

The test writes:

- `/tmp/fragile_parser_backend_cpp_type_qualifier_decay_snapshot_fixture_*/parser_backend_cpp_type_qualifier_decay_snapshot_manifest.txt`

and asserts the expected snapshot set per backend.

## Snapshot locked by test

- `libclang` / `hybrid`:
  - pointer/ref qualifier families remain explicit `CppType::Pointer` / `CppType::Reference` with expected const flags
  - typedef surfaces preserve array shapes (`Array { size: Some(4) }`, `Array { size: None }`)
  - function parameter array forms remain array-typed in this direct parse snapshot
  - reference-to-array boundary remains non-decayed `Reference<Array<...>>`
- `libtooling`:
  - pointer/ref qualifier families match `libclang`/`hybrid`
  - typedef and reference-boundary array shapes match `libclang`/`hybrid`
  - function parameter array forms snapshot as decayed pointer shapes (`Pointer<Int>`)

This gives a deterministic baseline for follow-up type-lowering unification work.

## Validation

Executed and passing:

- `cargo test -p fragile-clang --test parser_backend_parity_tests -- --nocapture`
