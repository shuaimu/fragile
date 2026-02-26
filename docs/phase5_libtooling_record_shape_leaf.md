# Phase 5.2 leaf: LibTooling record-shape conversion

Date: 2026-02-26

Implemented scope:
- Converted `TagCXXRecordDecl` from `Unknown` to `ClangNodeKind::RecordDecl` / `ClangNodeKind::UnionDecl` mapping (named records/unions).
- Converted `TagFieldDecl` from `DeclRefExpr` placeholder to `ClangNodeKind::FieldDecl`.
- Added conservative access decoding helper for field visibility.
- Extended parser-backend parity fixture to include a named record declaration (`struct Point`) and assert marker parity across `libclang`, `hybrid`, and `libtooling`.

Key finding:
- Non-template `CXXRecordDecl` currently arrives from `fragile-ast-exporter` without field children in this path.
- Because of this exporter shape, this leaf validates record declaration parity (`pub struct Point`) but does not yet enforce field-level parity for non-template record bodies.

Verification:
- `cargo test -p fragile-clang --test parser_backend_parity_tests`
- Full `cargo test`
