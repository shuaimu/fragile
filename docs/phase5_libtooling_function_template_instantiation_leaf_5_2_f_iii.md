# Phase 5.2.f.iii: Function-decl template-instantiation surfaces

Date: 2026-02-26

## Scope

Add deterministic LibTooling handling for free-function template specialization/
instantiation surfaces represented as `TagFunctionDecl` nodes.

## Implementation

- `AstExporter.cpp` (`VisitFunctionDecl`) now exports additional metadata for function-decl
  template instantiations:
  - `isTemplateInstantiation` flag
  - template argument text payload array (from `getTemplateSpecializationArgs`)
- `libtooling.rs` conversion now detects this metadata and maps eligible
  `TagFunctionDecl` nodes to `ClangNodeKind::FunctionTemplateInstantiation`.
- Added bounded template-argument text -> `CppType` conversion for common primitive spellings,
  with fallback to `CppType::Named(...)` for non-primitive or non-type argument text.

## Validation

Added focused regressions in `libtooling.rs`:
- synthetic conversion test for `TagFunctionDecl` template-instantiation mapping
- parse-roundtrip test fixture proving exporter metadata (`extras[4]/extras[5]`) is present and
  conversion yields `FunctionTemplateInstantiation` with non-empty template args

Execution evidence:
- `cargo test -p fragile-clang libtooling::tests -- --nocapture` passes with new coverage.
