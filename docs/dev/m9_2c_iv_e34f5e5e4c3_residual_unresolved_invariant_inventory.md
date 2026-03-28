# M9.2.c.iv.e.34.f.5.e.5.e.4.c.3 residual unresolved-invariant closure inventory

Date: 2026-03-28
Task leaf: `M9.2.c.iv.e.34.f.5.e.5.e.4.c.3`

## Scope

Resolve the post-c.2 unresolved-type invariant stop that blocked strict replay in:

- `event.cc`
- `fiber_context_runtime.cc`
- `fiber_impl.cc`
- `quorum_event.cc`

without target-specific conditionals or native fallback.

## Wrong-approach check

- Reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Reviewed `docs/dev/wrong.md`.
- No force-native path, no target-specific branch, no fake pass-through stubs.

## Implementation

Bounded invariant-filter update in both compile entrypoints:

- `crates/fragile-cli/src/bin/fragilec.rs`
- `crates/fragile-driver/src/lib.rs`

Added source-location pseudo-type names to known-internal invariant filtering:

- `File`
- `Line`
- `Column`
- `Function`

Rationale: these are macro metadata tokens that can surface in generated signatures and are not real unresolved runtime type dependencies.

## Focused regression tests

Added/validated focused tests:

- `fragile-driver`:
  - `tests::known_internal_type_source_location_tokens`
  - `tests::unresolved_type_invariant_passes_for_source_location_tokens`
- `fragile-cli`:
  - `tests::known_internal_type_source_location_tokens`
  - `tests::unresolved_type_invariant_passes_for_source_location_tokens`

Commands:

```bash
cargo test -p fragile-driver source_location_tokens -- --nocapture
cargo test -p fragile-cli source_location_tokens -- --nocapture
```

Result: all passed.

## Strict replay evidence

Command:

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260328T162346Z_p2277812
```

Run-root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260328T201655Z_p2479647`

Manifest shift:

- baseline `first_error_key`:
  - `error:fragilec:[fragilec] fragile unresolved-type invariant failed for .../reactor/event.cc: File, Line`
- post-c.3 `first_error_key`:
  - `error:fragilec:[fragilec] fragile rustc object compile failed for .../reactor/quorum_event.cc (parser-output-handoff)`

Lane contract (still blocked):

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0/1`

Current dominant residuals moved past the previous unresolved-invariant stop:

- `reactor.cc` / `quorum_event.cc` typed lanes (`E0308`, `E0133`, `E0425`)
- `rpc/client.cpp` unresolved-type invariant (`rrr_Client_const`)

## Conclusion

`c.3` closure goal met: prior unresolved-type invariant stop in the four reactor files is cleared, and replay now progresses to deeper typed/symbol blockers for next-leaf handling.
