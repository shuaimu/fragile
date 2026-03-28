# M9.2.c.iv.e.34.f.5.e.5.e.4.b Invalid-Null Slice Compare Lane Closure

## Scope

- Task leaf: `M9.2.c.iv.e.34.f.5.e.5.e.4.b`
- Goal: remove shared strict-rustc `invalid_null_arguments` aborts in `event.cc` and `fiber_impl.cc` caused by degraded null-byte-slice compare lane.

Wrong-approach check completed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

## Residual Before Fix

From replay run-root `/tmp/fragile_m9_2_strict_runtime_replay_20260328T092947Z_p1922380`:

- `lane_fragilec/build.stderr` blocked on:
  - `error: calling this function with a null pointer is undefined behavior`
  - marker lane:
    - `std::slice::from_raw_parts(std::ptr::null() as *const u8, (self.len_) as usize)`
- affected object files:
  - `vendor/mako/src/rrr/reactor/event.cc`
  - `vendor/mako/src/rrr/reactor/fiber_impl.cc`

## Implementation

Bounded normalization in `normalize_rpc_event_surface_artifacts`:

1. Trigger guard widened to run when the null-byte-slice marker is present.
2. Rewrote degraded compare lane:
   - from:
     - `std::slice::from_raw_parts(std::ptr::null() as *const u8, (self.len_) as usize)`
   - to:
     - `&[]`

This keeps the change generic for the shared event/fiber artifact family while avoiding file-specific conditionals.

## Validation

Focused unit test:

- `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_null_slice_compare_lane_to_empty_slice -- --nocapture`

Focused strict probes (harness-equivalent flags) after rebuilding release `fragilec`:

- run-root: `/tmp/fragile_e34f5e5e4b_focus_after_20260328T112432Z_p2017254`
- summary:
  - `event_status=0`
  - `fiber_status=0`
  - `event_invalid_null_arguments_count=0`
  - `fiber_invalid_null_arguments_count=0`
  - `event_null_from_raw_parts_marker_count=0`
  - `fiber_null_from_raw_parts_marker_count=0`

Evidence files:

- `/tmp/fragile_e34f5e5e4b_focus_after_20260328T112432Z_p2017254/summary.txt`
- `/tmp/fragile_e34f5e5e4b_focus_after_20260328T112432Z_p2017254/event.stderr`
- `/tmp/fragile_e34f5e5e4b_focus_after_20260328T112432Z_p2017254/fiber_impl.stderr`

## Next Leaf

- `M9.2.c.iv.e.34.f.5.e.5.e.4.c` (strict replay rerun + full lane-contract verification)
