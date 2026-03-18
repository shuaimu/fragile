# M5.A1.a Mapped Family Acceptance Regression (2026-03-18)

## Context

Top unfinished P0 task after completing `M5.3` was `M5.A1`.

`M5.A1` is broad, so it was decomposed into leaf tasks. This iteration executes
`M5.A1.a`.

## Scope Sizing

`M5.A1` overall is larger than one small patch, but `M5.A1.a` is small
(<1000 LOC):

- `TODO.md` decomposition + leaf status update: small.
- One focused active parser-output handoff acceptance test in
  `crates/fragile-clang/src/lib.rs`.
- Documentation updates in `docs/`.

## Implemented Leaf

- `M5.A1.a Add active parser-output handoff acceptance regression for mapped associative/sequence/smart-pointer families (`map`, `unordered_map`, `vector`, `shared_ptr`, `unique_ptr`) proving canonical pre-generated target resolution with no unresolved placeholder structs.`

## Design Decision

Use a single deterministic integration-style parser-output handoff regression
that includes one representative type per mapped family and checks:

1. mapping manifest coverage for all five families,
2. canonical target routing in emitted output,
3. no unresolved placeholder structs for family-lowered names.

This keeps coverage generic and directly tied to active handoff behavior,
without introducing target-specific logic or fake stubs.

## Implementation Summary

Added test in `crates/fragile-clang/src/lib.rs`:

- `parser_output_codegen_active_handoff_mapped_supported_associative_sequence_smart_pointer_families_resolve_to_pre_generated_targets`

The test:

- builds a temporary C++ source with lowered names:
  - `map_int__int`, `unordered_map_int__int`, `vector_int`,
    `shared_ptr_int`, `unique_ptr_int`
- adds parser-output nodes for:
  - `stl_map_placeholder`, `stl_unordered_map_placeholder`,
    `stl_vector_placeholder`, `stl_shared_ptr_placeholder`,
    `stl_unique_ptr_placeholder`
- transpiles via `transpile_parser_output_to_rust`
- asserts deterministic manifest lines and canonical resolution markers.

## Wrong-Approach Compliance

Checked against:

- `docs/fragile-dev-book.md` section `1.3`
- `docs/dev/wrong.md`

No forbidden approaches were introduced:

- no target-specific hacks,
- no semantic stubs/fake bodies,
- no force-native bypass behavior.

## Validation

Focused:

- `cargo test -p fragile-clang parser_output_codegen_active_handoff_mapped_supported_associative_sequence_smart_pointer_families_resolve_to_pre_generated_targets -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_ -- --nocapture`

Full regression (performed after leaf implementation):

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
