# M9.2.c.iv.f.5.a Post-f.4 Residual Typed-Error Bucket Manifest

## Scope

Leaf `M9.2.c.iv.f.5.a` captures deterministic post-f.4 typed-error buckets in the contract shape:

- `error code -> compile unit -> count -> exemplar signature`

Compile units in scope:

- `reactor.cc`
- `rpc/client.cpp`
- `rpc/server.cpp`
- `rpc/utils.cpp`

Replay root anchor:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835`

Input file:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835/lane_fragilec/build.stderr`

## Wrong-Approach Check

Re-reviewed before extraction:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No shortcut policy violations:

- no force-native bypass,
- no target-specific branching,
- no semantic stubs,
- no suppression-only accounting.

## Deterministic Extraction Contract

Extraction classifies compile-unit sections from lines shaped as:

- `[fragilec] fragile rustc object compile failed for ... (parser-output-handoff)`

Then counts `error[E....]` codes within each active section and records first-seen exemplar signatures.

Determinism check:

- pass1: `/tmp/fragile_f5a_postf4_manifest_pass1.tsv`
- pass2: `/tmp/fragile_f5a_postf4_manifest_pass2.tsv`
- `DIFF_STATUS=identical`
- row counts: `32` and `32`

## Manifest (code -> compile-unit -> count -> exemplar)

| compile_unit | error_code | count | exemplar signature |
|---|---:|---:|---|
| `reactor.cc` | `E0277` | 1 | error[E0277]: the trait bound `Result<(), TrySendError<()>>: Default` is not satisfied |
| `reactor.cc` | `E0308` | 1 | error[E0308]: mismatched types |
| `reactor.cc` | `E0425` | 1 | error[E0425]: cannot find type `__thread_id` in this scope |
| `reactor.cc` | `E0596` | 1 | error[E0596]: cannot borrow data in a `*const` pointer as mutable |
| `rpc/client.cpp` | `E0308` | 19 | error[E0308]: mismatched types |
| `rpc/client.cpp` | `E0599` | 15 | error[E0599]: no method named `op_sub` found for type `i64` in the current scope |
| `rpc/client.cpp` | `E0277` | 9 | error[E0277]: the trait bound `[i8; 118]: Default` is not satisfied |
| `rpc/client.cpp` | `E0609` | 9 | error[E0609]: no field `ready` on type `rrr::State` |
| `rpc/client.cpp` | `E0425` | 7 | error[E0425]: cannot find function `write_i32` in module `super::rusty` |
| `rpc/client.cpp` | `E0596` | 6 | error[E0596]: cannot borrow `self.pending_queue_` as mutable, as it is behind a `&` reference |
| `rpc/client.cpp` | `E0369` | 2 | error[E0369]: binary operation `!=` cannot be applied to type `std___wrap_iter_rusty_Arc_rrr_Client` |
| `rpc/client.cpp` | `E0594` | 2 | error[E0594]: cannot assign to `self.on_server_restart_`, which is behind a `&` reference |
| `rpc/client.cpp` | `E0606` | 2 | error[E0606]: casting `&u32` as `f64` is invalid |
| `rpc/client.cpp` | `E0618` | 2 | error[E0618]: expected function, found `i64` |
| `rpc/client.cpp` | `E0133` | 1 | error[E0133]: dereference of raw pointer is unsafe and requires unsafe function or block |
| `rpc/client.cpp` | `E0282` | 1 | error[E0282]: type annotations needed |
| `rpc/client.cpp` | `E0507` | 1 | error[E0507]: cannot move out of `self.host_` which is behind a shared reference |
| `rpc/client.cpp` | `E0603` | 1 | error[E0603]: struct import `Arc` is private |
| `rpc/server.cpp` | `E0599` | 12 | error[E0599]: no method named `write_to_fd` found for struct `rrr_Marshal` in the current scope |
| `rpc/server.cpp` | `E0277` | 8 | error[E0277]: the trait bound `[i8; 108]: Default` is not satisfied |
| `rpc/server.cpp` | `E0425` | 5 | error[E0425]: cannot find function `write_i32` in module `super::rusty` |
| `rpc/server.cpp` | `E0308` | 4 | error[E0308]: mismatched types |
| `rpc/server.cpp` | `E0609` | 3 | error[E0609]: no field `shutdown` on type `rrr_Server_ShutdownState` |
| `rpc/server.cpp` | `E0283` | 1 | error[E0283]: type annotations needed |
| `rpc/server.cpp` | `E0433` | 1 | error[E0433]: failed to resolve: use of unresolved module or unlinked crate `_unnamedenumat_home_shuai_workspace_fragile_vendor_mako_src_rrr_rpc_server_hpp_251_5_` |
| `rpc/server.cpp` | `E0507` | 1 | error[E0507]: cannot move out of a raw pointer |
| `rpc/utils.cpp` | `E0133` | 3 | error[E0133]: call to unsafe function `fcntl_1` is unsafe and requires unsafe function or block |
| `rpc/utils.cpp` | `E0186` | 3 | error[E0186]: method `__on_zero_shared` has a `&mut self` declaration in the trait, but not in the impl |
| `rpc/utils.cpp` | `E0277` | 2 | error[E0277]: the trait bound `[i8; 118]: Default` is not satisfied |
| `rpc/utils.cpp` | `E0609` | 2 | error[E0609]: no field `ai_addr` on type `rrr_AddrInfo` |
| `rpc/utils.cpp` | `E0308` | 1 | error[E0308]: mismatched types |
| `rpc/utils.cpp` | `E0425` | 1 | error[E0425]: cannot find function `bind_i32_ptr_const_sockaddr` in module `super` |

## Dominant Totals (Scoped to the Four Compile Units)

- `E0599 = 27`
- `E0308 = 25`
- `E0277 = 20`
- `E0425 = 14`
- `E0609 = 14`

These totals align with the post-f.4 replay inventory and provide deterministic input for `M9.2.c.iv.f.5.b` and follow-on f.5 slices.

## Handoff

This manifest is the bounded execution input for:

1. `M9.2.c.iv.f.5.b` (`E0599` compatibility-surface slice)
2. `M9.2.c.iv.f.5.c` (`E0308` value-shape slice)
3. `M9.2.c.iv.f.5.d` (`E0277/E0425/E0609` support slice)
