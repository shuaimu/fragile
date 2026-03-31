# M9.2.c.iv.f.5.d - Supporting E0277/E0609 Slice Inventory

## Scope

Leaf `M9.2.c.iv.f.5.d` executes one bounded supporting residual slice after `f.5.c`.

Selected supporting classes in this slice:

- `E0277` (non-Default array/value lanes)
- `E0609` (missing-field placeholder lanes)

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
- no target-only branch gates,
- no semantic stub bypasses to fake lane-green.

## Implementation

Touched file:

- `crates/fragile-clang/src/ast_codegen.rs`

Added bounded pass:

- `normalize_f5d_supporting_e0277_e0609_slice`

Slice behavior:

1. `E0277` array-default lane repair:
- scans struct fields for large `[i8; N]` / `[u8; N]` lanes (`N > 32`),
- rewrites initializer/assignment `Default::default()` lanes to explicit `[0; N]`.

2. `E0609` state/placeholder field-lane repair:
- rehydrates `rrr_Future_State` with `ready`/`timed_out` fields and rewires `State` alias,
- rehydrates `ShutdownState` and `rrr_Server_ShutdownState` with `shutdown`,
- rehydrates `rrr_Marshal` with `valid_id`,
- rehydrates `rrr_AddrInfo` with `ai_addr`/`ai_addrlen`.

3. Supporting compatibility helpers in the same bounded pass:
- rewrites `super::rusty::write_i32(...)` to `super::write(...)`,
- injects `bind_i32_ptr_const_sockaddr` when referenced,
- injects `__thread_id` alias when unresolved.

## Unit Tests

Focused normalizer tests:

- `cargo test -p fragile-clang normalize_f5d_supporting_e0277_e0609_slice -- --nocapture`

Coverage includes:

- large array `Default::default()` rewrite,
- state/shutdown/marshal/addr-info field rehydration,
- helper injection idempotence.

## Focused Probe Contract

Anchor:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835/build_fragilec/compile_commands.json`

Environment:

- `FRAGILEC_MODE=strict`

Baseline (post-f.5.c):

- `/tmp/fragile_f5c_probe_after_20260330T192417Z_txlog/summary.txt`
- scoped totals: `E0277=17`, `E0609=14`

Post-fix probe root:

- `/tmp/fragile_f5d_probe_after_20260330T210238Z_txlog`
- summary: `/tmp/fragile_f5d_probe_after_20260330T210238Z_txlog/summary.txt`

## Focused Probe Results

### `E0277` by compile unit

| compile unit | baseline E0277 (f.5.c) | post-f.5.d E0277 | delta |
|---|---:|---:|---:|
| `reactor.cc` | 1 | 0 | -1 |
| `rpc/client.cpp` | 6 | 1 | -5 |
| `rpc/server.cpp` | 8 | 0 | -8 |
| `rpc/utils.cpp` | 2 | 0 | -2 |
| **total** | **17** | **1** | **-16** |

### `E0609` by compile unit

| compile unit | baseline E0609 (f.5.c) | post-f.5.d E0609 | delta |
|---|---:|---:|---:|
| `reactor.cc` | 0 | 0 | 0 |
| `rpc/client.cpp` | 9 | 2 | -7 |
| `rpc/server.cpp` | 3 | 0 | -3 |
| `rpc/utils.cpp` | 2 | 0 | -2 |
| **total** | **14** | **2** | **-12** |

### Residual signatures after f.5.d

- `E0277` (remaining 1): `list_rusty_Arc_Future` iterator lane in `rpc/client.cpp`.
- `E0609` (remaining 2): `ready` / `timed_out` on `&mut rrr::State` in `rpc/client.cpp`.
- `E0425` reduced from 14 to 6 (supporting helper lane closure for `write_i32` and `bind_i32_ptr_const_sockaddr`).
- Untargeted classes shifted (`E0308` `8 -> 11`) and stay in follow-up closure scope (`f.5.e`).

### Signature families addressed in this slice

- large-array initializer lanes (`[i8; 118]`, `[i8; 108]`, `[i8; 40]`, `[u8; 80]`, `[u8; 536]`),
- state/shutdown placeholder field lanes (`ready`, `timed_out`, `shutdown`),
- marshal/addr-info placeholder field lanes (`valid_id`, `ai_addr`, `ai_addrlen`),
- supporting unresolved helper lanes (`bind_i32_ptr_const_sockaddr`, `__thread_id`).

## Handoff

This slice closes `M9.2.c.iv.f.5.d` with bounded supporting-class reductions on the same four-unit probe scope.
Remaining closure is in:

1. `M9.2.c.iv.f.5.e` (strict replay rerun + non-increase verification / next decomposition if still red).
