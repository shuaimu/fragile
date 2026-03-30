# M9.2.c.iv.f.5.c - E0308 Value-Shape Slice Inventory

## Scope

Leaf `M9.2.c.iv.f.5.c` executes the next bounded dominant `E0308` slice from `f.5`:

- target: residual type-lane/value-shape mismatches in `reactor.cc` + `rpc/*` units,
- bounded edits in codegen late normalizers (no parser/runtime target branching),
- no force-native bypass.

Primary compile-unit scope:

- `reactor.cc`
- `rpc/client.cpp`
- `rpc/server.cpp`
- `rpc/utils.cpp`

## Wrong-Approach Check

Re-reviewed before edits:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrail confirmation:

- no force-native fallback,
- no mako-only branch gates,
- no semantic stubs/suppression-only closure.

## Implementation

Touched file:

- `crates/fragile-clang/src/ast_codegen.rs`

Added bounded pass:

- `normalize_e0308_f5c_value_shape_mismatches`

Slice behavior:

1. mutable RNG callshape rehydration:
   - `.op_call(rd)` / `.op_call(gen)` -> `.op_call(&mut rd)` / `.op_call(&mut gen)` when argument is a known mutable local binding.
2. Harmonize replay-dominant queue/load-balancer value lanes:
   - `u64 <- .size()` / `u64 <- .get()` declaration lanes,
   - queue `size/max_size` comparison/subtraction lanes,
   - round-robin modulo lane `% (pool_size)` to `% (pool_size as usize)`,
   - `Cell<usize>::set(next)` cast lane for `next: u64`.
3. Repair degraded return/reference lanes:
   - `return &unsafe { __fsv_*.assume_init_mut() };` -> `return unsafe { __fsv_*.assume_init_ref() };`.
4. Repair bounded callshape aliases:
   - `.reconnect(0)` -> `.reconnect(Default::default())`,
   - `wait_while(...).unwrap().clone()` -> `wait_while(...).unwrap()`,
   - `&mut ...as_ref().unwrap()` -> `...as_mut().unwrap()`,
   - Option-unwrap Arc alias type lane (`Arc<rrr_T>` to payload `Arc<T>` when deterministically derivable from `Option<Arc<T>>` source).
5. Prevent over-broad size casting:
   - preserved `usize`-typed `s.size()` lanes in string/iterator helper paths (no blanket `.size() as u64` rewrite).

## Unit Tests

Focused normalizer tests:

- `cargo test -p fragile-clang normalize_e0308_f5c_value_shape_mismatches -- --nocapture`

Coverage includes:

- RNG `op_call` mutability + `pow_1`/Cell lane rewrites,
- `assume_init_ref` + reconnect/wait/unwrap callshape repairs,
- Arc alias unwrap type-lane rewrite,
- explicit non-regression guard for `usize` `size()` helper lanes.

## Focused Probe Contract

Anchor:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835/build_fragilec/compile_commands.json`

Environment:

- `FRAGILEC_MODE=strict`

Post-fix focused probe root:

- `/tmp/fragile_f5c_probe_after_20260330T192417Z_txlog`
- summary: `/tmp/fragile_f5c_probe_after_20260330T192417Z_txlog/summary.txt`

Baseline for this leaf:

- `f.5.a` replay manifest totals (`E0308=25` across the same four compile units).

## Focused Probe Results

### E0308 by compile unit

| compile unit | baseline E0308 (f.5.a) | post-f.5.c E0308 | delta |
|---|---:|---:|---:|
| `reactor.cc` | 1 | 1 | 0 |
| `rpc/client.cpp` | 19 | 2 | -17 |
| `rpc/server.cpp` | 4 | 4 | 0 |
| `rpc/utils.cpp` | 1 | 1 | 0 |
| **total** | **25** | **8** | **-17** |

### Residual E0308 signatures after f.5.c

- `reactor.cc`: `__thread_id::new_1(poll_tid)` lane (`expected u64, found std___thread_id`).
- `rpc/client.cpp`: 
  - degraded template select return lane (function body returns `()` in one branch),
  - `pow_1(2.0, attempt)` second-arg lane (`expected f64, found u16`).
- `rpc/server.cpp`:
  - `&rrr_Request` vs `&Request` reply argument lane,
  - `pending_rpc_to_service_.op_index(rpc_id) = svc_index` (`expected i32, found u64`),
  - `MutexGuard<rrr_Server_ShutdownState>` vs `MutexGuard<ShutdownState>`,
  - `Arc<rrr_ServerListener>` vs `Arc<ServerListener>`.
- `rpc/utils.cpp`: `rrr_AddrInfo` vs `AddrInfo` unwrap lane.

## Handoff

This slice closes `M9.2.c.iv.f.5.c` with bounded `E0308` reduction evidence.
Remaining typed clusters continue under:

1. `M9.2.c.iv.f.5.d` (supporting residual classes),
2. `M9.2.c.iv.f.5.e` (end-to-end strict replay + non-increase verification).
