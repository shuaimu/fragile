# M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.c.1 reactor/quorum symbol-signature inventory

Date: 2026-03-29
Task leaf: `M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.c.1`

## Scope

Execute a bounded first slice of `c.4.c` by fixing high-signal reactor/quorum
symbol/signature drifts surfaced after `c.4.b`:

- unresolved `sp_running_coro_th_` lane in `Fiber::current_fiber`
- unqualified `get_reactor()` call in poll loop
- missing `this_thread::get_id()` helper in generated module
- invalid placeholder signature lane `func: &mut _` (E0121)
- `remove_xid(fd)` key-name drift in quorum lane
- raw-deref iterator compare lanes in reactor poll path (`E0133`)
- mutability drift `remove_fds.swap(&self.pending_remove_)` (`E0308`)

## Wrong-approach check

- Reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Reviewed `docs/dev/wrong.md`.
- No target-file conditionals in drivers, no force-native bypass, no fake runtime
  shim binaries; fixes are generic late normalization rewrites in `ast_codegen`.

## Implementation

File changed:

- `crates/fragile-clang/src/ast_codegen.rs`

Added `normalize_rpc_reactor_symbol_and_signature_artifacts` and wired it into
pipeline tail after `normalize_rpc_event_surface_artifacts`.

Normalization behavior:

- rewrites `sp_running_coro_th_.borrow()` to static-member lane
  `unsafe { (REACTOR_SP_RUNNING_CORO_TH_).borrow() }`
- rewrites `(*get_reactor().op_arrow())` to `(*Reactor::get_reactor().op_arrow())`
- injects `this_thread::get_id` when referenced but missing
- rewrites signature placeholder `func: &mut _` to `func: &mut std::ffi::c_void`
- fixes quorum `remove_xid` drift `self.xids_.erase(fd);` -> `self.xids_.erase(site);`
- rewrites raw-deref compare lanes to pointer-iterator direct compare
- rewrites `remove_fds.swap(&self.pending_remove_)` to mutable argument lane

## Focused tests

Added tests:

- `test_normalize_rpc_reactor_symbol_and_signature_artifacts_rewrites_core_markers`
- `test_normalize_rpc_reactor_symbol_and_signature_artifacts_get_id_injection_is_idempotent`

Command:

```bash
cargo test -p fragile-clang normalize_rpc_reactor_symbol_and_signature_artifacts -- --nocapture
```

Result: passed (`2 passed; 0 failed`).

## Focused compile probe

A compile-commands-driven strict probe was run from
`/tmp/fragile_m9_2_strict_runtime_replay_20260329T001857Z_p2759862/build_fragilec`.

Probe artifact root:

- `/tmp/fragile_c4c1_focus_20260329T203604`

Evidence:

- `focus_1.status=0` for the extracted `quorum_event.cc` txlog compile command.

Note:

- A full replay and full `reactor.cc` txlog-focused compile probe were started but
  are deferred to `c.4.c.4` to keep this first slice bounded and avoid turning
  `c.4.c.1` into an end-to-end replay leaf.

## Conclusion

`c.4.c.1` is complete as a bounded implementation slice:

- targeted reactor/quorum symbol-signature drifts are normalized in a generic pass,
- focused unit coverage is in place,
- the quorum focused compile probe succeeded.

Next leaves:

- `c.4.c.2` for `rpc/client.cpp` syntax/call-shape drift,
- `c.4.c.3` for client/reactor compatibility surface gaps,
- `c.4.c.4` for strict replay delta capture.
