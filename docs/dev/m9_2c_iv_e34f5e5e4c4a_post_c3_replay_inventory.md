# M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.a post-c.3 replay inventory refresh

Date: 2026-03-28
Task leaf: `M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.a`

## Scope

Capture a deterministic strict replay refresh after `c.3`, validate non-increase against the post-c.3 baseline, and keep const-suffix qualifier alias handling bounded/safe.

## Wrong-approach check

- Reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Reviewed `docs/dev/wrong.md`.
- No target-specific conditionals, no force-native bypass, no fake stubs.

## Implementation

Bounded qualifier-family update in `crates/fragile-clang/src/ast_codegen.rs`:

- Added `_const` -> base sibling handling in `qualifier_family_siblings`.
- Added guards so qualifier-family alias fallback does **not** target:
  - generic type heads (for example `std_vector<T>`), or
  - trait declarations.

This keeps aliasing deterministic and prevents invalid fallback aliases like
`std_vector_const = std_vector`.

Focused tests added:

- `test_close_unresolved_type_reference_gaps_adds_const_suffix_qualifier_aliases`
- `test_close_unresolved_type_reference_gaps_does_not_alias_const_suffix_to_generic_type_heads`

Focused test commands:

```bash
cargo test -p fragile-clang const_suffix_qualifier_aliases -- --nocapture
cargo test -p fragile-clang does_not_alias_const_suffix_to_generic_type_heads -- --nocapture
```

Result: passed.

## Strict replay evidence

Command:

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260328T201655Z_p2479647
```

Run-root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260328T211915Z_p2548616`

Manifest delta vs baseline `/tmp/fragile_m9_2_strict_runtime_replay_20260328T201655Z_p2479647`:

- `rustc_error_total_count: 31 -> 31`
- `rustc_error_unique_count: 22 -> 22`
- `first_error_key`: unchanged (`quorum_event.cc` parser-output-handoff compile failure)
- `non_increase_total_vs_baseline=true`
- `non_increase_unique_vs_baseline=true`
- `non_increase_verdict=true`

Lane contract remains blocked:

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0/1`

Residual blockers remain:

- `rpc/client.cpp` unresolved-type invariant: `rrr_Client_const`
- `reactor.cc` / `quorum_event.cc` typed cluster (`E0425`, `E0308`, `E0133`, `E0121`, `E0277`, `E0596`)

## Conclusion

`c.4.a` is complete: deterministic replay refresh captured, qualifier-family safety guard validated, and blocker non-increase re-established vs post-c.3 baseline. Remaining closure work is isolated to `c.4.b` and `c.4.c`.
