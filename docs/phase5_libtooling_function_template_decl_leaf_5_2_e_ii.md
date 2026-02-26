# Phase 5.2.e.ii: Function-template declaration conversion

Date: 2026-02-26

## Scope

Implemented the first pass of LibTooling `FunctionTemplateDecl` conversion for exporter-supported surfaces:
- Export `TagFunctionTemplateDecl` with template-parameter children, function-parameter children, and optional body child.
- Convert `TagFunctionTemplateDecl` to concrete `ClangNodeKind::FunctionTemplateDecl` in `libtooling.rs`.

## Notes

- Conversion now preserves:
  - template parameter names
  - parameter-pack indices (when present)
  - function signature shape (return/params via `FunctionProtoType` fallback when available)
  - definition/body presence
- `requires_clause` remains `None` in this leaf (no exporter surface yet).

## Validation

- Added synthetic conversion regression for `FunctionTemplateDecl`.
- Added parse-roundtrip regression verifying exported function-template child linkage and conversion.
- Extended parser-backend parity fixture input to include a simple function-template declaration + use site to exercise this path in replay runs.
