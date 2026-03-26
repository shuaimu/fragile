# M9.2.c.iv.e.34.f.4 marshal compatibility-surface closure inventory

Date: 2026-03-26  
Leaf: `M9.2.c.iv.e.34.f.4`

## Scope sizing (<1000 LOC)

- Added one bounded late normalization pass in `ast_codegen`:
  - `normalize_rpc_marshal_surface_artifacts`
- Added focused unit tests for the new pass.
- Added closure-documentation tests in `m9_rpc_closure_tests.rs`.
- Total implementation/testing changes are bounded and remain well below 1000 LOC.

## Wrong-approach check

- Re-reviewed:
  - `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
  - `docs/dev/wrong.md`
- No target-specific (`mako`/`rpc`) conditionals were introduced.
- No force-native bypass path was introduced.
- No rollback-pattern expansion was introduced.

## Design decisions

From the `e.34.f.1`/`e.34.f.3` inventory and focused marshal baseline
(`/tmp/fragile_e34f4_marshal_before_udF2hZ`), the dominant residuals were
placeholder-lane and compatibility-surface gaps concentrated in:

- `std_shared_ptr` method surfaces (`op_arrow`, `op_eq`)
- `MarshallDeputy_MarContainer` method lanes (`find`, `end`)
- `rrr_Marshal` surface lanes (`read`, `write`, `peek`, `op_shl`, `op_shr`)
- `chunk` lane/method surface (`next`, `reset`, `fully_*`, `read/write`, fd I/O, bookmark lane)
- `Marshal_bookmark` placeholder field lanes and `bookmark`-typed local artifacts
- marshal call-shape/type-lane degradations (`*v.op_eq`, `*m.op_shl`, `*m.op_shr`, pointer deref/cast artifacts, enum/int bitand cast precedence)

Chosen bounded fix:

- Added `normalize_rpc_marshal_surface_artifacts` as a late marshal-focused compatibility pass in the normalization tail.
- The pass performs bounded textual normalizations for known degraded marshal lane artifacts, then appends generic compat impl surfaces for unresolved placeholder structs/types.
- Compat insertion now uses impl-block-aware method detection so reruns stay idempotent and existing method surfaces are not duplicated.

## Implemented closures in code

Primary implementation file:

- `crates/fragile-clang/src/ast_codegen.rs`

Key additions:

- New pass wiring:
  - `normalize_rpc_marshal_surface_artifacts` invoked in the late normalization tail.
- New pass behavior:
  - call-shape rewrites for marshal/operator/bookmark/container artifacts,
  - placeholder struct rehydration for `chunk` and `Marshal_bookmark`,
  - compat impl emission for:
    - `std_shared_ptr<T>` (`op_arrow`, `op_eq`)
    - `MarshallDeputy_MarContainer` (`find`, `end`)
    - `chunk` (`reset`, `fully_written`, `fully_read`, `write`, `read`, `read_from_fd`, `write_to_fd`, `content_size`, `resize_to_current`, `is_shared_data_chunk`, `set_bookmark`)
    - `rrr_Marshal` (`write`, `read`, `peek`, `op_shl`, `op_shr`)
    - `rrr_v32`/`rrr_v64` (`set`)

## Focused test evidence

Unit tests added in `ast_codegen`:

- `test_normalize_rpc_marshal_surface_artifacts_rehydrates_placeholder_struct_lanes`
- `test_normalize_rpc_marshal_surface_artifacts_rewrites_marshal_callshape_artifacts`
- `test_normalize_rpc_marshal_surface_artifacts_adds_rrr_marshal_and_value_compat_surfaces`
- `test_normalize_rpc_marshal_surface_artifacts_adds_only_missing_shared_ptr_methods`
- `test_normalize_rpc_marshal_surface_artifacts_ignores_other_type_op_arrow_methods`
- `test_normalize_rpc_marshal_surface_artifacts_is_idempotent_for_compat_impls`

Focused run:

- `cargo test -p fragile-clang --lib normalize_rpc_marshal_surface_artifacts -- --nocapture`
- Result: pass (`6 passed`, `0 failed`).

Closure tests added:

- `m9_2c_iv_e34f4_task_documented_in_todo`
- `m9_2c_iv_e34f4_inventory_document_exists_and_records_marshal_surface_closure`

## Baseline reference

- Prior marshal focused baseline inventory root (pre-f.4 implementation reference):
  - `/tmp/fragile_e34f4_marshal_before_udF2hZ`
