# Phase 5.2.e.i: Class-template + template-type-parameter node conversion

Date: 2026-02-26

## Leaf scope

This leaf covers a bounded subset of `5.2.e`:
- Export and convert `ClassTemplateDecl` as a concrete `ClangNodeKind::ClassTemplateDecl`.
- Export and convert `TemplateTypeParmDecl` as `ClangNodeKind::TemplateTypeParmDecl`.
- Preserve class-template child linkage (template params + member decls) in the LibTooling graph.

Out of scope for this leaf:
- Full `FunctionTemplateDecl` conversion.
- Non-type/template-template parameter model decisions.

## Implementation notes

- `AstExporter.cpp` now links class-template children from the templated record declaration and emits template parameter declarations as first-class AST entries.
- `libtooling.rs` now maps `TagClassTemplateDecl` and `TagTemplateTypeParmDecl` away from `Unknown(...)`.
- Parser-backend parity fixture now includes `template<typename T> struct Box { T value; };` and asserts template markers across `libclang`, `hybrid`, and `libtooling`.

## Validation

- Added focused `libtooling.rs` tests for class-template/type-param conversion and parse-roundtrip child linkage.
- Extended backend parity replay assertions for template markers.
