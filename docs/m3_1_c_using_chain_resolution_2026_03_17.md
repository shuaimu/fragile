# M3.1.c Using-Chain STL Alias Resolution (2026-03-17)

## Objective

Close TODO leaf `M3.1.c` by extending canonical STL alias detection to resolve
`using` declaration/directive chains over the existing typedef/type-alias symbol
table.

## Scope and Sizing

This leaf is bounded to parser-side symbol detection logic in
`fragile-parser-clang` and remains well below 1000 LOC:

- collect `UsingDeclaration` and `UsingDirective` records with scope tracking
- resolve alias target spellings through visible using chains
- keep ambiguous using-based matches unresolved (no guessed fallback)
- add deterministic unit coverage for using declaration/directive scenarios

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific conditionals
- no semantic stubs/fake method bodies
- no force-native escape hatch usage
- no silent fallback guesses for ambiguous symbols

## Design

`extract_stl_type_alias_symbol_table(...)` now includes using-aware lookup:

1. Collect type alias records (`TypeAliasDecl`/`TypeAliasTemplateDecl`/`TypedefDecl`).
2. Collect using records:
   - `UsingDeclaration { qualified_name }`
   - `UsingDirective { namespace }`
3. For each unresolved alias target spelling token, generate candidate symbols
   from:
   - lexical-scope alias lookup (existing behavior)
   - visible `using X::Y` imported names
   - visible/transitive `using namespace X` namespaces
4. Resolve candidates against:
   - direct canonical `std::...` family detection
   - already-resolved alias symbol table entries
5. If candidate matches map to multiple canonical families, keep unresolved.

## Validation

Focused:

- `cargo test -p fragile-parser-clang`

Full regression:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

Call:

- `extract_stl_type_alias_symbol_table(root: &ClangNode) -> BTreeMap<String, String>`

Behavior now includes:

- direct `std::` alias normalization
- typedef/type-alias chain resolution
- `using` declaration/directive chain resolution
- ambiguity-safe behavior: unresolved when multiple canonical families match
