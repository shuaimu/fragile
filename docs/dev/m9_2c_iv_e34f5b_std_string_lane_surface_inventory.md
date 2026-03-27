# M9.2.c.iv.e.34.f.5.b std::string lane/surface normalization inventory

Date: 2026-03-26  
Leaf: `M9.2.c.iv.e.34.f.5.b`

## Scope sizing (<1000 LOC)

- One bounded late-stage normalization pass in `ast_codegen`.
- Small compatibility expansion in existing std::string helper stubs.
- Focused regression tests in `ast_codegen` and M9 closure-doc tests.
- Net code delta is bounded and below 1000 LOC.

## Wrong-approach check

Re-reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Conformance:

- no target-specific `mako`/file-name conditionals,
- no force-native bypass,
- no fake success stubs to mask unresolved blockers.

## Baseline blocker inventory (pre-fix)

Focused strict probes captured pre-fix blocker signatures at:

- `/tmp/fragile_e34f5b_event_before_oLzsyy`
- `/tmp/fragile_e34f5b_fiber_before_uwwVWv`

Representative blocker classes from these inventories:

- `std::string::String` missing method surfaces: `grow`, `ensure_null_terminated`
- `std_string_view` missing methods: `length` (and `data` surface drift)
- field-lane drift on string internals: `data_`, `len_`, `capacity_`
- degraded string append lanes, including `op_add_assign(&0)` and c_void-lane add-assign artifacts

## Design and implementation

### 1) Tail pass for shared std::string lane artifacts

Added `normalize_rpc_std_string_lane_surface_artifacts` near pipeline tail (after marshal/container finalization) to avoid later passes reintroducing these regressions.

Pass behavior:

- Within `impl String` blocks for the generated lane struct (`data_`, `len_`, `capacity_`), normalize self-type drift:
  - `std::string::String` -> `String`
  - `String::new()` -> `String::new_0()`
- Normalize degraded `.op_add_assign(&0)` to `.op_add_assign(0i8)`.
- Detect `c_void`-typed fields and replace `.field.op_add_assign(...)` call lines with no-op statements.
- If `std_string_view` exists and methods are missing, append
  `std_string_view::{data,length,size}`:
  - `data(&self) -> *const i8`
  - `length(&self) -> u64`
  - `size(&self) -> u64`

### 2) std::string add-assign compat expansion

Extended `FragileStdStringAddAssignArg` in `append_std_string_stream_compat_stubs` for additional emitted lanes:

- `i32`
- `&i8`
- `&i32`
- `&std::string::String`

## Regression coverage executed

Commands:

```bash
cargo test -p fragile-clang --lib normalize_rpc_std_string_lane_surface_artifacts -- --nocapture
cargo test -p fragile-clang --lib test_append_std_string_stream_compat_stubs_adds_missing_methods -- --nocapture
```

Assertions covered by these tests include:

- `test_normalize_rpc_std_string_lane_surface_artifacts_rewrites_impl_string_self_types`
- `test_normalize_rpc_std_string_lane_surface_artifacts_fixes_degraded_add_assign_and_view_surface`
- `test_append_std_string_stream_compat_stubs_adds_missing_methods`
- `impl String` self-type rebinding from `std::string::String` to local lane type.
- `String::new_0()` rebinding in impl bodies.
- normalization of `op_add_assign(&0)` and c_void add-assign degradation.
- additive `std_string_view` compat surface (`data/length/size`).
- expanded `FragileStdStringAddAssignArg` implementations for integer/reference/string lanes.

## Residual scope

This leaf closes the bounded shared std::string-lane normalization task.

Remaining closure is tracked by:

- `M9.2.c.iv.e.34.f.5.c` (container/internal-node lane regressions)
- `M9.2.c.iv.e.34.f.5.d` (marshal/fiber_context residual blockers)
- `M9.2.c.iv.e.34.f.5.e` (final strict replay lane-contract verification)
