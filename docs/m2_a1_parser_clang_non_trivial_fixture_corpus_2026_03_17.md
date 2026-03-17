# M2.A1 Non-Trivial Parser Fixture Corpus Closure (2026-03-17)

## Objective

Close TODO acceptance leaf `M2.A1` by proving the new
`fragile-parser-clang` backend can parse and emit `ParserOutput v1` across a
non-trivial checked-in C/C++ fixture corpus.

## Scope and Sizing

This change remains below 1000 LOC:

- add checked-in fixture corpus files:
  - `crates/fragile-parser-clang/tests/fixtures/m2_a1/include/*.hpp`
  - `crates/fragile-parser-clang/tests/fixtures/m2_a1/src/*.cpp`
  - `crates/fragile-parser-clang/tests/fixtures/m2_a1/src/*.c`
- add parser-core-backed integration test:
  - `crates/fragile-parser-clang/tests/non_trivial_corpus_tests.rs`
- update TODO/dev-book/docs

No TODO decomposition was required.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This leaf avoids forbidden shortcuts:

- no target-specific hacks
- no fake semantic fallback method bodies
- no force-native bypass strategy

## Design

Added a checked-in fixture corpus (`m2_a1`) with mixed C and C++ features:

- namespace/class/template/alias/function-pointer surfaces
- control-flow and aggregate initialization
- include/define resolution through both `include_directives` and
  `frontend_args`

Added integration coverage that dispatches through `BackendRegistry` using
backend id `fragile-parser-clang` and validates:

1. parse succeeds for each corpus translation unit
2. schema version is `1.0.0`
3. output is deterministic across repeated parses
4. node ids are deterministic (`n0..nN`)
5. required node kinds/names are present per corpus entry
6. aggregate node volume is non-trivial across corpus entries

## Validation

Focused:

- `cargo test -p fragile-parser-clang`

Full regression gates for this leaf:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

Fixture corpus location:

- `crates/fragile-parser-clang/tests/fixtures/m2_a1`

Primary gate test:

- `cargo test -p fragile-parser-clang --test non_trivial_corpus_tests`

This gate should stay green when parser-core contract changes are made to
`fragile-parser-clang`.
