# Leaf 2.3 design note (2026-03-13)

## Goal
Close the highest-frequency unresolved-name/type blocker family from RPC compile blocker captures with a generic codegen fix and focused regression tests.

## Evidence-driven target selection
Using archived deterministic logs:

- `logs/mako_bench_cmp_20260308/fragilec_rpcbench_build.log`
- `logs/mako_bench_cmp_20260308/b6_fragilec_rpcbench_build.log`

`error[E0425]` unresolved-type counts show the top family is `Fiber` (79 hits in each log), followed by `Event` and `Reactor`.

## Size / risk estimate
- Expected code change size: well under 500 LOC.
- Actual leaf implementation scope: one normalization function + focused unit tests (<200 LOC net change).

## Chosen generic fix
Enhance `normalize_unresolved_namespaced_type_aliases` in `crates/fragile-clang/src/ast_codegen.rs` so that when a bare unresolved type leaf has one root-accessible namespaced target, item-type positions are rewritten to a fully qualified target (`crate::<ns>::Type` when needed), including:

- alias RHS
- struct/union fields
- function signature parameter/return types
- static type positions

The existing alias-emission behavior is kept for non-conflicted leaves. The rewrite now still applies when alias emission is intentionally skipped (for example module-name conflict cases), preventing unresolved bare type tokens from surviving to rustc.

## Wrong-approach checks
Complies with `docs/fragile-dev-book.md` and `docs/dev/wrong.md` constraints:

- no `rpcbench`/`test_rpc` target-name conditionals
- no force-native bypasses
- no fake semantic stubs/fallback bodies
- generic parser/codegen-path fix with deterministic unit regressions

## Regressions added
- `test_normalize_unresolved_namespaced_type_aliases_rewrites_bare_rhs_type_uses`
- expanded `test_normalize_unresolved_namespaced_type_aliases_skips_reserved_thread_alias_when_top_level_module_conflicts`
