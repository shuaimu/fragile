# M9.2.c.iv.f.5.b - E0599 Compatibility-Surface Slice Inventory

## Scope

Leaf `M9.2.c.iv.f.5.b` executes the first bounded dominant `E0599` residual slice from `f.5`:

- target: missing RPC/marshal/map-like compatibility surfaces,
- bounded edits: `<=400 LOC` in production code,
- no target-specific branching and no force-native bypass.

Primary compile units in this slice:

- `rpc/client.cpp`
- `rpc/server.cpp`

Pre-fix baseline anchor:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835/lane_fragilec/build.stderr`

Post-fix focused probe root:

- `/tmp/fragile_f5b_probe_after_20260330T153529Z_txlog`

## Wrong-Approach Check

Re-reviewed before edits:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Confirmed guardrails:

- no force-native fallback,
- no target-only hacks,
- no suppression-only accounting.

## Implementation

Touched file:

- `crates/fragile-clang/src/ast_codegen.rs`

Extended bounded pass:

- `normalize_e0061_e0599_rpc_surface_compatibility_slice`

Implemented compatibility closures:

1. Rehydrated missing marshal surfaces on `rrr_Marshal` when referenced:
   - `content_size`
   - `write_to_fd`
   - `empty`
2. Added missing server reply surface:
   - `rrr_ServerConnection::reply<T>(&mut self, &T, i32)`
3. Added map/container-like Vec compatibility trait:
   - `FragileVecSizeOpIndexCompat<T>` for `Vec<T>` with `size` and `op_index`
4. Added Box deref compatibility trait:
   - `FragileBoxOpDerefCompat<T>` for `Box<T>`
5. Added random-device call compatibility:
   - `std_random_device::op_call`
6. Normalized request-options retry callshape:
   - `.can_retry(args...)` -> `.can_retry()`

These changes stay generic and avoid target-conditional logic.

## Unit Tests

Focused pass tests:

- `cargo test -p fragile-clang normalize_e0061_e0599_rpc_surface_compatibility_slice -- --nocapture`

Covered:

- arity trimming and callback callshape normalization,
- lock/method surfaces from prior slice,
- new marshal/server/vec/box/random compatibility closures in:
  - `test_normalize_e0061_e0599_rpc_surface_compatibility_slice_adds_marshal_rpc_vec_box_and_random_surfaces`

## Focused Probe Results

### Pre-fix E0599 Baseline (from anchored replay root)

`rpc/client.cpp`: `15`

- `Cell::get` trait-bounds: `3`
- `op_sub`: `2`
- raw-pointer `op_arrow`: `2`
- `content_size`: `2`
- `empty`: `2`
- `make_ref_mut_i64`: `1`
- `can_retry`: `1`
- `write_to_fd`: `1`
- `time_since_epoch`: `1`

`rpc/server.cpp`: `12`

- `size`: `3`
- `empty`: `2`
- `op_call`: `2`
- `write_to_fd`: `1`
- `reply`: `1`
- `op_deref`: `1`
- `op_index`: `1`
- `time_since_epoch`: `1`

Subtotal on dominant units: `27`.

### Post-fix E0599 (focused txlog probe)

Source:

- `/tmp/fragile_f5b_probe_after_20260330T153529Z_txlog/summary.txt`

`rpc/client.cpp`: `9`

- `Cell::get` trait-bounds: `3`
- `op_sub`: `2`
- raw-pointer `op_arrow`: `2`
- `make_ref_mut_i64`: `1`
- `time_since_epoch`: `1`

`rpc/server.cpp`: `2`

- raw-pointer `op_arrow`: `1`
- `time_since_epoch`: `1`

Subtotal on dominant units: `11`.

### Delta

| compile unit | pre E0599 | post E0599 | delta |
|---|---:|---:|---:|
| `rpc/client.cpp` | 15 | 9 | -6 |
| `rpc/server.cpp` | 12 | 2 | -10 |
| **total** | **27** | **11** | **-16** |

Cleared signatures in this slice:

- marshal/server lanes:
  - `content_size`, `write_to_fd`, `empty`, `reply`
- container/deref lanes:
  - `size`, `op_index`, `op_deref`
- random/request-options lanes:
  - `op_call`, `can_retry`

## Residuals Forward

Residual `E0599` signatures after `f.5.b` (dominant units) are now:

- numeric/pointer lanes: `op_sub`, raw-pointer `op_arrow`, `time_since_epoch`
- pointer helper lane: `make_ref_mut_i64`
- trait-bound lane: `Cell::get` (Copy-bound unsatisfied)

These roll into follow-on bounded slices (`f.5.c` and `f.5.d`) along with dominant `E0308`/supporting families.
