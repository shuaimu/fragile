# Phase 5.3.c.i: Cross-backend CppType snapshot coverage for decltype + template-dependent families

Date: 2026-02-26

## Scope

Implement `5.3.c.i` by adding focused parse-roundtrip snapshot assertions on direct parser
outputs for:

- `decltype` families (`using` alias + direct `decltype(...)` signature shape)
- template placeholder/dependent families (template parameter and dependent type spellings)

This leaf stayed small (<500 LOC), implemented entirely in
`crates/fragile-clang/tests/parser_backend_parity_tests.rs`.

## Design

Added a dedicated fixture and snapshot gate:

- New test: `test_parser_backend_cpp_type_snapshot_decltype_and_template_families`
- Fixture source includes:
  - `DecltypeAlias` + `decltype_alias_identity`
  - `decltype_direct_identity`
  - `dependent_identity<T>`
  - `dependent_holder_identity<T>` with `typename Holder<T>::value_type`
- Snapshot capture is taken from direct parser outputs per backend:
  - `libclang`: `ClangParser::parse_file`
  - `hybrid`: direct parse shape from `ClangParser::parse_file` (same primary AST lane)
  - `libtooling`: `LibToolingParser::parse_file` + `convert_to_clang_node` on top nodes

The test writes a deterministic manifest:

- `/tmp/fragile_parser_backend_cpp_type_snapshot_fixture_*/parser_backend_cpp_type_snapshot_manifest.txt`

and asserts the full expected snapshot set per backend.

## Snapshot locked by test

- `libclang` / `hybrid`:
  - `decltype` alias underlying and direct `decltype` function signature retain
    `Named("decltype(1 + 2)")` in direct parse shape where expected
  - template-dependent surfaces remain explicit (`TemplateParam` / `DependentType`)
- `libtooling`:
  - `decltype` families resolve to concrete `Int`
  - dependent template parameter families currently degrade to concrete/fallback surfaces
    (`Int` return with `Named("auto")` params in this fixture)

This provides a stable baseline for follow-up unification work in `5.3.c.ii` / `5.3.d`.

## Validation

Executed and passing:

- `cargo test -p fragile-clang --test parser_backend_parity_tests -- --nocapture`
