# Phase 5.2.f.ii: Class-template-specialization metadata parity locks

Date: 2026-02-26

## Scope

Lock metadata parity for class-template-specialization nodes in LibTooling paths,
with emphasis on exporter surfaces already present in AST extras:
- qualified specialization naming
- template-argument payloads
- implicit-instantiation / explicit-specialization markers

## Implementation

- Extended `SpecializationFieldInfo` in `libtooling.rs` with:
  - `is_implicit_instantiation: bool`
  - `is_explicit_specialization: bool`
- `extract_specialization_field_types()` now propagates exporter metadata:
  - reads markers from specialization node extras indices 3 and 4
  - preserves template-argument text extraction from extras index 2

## Validation

Added focused regressions in `libtooling.rs` tests:
- synthetic metadata extraction test proving template args + instantiation markers are preserved
- parse-roundtrip test fixture with both explicit (`Box<int>`) and implicit (`Box<long>`) specialization paths
  asserting specialization-node extras and extracted metadata both carry expected marker classes

Execution evidence:
- `cargo test -p fragile-clang libtooling::tests -- --nocapture` passes with the new tests.
