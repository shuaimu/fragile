# Phase 5.2 leaf: non-template record field linkage in LibTooling export

Date: 2026-02-26

Scope:
- Updated `VisitCXXRecordDecl` in `fragile-ast-exporter` to attach concrete record field children for non-template, non-specialization records.
- Added anonymous-struct/union flattening so macro-expanded anonymous members are linked as fields.
- Kept existing specialization flow unchanged.

Why:
- LibTooling conversion for `TagCXXRecordDecl` can only preserve concrete field shape if record nodes are linked to `FieldDecl` children in exported AST data.
- Previously, non-template record declarations were emitted without member children, so downstream record nodes were effectively fieldless even when the C++ source had fields.

Verification:
- Extended parser-backend parity fixture continues to require typedef/alias/enum parity and now validates struct field markers (`Point.x`, `Point.y`) across `libclang`, `hybrid`, and `libtooling`.
- `cargo test -p fragile-clang --test parser_backend_parity_tests`
- Full `cargo test`
