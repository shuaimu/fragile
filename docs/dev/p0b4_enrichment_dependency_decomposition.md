# P0.b.4 Enrichment Dependency Decomposition (2026-03-23)

## Why P0.b.4.a was split

Original `P0.b.4.a` combined:
- `lib.rs` enrichment hook removal,
- `AstCodeGen` LibTooling state field removal,
- setter API removal.

That bundle is not compile-safe as a single first leaf because:
- `crates/fragile-cli/src/main.rs` still calls `AstCodeGen` LibTooling setter APIs,
- `AstCodeGen` LibTooling fields are referenced by multiple lookup/generation methods and callsites that are removed in later leaves.

## Compile-safe leaf order

1. `P0.b.4.a`: remove `apply_libtooling_enrichment` and its callsite in `transpile_cpp_to_rust_with_options`.
2. `P0.b.4.b`: remove template-generation callsites that depend on LibTooling state.
3. `P0.b.4.c`: remove LibTooling-only helper methods (`find_*`, rollback helper, `generate_libtooling_only_methods`).
4. `P0.b.4.d`: remove LibTooling `AstCodeGen` fields, initializers, and setter APIs.
5. `P0.b.4.e`: update anti-regression tests and ceilings.

Each leaf remains under the <1000 LOC target and keeps the build/test loop honest.

## Wrong-approach guard

Checked against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)` and `docs/dev/wrong.md`:
- no target-specific hacks,
- no force-native escape paths,
- no fake semantic stubs to mask missing functionality.
