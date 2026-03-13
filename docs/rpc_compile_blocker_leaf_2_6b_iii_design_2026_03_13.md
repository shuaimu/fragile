# RPC Compile Blocker Leaf 2.6.b.iii Design (2026-03-13)

## Objective

Leaf `2.6.b.iii` requires a generic fix for the blocker class captured in `2.6.b.ii`
(`unresolved_name_or_type_e0425`) and focused compiler regressions to prevent recurrence.

## Scope Sizing

This leaf was completed in a small change set (<500 LOC):

- one codegen normalization update in `crates/fragile-clang/src/ast_codegen.rs`
- two focused unit regressions in the same test module
- TODO/dev-book documentation updates

No further decomposition was necessary.

## Blocker Analysis

The latest strict `2.6.a` replay remains timeout-bound (`build_timeout`), so non-timeout
`E0425` subtype analysis used deterministic archived non-timeout capture from:

- `/tmp/fragile_rpc_leaf_2_5_current_20260313/lane_fragilec/build.stderr`

Observed unresolved helper calls were dominated by:

- `signal`
- `getopt`
- `atoi`

These helpers already exist in the runtime header surface (`fragile-stl`), indicating the
remaining issue is call-path resolution/scope qualification in lowered code, not missing helper
surfaces.

## Decision

Implement a generic runtime path normalization that rewrites bare helper calls to crate-qualified
calls:

- `signal(...)` -> `crate::signal(...)`
- `getopt(...)` -> `crate::getopt(...)`
- `atoi(...)` -> `crate::atoi(...)`

with safeguards that preserve:

- helper definitions (`pub fn signal...`, etc.)
- already-qualified calls (`crate::`, `super::`, `self::`)

This keeps behavior on existing runtime shims while removing unresolved bare-call lookup failures
inside nested module scopes.

## Wrong-Approach Check

Checked against `docs/dev/wrong.md` and project constraints:

- no RPC-target-specific code paths
- no force-native bypass
- no fake semantic method bodies added
- no synthetic success markers

The fix is generic normalization on emitted Rust call paths only.

## Implementation

Updated `crates/fragile-clang/src/ast_codegen.rs`:

- added `normalize_bare_runtime_helper_calls`
- extended `normalize_known_runtime_path_misresolutions` to apply helper qualification for
  `signal`, `getopt`, and `atoi`

Added focused tests:

- `test_normalize_known_runtime_path_misresolutions_qualifies_bare_runtime_helpers`
- `test_normalize_known_runtime_path_misresolutions_keeps_runtime_helper_definitions`

## Validation

- `cargo test -p fragile-clang test_normalize_known_runtime_path_misresolutions_ -- --nocapture`

