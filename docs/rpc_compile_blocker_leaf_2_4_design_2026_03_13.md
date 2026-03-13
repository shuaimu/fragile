# RPC Compile Blocker Leaf 2.4 Design (2026-03-13)

## Scope

Leaf `2.4` closes the next-ranked post-`2.3` type-lowering/default-synthesis blocker family for RPC bring-up.

## Analysis

Using archived deterministic blocker captures:

- `logs/mako_bench_cmp_20260308/fragilec_rpcbench_build.log`
- `logs/mako_bench_cmp_20260308/b6_fragilec_rpcbench_build.log`

Error ranking after `E0425` unresolved-name noise:

- `E0425`: dominant (168 / 170)
- `E0277`: one hit in each log, both:
  - `the trait bound std::thread::JoinHandle<()>: Default is not satisfied`

This points to a generic default-synthesis mismatch for non-`Default` wrapper fields (`JoinHandle`, alias wrappers, and related mpsc/thread primitives).

## Decision

Adjust generic `Default`-impl normalization in `AstCodeGen` so we preserve fieldwise non-`Default` wrapper initialization behavior while still rewriting whole-struct zeroed defaults.

Specifically:

1. Keep rewriting whole-struct `unsafe { std::mem::zeroed() }` defaults.
2. Do not rewrite per-field `unsafe { std::mem::zeroed() }` expressions inside `Self { ... }` fieldwise defaults to `MaybeUninit::<Self>::zeroed().assume_init()`.
3. Detect actual struct-literal lines (`Self {`) instead of matching `fn default() -> Self {` signatures.

## Wrong-Approach Check

Aligned with project constraints and `docs/dev/wrong.md`:

- no RPC target-name conditionals
- no fake semantic method bodies
- no fallback "pretend success" stubs
- fix is generic codegen normalization only

## Implementation

File changed:

- `crates/fragile-clang/src/ast_codegen.rs`

Changes:

1. In both existing-default rewrite passes, replaced broad `block_text.contains("Self {")` checks with line-level literal detection:
   - `block_slice.iter().any(|line| line.trim_start().starts_with("Self {"))`
2. Applied that guard consistently to:
   - `can_rewrite` decision path
   - whole-block `zeroed() -> MaybeUninit::<Self>` replacement path
3. Added focused regressions:
   - `test_normalize_add_missing_struct_default_clone_impls_zeroes_join_handle_fields`
   - `test_normalize_add_missing_struct_default_clone_impls_zeroes_join_handle_alias_fields`
4. Kept prior rewrite regression passing:
   - `test_normalize_add_missing_struct_default_clone_impls_rewrites_existing_zeroed_defaults_fieldwise`

## Validation

Focused validations:

- `cargo test -p fragile-clang --lib normalize_add_missing_struct_default_clone_impls_zeroes_ -- --nocapture`
- `cargo test -p fragile-clang --lib test_normalize_add_missing_struct_default_clone_impls_rewrites_existing_zeroed_defaults_fieldwise -- --nocapture`

Whole-suite checks:

- `cargo test --workspace` (baseline red in `fragile-clang` `ast_codegen`)
- `FRAGILE_ENABLE_DEGRADED_FALLBACK=1 cargo test --workspace` (still baseline red: `739 passed / 24 failed` in `fragile-clang` `ast_codegen`)
- Python test runner unavailable in this environment (`python3 -m pytest`: module not installed)

Conclusion: leaf-`2.4` fix and regressions pass; full-workspace pre-existing red cluster remains outside this leaf scope.
