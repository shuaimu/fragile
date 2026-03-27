# M9.2.c.iv.e.34.f.5.d marshal/fiber-context residual compatibility inventory

Date: 2026-03-27  
Leaf: `M9.2.c.iv.e.34.f.5.d`

## Scope sizing (<1000 LOC)

- One bounded late normalization pass in `ast_codegen`.
- Focused unit tests for marshal/fiber-context residual lane closure.
- TODO + closure-doc updates only.
- Implementation stayed below 1000 LOC.

## Wrong-approach check

Re-reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Conformance:

- no target-specific `mako`/`rpcbench`/`test_rpc` conditionals,
- no force-native bypass,
- no rollback or fake-success stubbing.

## Baseline residual signatures

Primary replay source:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260326T205524Z_p4045206/lane_fragilec/build.stderr`

Concrete residual references:

- `/tmp/fragilec_transpiled/marshal.cpp_c4e047655077a443_marshal.rs`
- `/tmp/fragilec_transpiled/fiber_context_runtime.cc_3cff9cf06085a213_fiber_context_runtime.rs`

Targeted marker families:

- `rrr_Marshallable` placeholder missing field lanes:
  - `kind_`
  - `bypass_to_socket_`
  - `__vtable`
- marshal lifetime/type mismatches:
  - `create_actual_object_from(&mut self, m: &mut Marshal) -> &mut Marshal`
  - `track_write_2` write-return lane (`i64` -> `u64` mismatch)
- degraded call-shape in fiber context:
  - `boost_coro_yield_t::new_1(&mut &mut __self as *mut Self)`

## Design and implementation

Added a bounded late pass:

- `normalize_rpc_marshal_fiber_context_artifacts`

Implemented closures:

1. Rehydrate degraded `rrr_Marshallable` placeholder lanes to include:
   - `__vtable`
   - `kind_`
   - `bypass_to_socket_`
   - `written_to_socket`
2. Rewrite marshal lifetime/type artifacts:
   - `create_actual_object_from` signature rehydrated with explicit `'a`.
   - `track_write_2` write call cast to `u64` lane.
3. Normalize residual degraded stdlib artifacts surfaced by marshal TU:
   - `__in_pattern_i8` tuple return -> `__in_pattern_result` struct return.
   - `__gv___from_chars_log2f_lut` literals normalized to `f32` lane literals.
4. Fix fiber-context constructor callshape:
   - `boost_coro_yield_t::new_1(&mut &mut __self as *mut Self)` ->
     `boost_coro_yield_t::new_1(&mut __self)`.

## Regression coverage executed

Focused test command:

```bash
cargo test -p fragile-clang --lib normalize_rpc_marshal_fiber_context_artifacts -- --nocapture
```

Focused assertions:

- `test_normalize_rpc_marshal_fiber_context_artifacts_rehydrates_rrr_marshallable_lanes_and_marshal_lifetimes`
- `test_normalize_rpc_marshal_fiber_context_artifacts_fixes_in_pattern_and_from_chars_lut_lanes`
- `test_normalize_rpc_marshal_fiber_context_artifacts_fixes_boost_coro_yield_constructor_callshape`

## Residual scope

This closes `M9.2.c.iv.e.34.f.5.d`.

Remaining closure leaf:

- `M9.2.c.iv.e.34.f.5.e`
