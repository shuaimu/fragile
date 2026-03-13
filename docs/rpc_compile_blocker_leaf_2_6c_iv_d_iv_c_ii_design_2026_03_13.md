# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.ii Design (2026-03-13)

## Scope

Leaf `2.6.c.iv.d.iv.c.ii` targets a generic codegen hot path in the
pre-`codegen_after_top_level_generation` checkpoint window.

Estimated implementation size was bounded (<500 LOC, actual delta is far
smaller), so no further TODO decomposition was required for this leaf.

## Context

From `2.6.c.iv.d.iv.c.i`, strict timeout replay checkpoints remain at
`codegen_after_template_instantiation_generation` by 300s.

In this area, class-template definitions are handled in two passes:

- precollection (`collect_template_definitions_with_namespace`)
- top-level traversal (`generate_top_level` `ClassTemplateDecl` arm)

The top-level path could still do unconditional definition replacement and
vector cloning even when the pre-collected definition was already richer.

## Decision

Unify class-template replacement policy and storage in a shared helper path used
by both passes.

Policy preserved:

- insert when no existing entry exists
- replace only when the candidate has field declarations and existing entry does
  not

## Implementation

File changed:

- `crates/fragile-clang/src/ast_codegen.rs`

Additions:

- `class_template_children_have_fields(children: &[ClangNode]) -> bool`
- `should_replace_class_template_definition(existing, candidate_has_fields)`
- `store_class_template_definition_if_better(key, template_params, children)`

Call-site changes:

- precollection now calls
  `store_class_template_definition_if_better(...)` for short + fully-qualified
  class-template keys.
- top-level `ClassTemplateDecl` fallback storage now calls the same helper
  instead of unconditional `insert(... node.children.clone())`.

Regression coverage:

- `test_generate_top_level_class_template_decl_does_not_replace_precollected_definition`

## Validation

Commands:

- `cargo test -p fragile-clang test_generate_top_level_class_template_decl_does_not_replace_precollected_definition -- --nocapture`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Results:

- focused regression passes.
- workspace cargo remains known baseline red cluster
  (`fragile-clang` lib `727` passed / `46` failed).
- python suite passes (`29`, skipped `1`).

## Wrong-Approach Guardrails

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-specific special casing
- no fallback semantic stub synthesis
- no force-native source/path usage

## Notes

This leaf completes the optimization/coverage step only. Replay non-increase
verification remains in the next leaf (`2.6.c.iv.d.iv.c.iii`).
