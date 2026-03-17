# M3.1.b Type-Alias STL Symbol Table (2026-03-17)

## Objective

Close TODO leaf `M3.1.b` by adding deterministic typedef/type-alias symbol
table extraction with canonical STL target normalization.

## Scope and Sizing

This leaf is bounded below 1000 LOC and limited to:

- `crates/fragile-parser-clang/src/lib.rs`
- `TODO.md`
- docs updates

No additional decomposition was required.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails followed:

- no semantic stubs/fake method bodies
- no target-specific hacks
- no native-force bypasses
- no silent fallback behavior

## Design

Added `fragile-parser-clang` helper:

- `extract_stl_type_alias_symbol_table(root: &ClangNode) -> BTreeMap<String, String>`

Behavior:

1. Traverses AST deterministically and collects type alias declarations:
   - `TypeAliasDecl`
   - `TypeAliasTemplateDecl`
   - `TypedefDecl`
2. Records aliases as fully-qualified names (`ns::Alias`) via namespace stack.
3. Resolves alias targets to STL families using:
   - direct `std::` family detection
   - typedef/type-alias chain resolution via previously resolved aliases
4. Emits canonical normalized targets:
   - `std::vector`, `std::map`, `std::unordered_map`, `std::string`,
     `std::optional`, `std::variant`, `std::tuple`,
     `std::shared_ptr`, `std::unique_ptr`
5. Keeps unresolved/ambiguous aliases out of the table (deterministic, no
   guessed fallback).

Out of scope for this leaf:

- `using` declaration/directive chain resolution (`M3.1.c`)

## Validation

Focused:

- `cargo test -p fragile-parser-clang`

Full regression gates:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

Programmatic usage:

```rust
use fragile_parser_clang::extract_stl_type_alias_symbol_table;

let aliases = extract_stl_type_alias_symbol_table(&clang_ast.translation_unit);
```

Example normalized entry:

- input alias spelling: `using VecAlias = std::__1::vector<int>;`
- output table entry: `VecAlias -> std::vector`
