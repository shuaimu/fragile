# M9.2.c.iv.e.34.f.5.e.3 `event.cc` residual typed-lane/surface closure inventory

Date: 2026-03-27  
Leaf: `M9.2.c.iv.e.34.f.5.e.3`

## Scope sizing (<1000 LOC)

- One bounded late-pass normalization update in `crates/fragile-clang/src/ast_codegen.rs`.
- Two focused unit tests for the new event normalization pass.
- TODO + inventory documentation updates only.
- Implementation stayed below 1000 LOC.

## Wrong-approach check

Re-reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Conformance:

- no target-specific `mako`/`rpcbench`/`test_rpc` conditionals,
- no force-native bypass,
- no rollback and no fake-success stubs.

## Baseline blocker signature (from e.1 replay)

Source replay run-root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260327T064414Z_p402022`

Event-only segment summary (`lane_fragilec/build.stderr`):

- `event_error_total=167`
- `event_error_codes=E0005=2,E0061=2,E0277=3,E0282=8,E0308=65,E0368=2,E0425=2,E0507=1,E0599=56,E0600=2,E0605=1,E0606=3,E0609=17,E0614=1,E0618=2`
- dominant targeted signatures:
  - `cannot find function fseeko = 1`
  - `super::rrr::print_stack_trace = 1`
  - `no method named __emplace_unique = 4`
  - `no method named empty for __string_view = 9`
  - `std_map_uint32_t__bool::begin = 1`
  - `std_weak_ptr_Event::lock = 1`
  - `std_ofstream::op_shl = 3`

Reference summary file:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260327T064414Z_p402022/lane_fragilec/event_before_e34f5e3_summary.txt`

## Design and implementation

Added a bounded late pass:

- `normalize_rpc_event_surface_artifacts`

Wired in post-processing pipeline after container/internal-node normalization.

The pass applies generic residual rewrites and compat-surface completion for event-lane artifacts:

- rewrite unresolved/global symbol call-shapes (`super::rrr::print_stack_trace`, `fseeko`),
- rewrite degraded `__table_.__emplace_unique(...).first).second` return shape,
- normalize future/shared_future `op_assign` swap call-shape artifacts,
- normalize degraded path/string-view and function-bool call-shape artifacts,
- append missing compat surfaces when absent:
  - `__string_view::{empty,data,length}`,
  - `path::{new_2,append}`,
  - `function_bool__int_::{is_null,op_bool}`,
  - `std_weak_ptr_Event::lock`,
  - `std_map_uint32_t__bool::{begin,end}`,
  - `std_ofstream::{op_shl,close}`,
  - `promise_void::new_1`,
  - bounded `c_void` operator compat traits used by event output.

## Regression coverage executed

Focused unit command:

```bash
cargo test -p fragile-clang normalize_rpc_event_surface_artifacts -- --nocapture
```

Focused assertion coverage includes:

- `test_normalize_rpc_event_surface_artifacts_adds_missing_event_compat_surfaces`
- `test_normalize_rpc_event_surface_artifacts_rewrites_event_callshape_artifacts`

## Focused strict `event.cc` probe evidence

Harness-equivalent strict compile command:

```bash
FRAGILEC_MODE=strict ./target/release/fragilec -c \
  -I vendor/mako/src \
  -I vendor/mako/src/rrr \
  -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako \
  -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1 -std=gnu++23 -w \
  vendor/mako/src/rrr/reactor/event.cc \
  -o /tmp/fragile_e34f5e3_event_compile_after_20260327T111148Z_p646543/event.o
```

Run-root:

- `/tmp/fragile_e34f5e3_event_compile_after_20260327T111148Z_p646543`

After metrics:

- `error_total=118` (from baseline `167`, delta `-49`)
- `E0308=56` (from `65`, delta `-9`)
- `E0599=15` (from `56`, delta `-41`)
- `E0609=16` (from `17`, delta `-1`)
- `E0282=0` (from `8`, delta `-8`)

Targeted signature markers all cleared (`0` each):

- `fseeko`
- `print_stack_trace`
- `__emplace_unique`
- `__string_view::empty`
- `std_map_uint32_t__bool::begin`
- `std_weak_ptr_Event::lock`
- `std_ofstream::op_shl`

## Residual scope

This closes `M9.2.c.iv.e.34.f.5.e.3`.

Remaining leaves under `e.34.f.5.e`:

- `M9.2.c.iv.e.34.f.5.e.4`
- `M9.2.c.iv.e.34.f.5.e.5`
