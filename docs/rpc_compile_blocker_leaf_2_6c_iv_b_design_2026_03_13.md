# RPC Compile Blocker Leaf 2.6.c.iv.b Design Note (2026-03-13)

## Scope

Leaf: `2.6.c.iv.b`  
Objective: implement the next generic codegen hot-path optimization indicated by
`2.6.c.iv.a` profiling data, then lock behavior with focused regressions.

## Input Evidence

From `2.6.c.iv.a` strict replay captures:

- `/tmp/fragile_rpc_leaf_2_6c_iv_a_callshape_profile_120_v4.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_a_callshape_profile_300_v1.txt`

Both reported:

- `status=codegen_started`
- no `normalize_problematic_callshape_artifacts` counters

Interpretation: timeout happens earlier than the callshape-normalizer phase.

## Wrong-Approach Guard

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-name conditionals (`rpcbench`/`test_rpc`)
- no force-native bypass
- no synthesized fake method bodies
- preserve generic traversal/codegen semantics

## Chosen Optimization

### Problem

`AstCodeGen::generate` previously executed `collect_template_info` twice.
Each pass traversed the AST with expensive usage work:

- template type use collection
- function template call-site instantiation inference

The second pass existed to recover uses/calls that appear before template
definitions in AST order.

### Change

Split collection into two explicit internal passes:

1. `collect_template_definitions_with_namespace(...)`
2. `collect_template_usages_with_namespace(...)`

Then run `collect_template_info(...)` once as:

- definition prepass
- usage pass

Also removed the old duplicate external `collect_template_info` invocation in
`generate`.

This preserves before-definition behavior while removing duplicated heavy usage
traversal work.

### Size

Implementation remains well within the requested small-task bound (`<500 LOC`,
roughly a few hundred lines touched including tests/docs).

## Regression Coverage

Added focused unit tests in `crates/fragile-clang/src/ast_codegen.rs`:

- `test_function_template_call_before_template_definition_still_instantiates`
- `test_class_template_type_use_before_template_definition_still_instantiates`

These lock the exact semantics the old second-pass behavior covered.

## Validation

Commands run:

```bash
cargo test -p fragile-clang template_definition_still_instantiates -- --nocapture
cargo test -p fragile-clang problematic_callshape -- --nocapture
python3 -m unittest discover -s tests/python -p 'test_*.py'
cargo test --workspace --all-targets
```

Results:

- new focused template-collection tests: pass
- problematic-callshape tests: pass
- python suite: pass (`29` tests, `1` skipped)
- full workspace cargo suite: known pre-existing `fragile-clang` lib baseline
  failures remain (`46` failures), unchanged from pre-leaf status

Strict replay evidence after optimization:

```bash
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_b_callshape_profile_120_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_b_stage_timing_120_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec --max-replays 1 --timeout-seconds 120

FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_b_callshape_profile_300_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_b_stage_timing_300_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec --max-replays 1 --timeout-seconds 300
```

Observed:

- replay remains `build_timeout` on `src/rrr/base/misc.cpp`
- callshape profile still records `status=codegen_started`

## Outcome

Leaf `2.6.c.iv.b` is complete:

- generic early codegen hot-path optimization implemented
- before-definition template semantics locked by focused tests
- strict timeout replay artifacts updated deterministically for next iteration
