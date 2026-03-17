# M2.2 fragile-parser-clang Backend Skeleton (2026-03-17)

## Objective

Close TODO leaf `M2.2` by implementing a concrete `fragile-parser-clang`
backend that satisfies the `fragile-parser-core::ParserBackend` trait and emits
`ParserOutput v1`.

## Scope and Sizing

This change is below 1000 LOC:

- add new workspace crate:
  `crates/fragile-parser-clang`
- implement backend skeleton:
  `crates/fragile-parser-clang/src/lib.rs`
- add unit tests for backend behavior:
  `crates/fragile-parser-clang/src/lib.rs` (`#[cfg(test)]`)
- update workspace/TODO/docs

No additional TODO decomposition was required.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This leaf avoids forbidden shortcuts:

- no target-specific hacks
- no fake semantic method bodies to mask parser gaps
- no force-native transpilation bypasses

## Design

Added crate `fragile-parser-clang` with concrete backend:

- `FragileParserClangBackend` implementing
  `fragile_parser_core::ParserBackend`
- backend id constant:
  `FRAGILE_PARSER_CLANG_BACKEND_ID = "fragile-parser-clang"`

Parse flow:

1. Convert `ParseRequest` language to `fragile_clang::ParserLanguage`.
2. Build effective include-path and define sets from:
   - `include_directives` / `defines`
   - parsed `frontend_args` (`-I`, `-isystem`, `-iquote`, `-D`)
3. Parse source file through `fragile_clang::ClangParser`.
4. Convert `ClangAst` to `ParserOutputV1`:
   - deterministic pre-order flattened node ids (`n0`, `n1`, ...)
   - normalized parser node kind mapping
   - best-effort name/type extraction from clang node payload
   - translation-unit metadata copied from request contract

The implementation is intentionally a skeleton:

- it establishes trait-compatible parsing and deterministic IR emission
- it does not yet implement STL boundary placeholder emission
  (that work is covered by later milestones)

## Validation

Focused:

- `cargo test -p fragile-parser-clang`

Full Python:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Full workspace:

- `cargo test --workspace --all-targets`

## User Manual

Use this backend from parser-core integration points by registering
`FragileParserClangBackend` in `BackendRegistry` and dispatching via backend id:

- backend id: `fragile-parser-clang`
- parse request contract: `fragile_parser_core::ParseRequest`
- output contract: `fragile_parser_core::ParserOutputV1`
