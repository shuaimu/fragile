# M9.2.c.iv.e.34.f.5.c container/internal-node compatibility inventory

Date: 2026-03-27  
Leaf: `M9.2.c.iv.e.34.f.5.c`

## Scope sizing (<1000 LOC)

- One bounded late-stage normalization pass in `ast_codegen`.
- Focused unit tests for tree-lane and unordered-set compat closure.
- TODO + closure-doc evidence updates only.
- Net implementation stayed below 1000 LOC.

## Wrong-approach check

Re-reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Conformance:

- no target-specific `mako`/file-name conditionals,
- no force-native bypass,
- no rollback expansion or fake “success” masking.

## Baseline blocker signatures (pre-fix)

Focused strict blocker signatures were captured in prior f.5 probes:

- `/tmp/fragile_e34f5b_event_before_oLzsyy/stderr.log`
- `/tmp/fragile_e34f5b_fiber_before_uwwVWv/stderr.log`

f.5.c-targeted residual markers:

- `E0609` on degraded tree lanes:
  - `no field __end_node_`
  - `no field __begin_node_`
  - `no field __size_`
- `E0599` on `std_unordered_set_int`:
  - missing `find`, `end`, `insert`, `begin`

## Design and implementation

### 1) `normalize_rpc_container_internal_node_artifacts`

Added a new bounded late pass in `ast_codegen`:

- Detect `impl __tree_*` blocks that reference any of:
  - `self.__begin_node_`
  - `self.__end_node_`
  - `self.__size_`
- Rehydrate matching placeholder `pub struct __tree_*` definitions from `_opaque` to:
  - `__begin_node_: *mut u8`
  - `__end_node_: *mut __tree_end_node`
  - `__size_: u64`
- Rehydrate corresponding `Default` impl lanes to null/zero values.
- Rewrite degraded `pub fn size(&self) -> usize { 0 }` stubs in those tree impls to
  `self.__size_ as usize`.

### 2) `std_unordered_set_*` compat surface completion

In the same pass, detect generated unordered-set placeholder structs and append only missing methods:

- `begin(&self) -> *mut std::ffi::c_void`
- `end(&self) -> *mut std::ffi::c_void`
- `find<T>(&self, _key: T) -> *mut std::ffi::c_void`
- `insert<T>(&mut self, _value: T)`

Compatibility injection is impl-block-aware and idempotent (no duplicate methods on rerun).

## Regression coverage executed

Command:

```bash
cargo test -p fragile-clang --lib normalize_rpc_container_internal_node_artifacts -- --nocapture
```

Covered assertions:

- `test_normalize_rpc_container_internal_node_artifacts_rehydrates_tree_internal_node_lanes`
- `test_normalize_rpc_container_internal_node_artifacts_adds_unordered_set_missing_methods`
- `test_normalize_rpc_container_internal_node_artifacts_is_idempotent_for_unordered_set_impls`

## Residual scope

This closes `M9.2.c.iv.e.34.f.5.c`.

Remaining final-closure leaves:

- `M9.2.c.iv.e.34.f.5.d`
- `M9.2.c.iv.e.34.f.5.e`
