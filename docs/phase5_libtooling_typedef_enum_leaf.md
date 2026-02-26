# Phase 5.2 leaf: LibTooling typedef/enum node conversion

Date: 2026-02-26

Scope implemented:
- Converted `TagTypedefDecl` to `ClangNodeKind::TypedefDecl`.
- Converted `TagTypeAliasDecl` to `ClangNodeKind::TypeAliasDecl`.
- Converted `TagEnumDecl` to `ClangNodeKind::EnumDecl` for named enums.
- Converted `TagEnumConstantDecl` to `ClangNodeKind::EnumConstantDecl` with value extraction.

Why this leaf:
- These node kinds were still mapped to `Unknown(...)`/placeholder shapes in `convert_node_with_depth`.
- They are high-impact declarations for type and constant fidelity in strict backend parity.
- The change is small and bounded (<500 LOC), making it a safe first leaf under TODO Phase 5.2.

Conservative edge handling:
- Unnamed enums and empty-name aliases remain on conservative `Unknown(...)` paths for now to avoid broad behavior changes until dedicated coverage is added.

Verification:
- Extended parser backend parity fixture to include typedef/type-alias/enum markers.
- Confirmed parity test passes for `libclang`, `hybrid`, and `libtooling` backends.
- Ran full `cargo test` successfully.
