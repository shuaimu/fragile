# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.a Design (2026-03-13)

## Scope

Leaf `2.6.c.iv.d.iv.c.iv.a` implements a bounded generic optimization in the
pre-`codegen_after_top_level_generation` checkpoint window.

Estimated implementation size was small (<500 LOC), so no further decomposition
was required for this sub-leaf.

## Context

`lookup_template_definition` previously returned owned tuples and cloned full
template-definition payloads (`Vec<String>`, `Vec<ClangNode>`) on each
successful lookup.

In this stage, `collect_template_type` performs high-frequency existence checks
with `.is_some()`, so clone-heavy lookup behavior was avoidable overhead.

## Decision

Switch lookup to borrowed return semantics and retain explicit cloning only where
mutable codegen requires owned values.

Chosen behavior:

- `lookup_template_definition` returns
  `Option<&(Vec<String>, Vec<ClangNode>)>`.
- existence checks stop cloning definition payloads.
- instantiation emission (`generate_template_instantiations`) clones at the
  call site before invoking `generate_template_struct` to satisfy borrow rules
  and preserve semantics.

## Implementation

File changed:

- `crates/fragile-clang/src/ast_codegen.rs`

Main updates:

1. Borrowed template-definition lookup (direct + inline namespace alias paths).
2. Instantiation generation clones moved to emission boundary only.
3. Focused regression added:
   `test_lookup_template_definition_uses_inline_namespace_alias_entry_reference`.

## Validation

Commands:

- `cargo test -p fragile-clang test_lookup_template_definition_uses_inline_namespace_alias_entry_reference -- --nocapture`
- `cargo build --release -p fragile-cli --bin fragilec`
- strict replay captures with fresh profile/timing artifacts:
  - timeout 120s: `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_a_callshape_profile_120_v1.txt`
  - timeout 300s: `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_a_callshape_profile_300_v1.txt`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Observed evidence:

- 120s replay profile reaches `codegen_after_template_collection`.
- 300s replay profile remains bounded at
  `codegen_after_template_instantiation_generation` with lower bytes than prior
  baseline (`574973 -> 567691`, `-7282`).
- blocker class remains `build_timeout` on `src/rrr/base/misc.cpp`.
- full suites match known baseline (`fragile-clang` lib `728` passed / `46`
  failed; Python `29` passed / `1` skipped).

## Wrong-Approach Guardrails

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no RPC target-specific conditionals
- no force-native source fallback
- no semantic stub synthesis to fake green builds

## Result

Leaf `2.6.c.iv.d.iv.c.iv.a` is complete and preserves behavior while reducing
clone overhead in template-definition lookup-heavy paths.
