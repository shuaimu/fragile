# M9.2.c.iv.e.34.f.5.e.5.d Reactor Command-Map/Event-Base Inventory

## Scope

Task `M9.2.c.iv.e.34.f.5.e.5.d` closes the bounded `quorum_event.cc` / `reactor.cc` straggler family from post-`e.5.e.5.c` replay evidence:

- `rrr_Cmd*` symbol gaps in generated command-variant lanes
- `Fiber::create_run__` callshape drift
- unordered-map `find/end/erase` surface gaps
- `IntEvent` direct-field drift (`__debug_creator`, `__vtable`, `status_`) that must resolve via `__base` lanes

## Wrong-approach check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Changes are bounded generic normalizations only: no target-file conditionals, no native bypass, no rollback deletion.

## Implementation summary

1. Extended `normalize_rpc_container_internal_node_artifacts`:
   - detect `std_unordered_map_*` / `unordered_map_*` impls and append missing compat methods with method-aware idempotence guards:
     - `end(&self) -> *mut std::ffi::c_void`
     - `find<T>(&self, _key: T) -> *mut std::ffi::c_void`
     - `erase<T>(&mut self, _value: T)`
     - `insert_or_assign<K, V>(&mut self, _key: K, _value: V)`

2. Extended `normalize_rpc_event_surface_artifacts`:
   - normalize `Fiber::create_run__` -> `Fiber::create_run_impl`
   - strip degraded closure-local deref fragments (`*__begin2.op_deref();`, `*_xbegin.op_deref();`) without dropping the surrounding callshape
   - rewrite `IntEvent` direct-field lanes to base lanes:
     - `(*...finalize_event_.op_arrow()).__debug_creator` -> `.__base.__debug_creator`
     - `(*__fragile_base).__vtable` -> `(*__fragile_base).__base.__vtable`
     - `((*...op_arrow()).status_)` -> `((*...op_arrow()).__base.status_)`
   - normalize `unsafe { *self.xids_.op_index(site) = xid };` -> `self.xids_.insert_or_assign(site, xid);`
   - append guarded alias bridge when unresolved `rrr_Cmd*` names are present:
     - `pub type rrr_CmdAddPollable = rrr::CmdAddPollable;`
     - … through `rrr_CmdShutdown`

## Focused validation

Commands:

- `cargo test -p fragile-clang test_normalize_rpc_container_internal_node_artifacts_adds_unordered_map_missing_methods -- --nocapture`
- `cargo test -p fragile-clang test_normalize_rpc_container_internal_node_artifacts_is_idempotent_for_unordered_map_impls -- --nocapture`
- `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_quorum_event_command_map_and_event_base_lanes -- --nocapture`

Result: pass.

New/updated focused tests verify:

- unordered-map compat injection + idempotence in `normalize_rpc_container_internal_node_artifacts`
- `Fiber::create_run__` normalization to `create_run_impl`
- `__begin2` deref cleanup
- `IntEvent` base-field rehydration (`__base.__vtable`, `__base.status_`, `__base.__debug_creator`)
- `xids_.insert_or_assign` rewrite
- `rrr_CmdAddPollable`/`rrr_CmdShutdown` alias bridge emission

## Remaining work after d

Leaf `M9.2.c.iv.e.34.f.5.e.5.d` is closed.

Follow-on leaf:

- `M9.2.c.iv.e.34.f.5.e.5.e` (strict replay rerun and lane-contract closure evidence)
