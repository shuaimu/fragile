# M3.2.c STL Boundary Fixture Regressions (2026-03-17)

## Objective

Close TODO leaf `M3.2.c` by adding deterministic fixture regressions that
assert:

- STL boundary placeholders are emitted for fixture boundary declarations.
- No deep STL internal subtree is lowered beneath emitted placeholder roots.

## Scope and Sizing

This leaf is well below 1000 LOC and scoped to parser-clang fixture regression
tests:

- no parser algorithm changes
- deterministic fixture-output assertions only
- recursive descendant checks for placeholder roots

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific conditionals
- no force-native paths
- no semantic stubs/fallback method bodies
- no guessed STL-family fallback

## Design

Updated fixture regression suite:

- `crates/fragile-parser-clang/tests/stl_symbol_detection_fixture_tests.rs`

Key additions:

- helper to collect deterministic named placeholder manifests
- helper to compute full descendant closure from parent/child links
- fixture assertion scoped to `consume_symbols` boundary roots:
  - compare repeated-run placeholder manifest to guarantee determinism
  - compare against explicit expected boundary placeholder set
  - assert each placeholder root has no descendants at any depth
- full-suite replay follow-up fix:
  - `crates/fragile-clang/src/ast_codegen.rs`
  - recover signed/unsigned identifier comparison casts during post-generation
    normalization (`size == literalSize` style) using known local/parameter
    integer lanes
  - added focused regressions for this normalization path

This closes the fixture-level gap left by `M3.2.a` and `M3.2.b`, which already
covered placeholder node-kind mapping and pruning mechanics.

## Validation

Focused:

- `cargo test -p fragile-parser-clang`

Full regression:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

For STL boundary fixture coverage, parser output now has deterministic
placeholder-root expectations and explicit guarantees that no deep STL internals
appear beneath those roots.
