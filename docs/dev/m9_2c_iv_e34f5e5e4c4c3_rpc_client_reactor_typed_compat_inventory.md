# M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.c.3 - RPC Client/Reactor Typed Compat Inventory

## Scope
Bounded closure for the c.4.c.3 signature cluster in `rpc/client.cpp` focused compile replay:
- missing `std_list_QueuedRequest` methods,
- `ReconnectPolicy` pointer-field access lane (`self.policy_.field`),
- `SpinMutexGuard` / `MutexGuard` / `Ref` default-lane drift,
- missing `op_bool`/`op_arrow` compat lanes.

## Implementation
Code changes in `crates/fragile-clang/src/ast_codegen.rs`:
- Added late pass `normalize_rpc_client_reactor_typed_compat_surfaces`.
- Wired the pass immediately after `normalize_rpc_client_syntax_and_enumbool_callshape_artifacts`.
- Added focused unit tests:
  - `test_normalize_rpc_client_reactor_typed_compat_surfaces_rewrites_pointer_and_guard_lanes`
  - `test_normalize_rpc_client_reactor_typed_compat_surfaces_injects_method_compat_surfaces_idempotently`

### Normalization Actions
1. `ReconnectPolicy` pointer-field lanes:
- `self.policy_.<field>` -> `(*self.policy_).<field>` for all residual fields.

2. Guard/ref degraded defaults:
- `rusty_MutexGuard_rrr_Future_State = Default::default()` -> `self.state_.lock().unwrap()`.
- `wait_while(Default::default(), ...)` -> `wait_while(guard, ...)`.
- `std::cell::Ref/RefMut<Option<Arc<ClientConnection>>> = Default::default()` -> `self.connection_.borrow()/borrow_mut()`.
- `rrr_SpinMutexGuard_* = Default::default()` -> bounded `lock().unwrap()` lanes.

3. Degraded chained method lanes:
- `state_machine_.is_null().can_connect()` -> `state_machine_.can_connect()`.
- `state_machine_.is_null().is_connected()` -> `state_machine_.is_connected()`.
- `pending_queue_.is_null().empty()` -> `pending_queue_.empty()`.

4. Compat surface insertion (idempotent):
- `std_list_QueuedRequest::{empty,pop_front,front,begin,end,erase,clear}` when missing.
- `std_function_void__*::op_bool` for targeted callback wrappers when missing.
- `FragileArcBoolCompat` trait for `Arc<T>::op_bool` lanes.
- `FragileMutexGuardArrowCompat` trait for `MutexGuard::op_arrow` lanes.
- `rrr_SpinMutexGuard_*::op_arrow` inferred from `data_: *mut UnsafeCell<T>`.

## Focused Replay Evidence
- Baseline run-root (c.4.c.2): `/tmp/fragile_c4c2_focus_20260329T005901Z`
- Post-c.4.c.3 run-root: `/tmp/fragile_c4c3_focus_20260329T021420Z`
- Status: `focus_1.status=1` (expected; leaf closes bounded cluster, not full compile green).

### Signature Counts (baseline -> post)
- `no method named ... std_list_QueuedRequest`: `9 -> 0`
- `no field ... *const rrr::ReconnectPolicy`: `17 -> 0`
- `no method named op_bool ... std_function_void__*`: `8 -> 0`
- `no method named op_arrow ... rrr_SpinMutexGuard_*`: `9 -> 0`
- `no method named op_arrow ... std::sync::MutexGuard`: `3 -> 0`
- `no method named op_bool ... Arc<rrr::Future>`: `1 -> 0`
- Total rustc errors (`error[E*]` lines): `190 -> 117`

## Residuals (Shifted Beyond c.4.c.3)
Post-pass leading blockers are no longer in the c.4.c.3 signature family and now center on:
- unresolved iterator pointer lane (`*mut c_void` `op_arrow` in queue-iterator path),
- `rrr_RequestOptions::can_retry` lane,
- broader `E0425`/`E0308`/`E0061` families.

These are follow-up inputs for `c.4.c.4` replay delta closure.

## Wrong-Approach Check
Reviewed against `docs/fragile-dev-book.md` section 1.3 and `docs/dev/wrong.md`:
- no target-specific branching,
- no force-native bypass,
- no deletion rollback pattern,
- bounded late normalization + idempotent compat insertion only.
