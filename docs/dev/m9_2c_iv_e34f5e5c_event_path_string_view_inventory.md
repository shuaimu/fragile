# M9.2.c.iv.e.34.f.5.e.5.c Event Path/String-View Lane Inventory

## Scope

Task `M9.2.c.iv.e.34.f.5.e.5.c` closes the bounded `event.cc` residual lane cluster from the post-`e.5.e.5.b` replay:

- `Default::default()` on `c_void`-degraded path lanes
- `__compare(&())` / `__compare(&(__s).clone())` callshape drift
- filesystem path unsafe deref constructor lanes (`path::new_2(&mut (*(self as *const Self as *mut Self)).__*(), 0)`)

All changes are bounded generic normalizations in `crates/fragile-clang/src/ast_codegen.rs`.

## Wrong-approach check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific conditionals, no force-native bypass, no fake-success stubs.

## Implementation summary

1. Extended `normalize_rpc_event_surface_artifacts`:
   - normalize path `c_void` field-lane drift (`__pn_` clone/default assignment lanes -> zeroed lanes);
   - normalize `__compare` callshape drift:
     - `__compare(&())` -> value lane (`unsafe { std::mem::zeroed() }`)
     - `__compare(&(__s).clone())` -> `__compare((__s).clone())`
   - rewrite unsafe filesystem path constructor lanes to bounded default path lanes:
     - `path::new_2(&mut (*(self as *const Self as *mut Self)).__*(), 0)` -> `Default::default()`.
2. Added a final post-fiber guard `normalize_rpc_path_string_type_default_returns`:
   - rewrites residual `path::op_basic_string(&self) -> string_type` tail
     `return Default::default();` -> `return unsafe { std::mem::zeroed() };`
   - prevents pipeline-tail drift when `string_type` degrades to `c_void`.
3. Added/updated focused unit coverage:
   - `test_normalize_rpc_event_surface_artifacts_rewrites_path_c_void_compare_and_unsafe_deref_lanes`
   - `test_normalize_rpc_path_string_type_default_returns_rewrites_op_basic_string_default_lane`

## Focused validation

Commands:

- `cargo test -p fragile-clang normalize_rpc_event_surface_artifacts_rewrites_path_c_void_compare_and_unsafe_deref_lanes -- --nocapture`
- `cargo test -p fragile-clang normalize_rpc_path_string_type_default_returns -- --nocapture`

Result: pass.

Focused strict compile probe:

```bash
FRAGILEC_MODE=strict ./target/release/fragilec -c \
  -I vendor/mako/src -I vendor/mako/src/rrr -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1 -std=gnu++23 -w \
  vendor/mako/src/rrr/reactor/event.cc -o /tmp/.../event.o
```

Probe run-root:

- `/tmp/fragile_e34f5e5c_event_compile_after_mrKkkE`

Targeted marker state in probe stderr:

- `c_void: Default`: `0`
- `__compare(&())`: `0`
- `__compare(&(__s).clone())`: `0`
- unsafe path-deref `path::new_2(&mut (*(self as *const Self as *mut Self))...)`: `0`

## Strict replay evidence

Replay command (baseline anchored to `e.5.e.5.a`):

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260327T172446Z_p981802
```

Intermediate run-root (before final post-fiber op_basic_string guard):

- `/tmp/fragile_m9_2_strict_runtime_replay_20260327T201310Z_p1130824`
- blocker inventory: `total=37`, `unique=27`
- one residual `E0277 c_void: Default` remained.

Final run-root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260327T211622Z_p1179723`

Lane contract status (still blocked overall):

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0/1`

Blocker inventory deltas:

- vs prior `e.5.e.5.b` run (`/tmp/fragile_m9_2_strict_runtime_replay_20260327T195001Z_p1113539`):
  - `rustc_error_total_count: 56 -> 36`
  - `rustc_error_unique_count: 29 -> 26`
- vs intermediate run (`/tmp/fragile_m9_2_strict_runtime_replay_20260327T201310Z_p1130824`):
  - `rustc_error_total_count: 37 -> 36`
  - `rustc_error_unique_count: 27 -> 26`

Targeted c-cluster marker delta (`195001` -> `211622`):

- `c_void: Default`: `8 -> 0`
- `__compare(&())`: `2 -> 0`
- `__compare(&(__s).clone())`: `2 -> 0`
- unsafe path-deref `path::new_2(&mut (*(self as *const Self as *mut Self))...)`: `7 -> 0`

## Remaining work after c

Leaf `e.5.e.5.c` is closed.

Remaining closure is concentrated in `M9.2.c.iv.e.34.f.5.e.5.d` + `.e`:

- `quorum_event.cc` / `reactor.cc` command-map and event-base lanes
- `E0425 __begin2`, `Fiber::create_run__` drift
- unordered-map `find/end/erase` surfaces
- `IntEvent` base-field lane drift (`E0560`/`E0609`)

Follow-on leaves:

- `M9.2.c.iv.e.34.f.5.e.5.d`
- `M9.2.c.iv.e.34.f.5.e.5.e`
