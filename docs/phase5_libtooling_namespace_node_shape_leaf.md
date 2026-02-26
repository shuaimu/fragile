# Phase 5.2.d: LibTooling namespace node-shape parity

Date: 2026-02-26

## Scope

Leaf task: close one high-impact `Unknown(...)` mapping in the LibTooling conversion path by restoring namespace container semantics.

Implemented surfaces:
- `TagNamespaceDecl` now converts to `ClangNodeKind::NamespaceDecl` in `libtooling.rs`.
- Namespace declarations now export child declaration IDs from `AstExporter.cpp` (`VisitNamespaceDecl`), so LibTooling node graphs preserve declaration nesting.
- Backend parity replay fixture now includes a namespaced function and asserts `pub mod math` + `ns_add` markers for `libclang`, `hybrid`, and `libtooling`.

## Why this leaf

`AstCodeGen` already has namespace-aware lowering logic (module generation, namespace tracking, inline namespace handling). When LibTooling emitted namespaces as `Unknown("NamespaceRelated")` and without child linkage, the codegen path lost namespace container structure and risked flat/global emission drift.

## Expected impact

- Improves LibTooling node-shape parity for namespace-contained declarations.
- Reduces pressure on downstream fallback heuristics that rely on globalized names.
- Establishes deterministic parity coverage for namespace structure in the parser-backend replay harness.
