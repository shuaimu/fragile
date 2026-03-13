# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.i Design (2026-03-13)

## Scope

Leaf `2.6.c.iv.d.iv.c.iv.c.i` implements a bounded generic optimization in the
pre-`codegen_after_top_level_generation` checkpoint window.

Estimated implementation size is small (<500 LOC), so no further decomposition
was required for this sub-leaf.

## Context

`generate_template_instantiations` previously cloned
`pending_template_instantiations` into a temporary `Vec<String>` before
iteration:

- extra allocation and name cloning on every pass
- overhead proportional to the number of pending template instantiations

## Decision

Consume the current pending set directly with `std::mem::take` and iterate owned
instantiation names.

Behavioral intent:

- remove clone-backed staging overhead
- keep generation semantics intact
- allow newly discovered instantiations to accumulate in
  `pending_template_instantiations` for subsequent iterations

## Implementation

File changed:

- `crates/fragile-clang/src/ast_codegen.rs`

Main updates:

1. `generate_template_instantiations` now uses
   `let instantiations = std::mem::take(&mut self.pending_template_instantiations);`
   and iterates `instantiations` directly.
2. Added focused regression:
   `test_generate_template_instantiations_consumes_pending_set_and_generates_structs`.

## Validation

Commands:

- `cargo test -p fragile-clang test_generate_template_instantiations_consumes_pending_set_and_generates_structs -- --nocapture`
- `cargo build --release -p fragile-cli --bin fragilec`
- strict replay captures with fresh profile/timing artifacts:
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_i_callshape_profile_120_v1.txt`
  - `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_i_callshape_profile_300_v1.txt`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Observed evidence:

- 120s profile reaches `codegen_after_template_collection`.
- 300s profile remains at
  `codegen_after_template_instantiation_generation` with lower bytes than the
  `2.6.c.iv.d.iv.c.i` baseline (`574973 -> 573560`, `-1413`).
- blocker class remains `build_timeout` on `src/rrr/base/misc.cpp`.
- full suites match known baseline (`fragile-clang` lib `728` passed / `46`
  failed; Python `29` passed / `1` skipped).

## Wrong-Approach Guardrails

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-specific conditionals
- no force-native source fallback
- no fake semantic stubs

## Result

Leaf `2.6.c.iv.d.iv.c.iv.c.i` is complete with reduced clone overhead in
pending template-instantiation staging and preserved behavior under regression
coverage.
