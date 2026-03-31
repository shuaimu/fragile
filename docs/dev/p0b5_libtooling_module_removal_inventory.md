# P0.b.5 LibTooling Module Removal Inventory

Date: 2026-03-23
Owner leaf: `P0.b.5.a`

## Scope and sizing

`P0.b.5` is too large for a single safe patch:
- `crates/fragile-clang/src/libtooling.rs`: 4425 LOC
- `crates/fragile-clang/src/lib.rs`: 3775 LOC and directly depends on `libtooling` symbols
- `crates/fragile-cli/src/main.rs`: external dependency on `fragile_clang` LibTooling exports

A single-shot delete would exceed the <1000 LOC target and mix API-surface, parser-path, and CLI coupling changes.

## Line-anchored dependency inventory

### A. Module exposure and internal parser-path dependencies (`fragile-clang`)

File: `crates/fragile-clang/src/lib.rs`

- `16`: `mod libtooling;`
- `25-28`: `pub use libtooling::{...}` exports that expose LibTooling symbols cross-crate
- `386`: `build_parser_from_options(...) -> LibToolingParser`
- `451`: `LibToolingParser::new().with_extra_args(...)`
- `1106`: `parse_libtooling_context(...)` (primary parser path still using LibTooling parser)
- `1470`: `convert_to_clang_node(...)` call from translation-unit conversion

### B. External consumers of `fragile-clang` LibTooling exports

File: `crates/fragile-cli/src/main.rs`

- `97`, `158`: `fragile_clang::MethodInfo`
- `103`, `162`: `fragile_clang::SpecializationFieldInfo`
- `108`: `fragile_clang::LibToolingParser::new()`
- `114`: `fragile_clang::extract_method_bodies_with_params(...)`
- `116`: `fragile_clang::extract_specialization_field_types(...)`

File: `examples/debug_libtooling.rs`

- `1`: imports `LibToolingParser`, `extract_method_bodies_with_params`
- `7`, `10`: direct invocation path through exported LibTooling symbols

## Coupling boundary with `P0.b.6`

`P0.b.6` owns removal of CLI `--use-libtooling` pre-parse flow and related artifacts.

`P0.b.5` must not delete the module before `P0.b.6` callsites are removed or rewritten.
Otherwise, `fragile-cli` and example artifacts will not compile once `pub use` is dropped.

## Ordered patch slices (<1000 LOC each)

1. `P0.b.5.b` (API exposure cut)
- Remove `pub use libtooling::{...}` from `crates/fragile-clang/src/lib.rs`.
- In the same slice, remove or rewrite all external consumers that depend on these exports (primarily `fragile-cli/src/main.rs`, `examples/debug_libtooling.rs`) so workspace remains compile-safe.

2. `P0.b.5.c` (internal dependency migration)
- Remove production-path imports from `src/libtooling.rs` by moving/rewiring remaining internals needed by `lib.rs` (`LibToolingParser`, AST conversion/extraction helpers) into non-exported internal placement.
- Keep changes bounded to parser-path internals only.

3. `P0.b.5.d` (module deletion)
- Delete `crates/fragile-clang/src/libtooling.rs`.
- Remove `mod libtooling;` and any stale imports/types.

4. `P0.b.5.e` (test/docs closure)
- Update `p0_libtooling_removal_audit_tests.rs` and `p0c_anti_regression_tests.rs` ceilings/contracts.
- Update operational docs that still refer to public LibTooling exports.

## Validation gates per slice

- `cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests -- --nocapture`
- `cargo test -p fragile-clang --test p0c_anti_regression_tests -- --nocapture`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Wrong-approach guard

Checked against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)` and `docs/dev/wrong.md`.

- No target-specific conditionals.
- No force-native bypasses.
- No fake semantic stubs to hide missing LibTooling removal work.
