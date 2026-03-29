# M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.b rpc/client const-invariant closure inventory

Date: 2026-03-28
Task leaf: `M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.b`

## Scope

Resolve the `rpc/client.cpp` unresolved-type invariant stop for `rrr_Client_const`
with a bounded deterministic normalization that does not introduce generic/trait
alias regressions.

## Wrong-approach check

- Reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Reviewed `docs/dev/wrong.md`.
- No target-specific conditionals, no force-native bypass, no fake semantic stubs.

## Implementation

Bounded invariant update:

- Added `AstCodeGen::has_concrete_defined_type_name(code, name)` in
  `crates/fragile-clang/src/ast_codegen.rs`.
- Added `_const` unresolved-type filter in both strict invariants:
  - `crates/fragile-cli/src/bin/fragilec.rs`
  - `crates/fragile-driver/src/lib.rs`
- Filter behavior is deliberately constrained:
  - allow unresolved `<Type>_const` only when concrete `<Type>` is defined in
    the same transpiled unit;
  - reject generic heads and trait heads.

Focused tests added:

- `test_has_concrete_defined_type_name_accepts_non_generic_structs_and_aliases`
- `test_has_concrete_defined_type_name_rejects_generic_heads_and_traits`
- `unresolved_type_invariant_passes_for_const_suffix_alias_with_concrete_base`
  (both `fragilec` and `fragile-driver`)
- `unresolved_type_invariant_rejects_const_suffix_alias_for_generic_or_trait_heads`
  (both `fragilec` and `fragile-driver`)

Focused commands executed:

```bash
cargo test -p fragile-clang has_concrete_defined_type_name -- --nocapture
cargo test -p fragile-clang const_suffix_qualifier_aliases -- --nocapture
cargo test -p fragile-driver unresolved_type_invariant_passes_for_const_suffix_alias_with_concrete_base -- --nocapture
cargo test -p fragile-driver unresolved_type_invariant_rejects_const_suffix_alias_for_generic_or_trait_heads -- --nocapture
cargo test -p fragile-cli unresolved_type_invariant_passes_for_const_suffix_alias_with_concrete_base -- --nocapture
cargo test -p fragile-cli unresolved_type_invariant_rejects_const_suffix_alias_for_generic_or_trait_heads -- --nocapture
```

Result: all passed.

## Strict replay evidence

Command:

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260328T211915Z_p2548616
```

Run-root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260328T230041Z_p2676907`

Manifest highlights:

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `blocker_error_total_count=264`
- `blocker_error_unique_count=91`
- `first_error_key=...quorum_event.cc parser-output-handoff compile failure`
- `non_increase_total_vs_baseline=false`
- `non_increase_unique_vs_baseline=false`
- `non_increase_verdict=false`

Key c.4.b closure signal:

- Prior stop marker is gone:
  - `[fragilec] fragile unresolved-type invariant failed for .../rpc/client.cpp: rrr_Client_const`
  - this marker does not appear in the new run-root `lane_fragilec/build.stderr`.
- Replay now advances to typed rustc blockers in `rpc/client.cpp`, including:
  - `E0425: cannot find type rrr_Client_const`
  - plus broader typed lanes (`E0308`, `E0599`, `E0609`, etc.).

## Conclusion

`c.4.b` is complete: the `rpc/client.cpp` unresolved-type invariant stop for
`rrr_Client_const` is removed with bounded generic safeguards against
const-suffix generic/trait alias regressions. Residual work moves to
`M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.c` (typed reactor/quorum/client clusters) and
`M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.d` (next replay gate).
