# M9.2.c.iv.e.34.f.5.e.5.b Reactor Shared-Straggler Inventory

## Scope

Task `M9.2.c.iv.e.34.f.5.e.5.b` resolves the shared cross-TU reactor-family straggler surfaces from the post-e.5.e.4 replay inventory:

- `print_stack_trace` path drift (`super::rrr`/`crate::rrr` callshape mismatch)
- `weak_ordering` return-lane drift (`expected weak_ordering, found partial_ordering`)
- pointer-event `log` callshape drift (`*mut rrr::Event` method-call shape)

All fixes are bounded generic normalizations in `crates/fragile-clang/src/ast_codegen.rs`.

## Wrong-approach check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific conditionals, no force-native bypass, no fake-success stubs.

## Implementation summary

1. Extended `normalize_rpc_event_surface_artifacts`:
   - normalize both `super::rrr::print_stack_trace(...)` and `crate::rrr::print_stack_trace(...)` to `super::print_stack_trace(...)`;
   - append an extern bridge only when missing:
     - `__fragile_extern_print_stack_trace`
     - `pub fn print_stack_trace(fp: *mut std::ffi::c_void)`
   - normalize residual `PARTIAL_ORDERING_EQUIVALENT`/`partial_ordering` return shape back to `weak_ordering` lane.
2. Extended `normalize_rpc_fiber_surface_artifacts`:
   - normalize pointer-event log callshape to dereference the event pointer before method call:
     - `unsafe { (*(*self.events_.op_index(i)).op_arrow()).log(); }`
   - removed the broad rewrite that reintroduced `weak_ordering -> partial_ordering` lane drift.
3. Added/updated unit coverage:
   - `test_normalize_rpc_event_surface_artifacts_rewrites_event_callshape_artifacts`
   - `test_normalize_rpc_event_surface_artifacts_rewrites_crate_rrr_print_stack_trace_path`
   - `test_normalize_rpc_fiber_surface_artifacts_rewrites_fiber_callshape_and_lane_artifacts`

## Focused validation

Commands:

- `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_event_callshape_artifacts -- --nocapture`
- `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_crate_rrr_print_stack_trace_path -- --nocapture`
- `cargo test -p fragile-clang test_normalize_rpc_fiber_surface_artifacts_rewrites_fiber_callshape_and_lane_artifacts -- --nocapture`

Result: all pass.

## Strict replay evidence

Replay command (baseline anchored to e.5.e.5.a):

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260327T172446Z_p981802
```

Final run-root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260327T195001Z_p1113539`

Lane contract status (still blocked overall):

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0/1`

Blocker inventory delta vs baseline (`/tmp/fragile_m9_2_strict_runtime_replay_20260327T172446Z_p981802`):

- `rustc_error_total_count: 154 -> 56`
- `rustc_error_unique_count: 77 -> 29`
- `non_increase_verdict=true`

Targeted marker delta (shared stragglers):

- `print_stack_trace`: `4 -> 0`
- `cannot find function \`print_stack_trace\``: `4 -> 0`
- `weak_ordering`, found `partial_ordering`: `4 -> 0`
- `raw pointer `*mut rrr::Event``: `4 -> 0`
- `expected \`weak_ordering\`, found \`partial_ordering\``: `4 -> 0`
- `no method named \`log\` found for raw pointer \`*mut rrr::Event\``: `4 -> 0`

Intermediate confirmation run-root during closure:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260327T191426Z_p1085529`
  - `print_stack_trace=0`, `log=0`, `weak_ordering_mismatch=3` (before final weak-ordering rewrite closure)

## Remaining work after b

Leaf `e.5.e.5.b` is closed; replay remains blocked by non-b clusters (primarily c/d scope):

- `event.cc` `__string_view`/path/c_void lanes (`E0277`, `E0308`, `E0133`, `E0599 clone`)
- `quorum_event.cc`/`reactor.cc` command-map + event-base surfaces (`E0425 __begin2`, `E0560/E0609 IntEvent lanes`, missing unordered-map methods, `Fiber::create_run__` drift)

Follow-on leaves remain:

- `M9.2.c.iv.e.34.f.5.e.5.c`
- `M9.2.c.iv.e.34.f.5.e.5.d`
- `M9.2.c.iv.e.34.f.5.e.5.e`
