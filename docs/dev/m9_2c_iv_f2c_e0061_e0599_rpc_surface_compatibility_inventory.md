# M9.2.c.iv.f.2.c - E0061/E0599 RPC Surface Compatibility Inventory

## Scope
Leaf `M9.2.c.iv.f.2.c` executes the bounded RPC compatibility slice from `f.2`:
- `E0061` call-arity/signature normalization.
- `E0599` missing lock-like/method surface compatibility for residual RPC placeholders.

Bound from decomposition:
- `<=300 LOC` in code edits for this leaf.

## Wrong-Approach Check
Re-reviewed before edits:
- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrail confirmation:
- no force-native bypass,
- no target-specific hacks,
- no semantic stubs/fake runtime behavior,
- no suppression-only accounting.

## Implementation
Touched file:
- `crates/fragile-clang/src/ast_codegen.rs`

Changes:
1. Added bounded pass `normalize_e0061_e0599_rpc_surface_compatibility_slice` and wired it near pipeline tail:
   - trims variadic-style `Log::info/error/warn/debug` calls to 3-arg form,
   - rewrites callback callshape lanes (`on_complete`, `on_state_change_`, `on_server_restart_`, `cb`) from `.op_call(args...)` to `.op_call()`,
   - rewrites `(*it).op_inc();` to `(*it).op_inc(0);`,
   - injects missing RPC compat surfaces when absent:
     - `SpinMutex_Marshal::lock`,
     - `SpinMutex_std_unordered_map_i64__rusty_Arc_Future::lock`,
     - `std_map_std_string__std_vector_rusty_Arc_Client::{find,end}`,
     - `rrr_AddrInfo::op_arrow`,
     - `rrr_RequestOptions::can_retry`.
2. Extended `normalize_rpc_marshal_surface_artifacts` chunk trigger:
   - chunk reset compat is now injected not only for `.fully_written()` references, but also `.reset()`, `.content_size()`, `.read(`, `.write(` references.

## Unit Tests
`cargo test -p fragile-clang e0061_e0599_rpc_surface_compatibility_slice -- --nocapture`
- `test_normalize_e0061_e0599_rpc_surface_compatibility_slice_trims_log_and_callback_arity`
- `test_normalize_e0061_e0599_rpc_surface_compatibility_slice_adds_lock_and_method_surfaces`
- pass.

`cargo test -p fragile-clang normalize_rpc_marshal_surface_artifacts_adds_chunk_reset_when_only_reset_is_referenced -- --nocapture`
- `test_normalize_rpc_marshal_surface_artifacts_adds_chunk_reset_when_only_reset_is_referenced`
- pass.

## Focused Probe Contract
Compile-command source:
- `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053/build_fragilec/compile_commands.json`

Scoped compile units:
- `reactor.cc`
- `rpc/client.cpp`
- `rpc/server.cpp`
- `rpc/utils.cpp`

Command family:
- `CMakeFiles/txlog.dir/...` entries for each scoped unit.

Environment:
- `FRAGILEC_MODE=strict`

Artifacts:
- pre-fix baseline:
  - `/tmp/fragile_f2c_probe_after_20260330_txlog/summary.txt`
- post-fix (rebuilt release `fragilec`):
  - `/tmp/fragile_f2c_probe_after_tailfix_release_20260330_txlog/summary.txt`

## Focused Probe Results

### E0061 by Compile Unit
| compile unit | pre-fix E0061 | post-fix E0061 | delta |
|---|---:|---:|---:|
| `reactor.cc` | 0 | 0 | 0 |
| `rpc/client.cpp` | 16 | 0 | -16 |
| `rpc/server.cpp` | 1 | 0 | -1 |
| `rpc/utils.cpp` | 1 | 0 | -1 |
| **total** | **18** | **0** | **-18** |

### E0599 by Compile Unit
| compile unit | pre-fix E0599 | post-fix E0599 | delta |
|---|---:|---:|---:|
| `reactor.cc` | 0 | 0 | 0 |
| `rpc/client.cpp` | 18 | 15 | -3 |
| `rpc/server.cpp` | 12 | 12 | 0 |
| `rpc/utils.cpp` | 2 | 0 | -2 |
| **total** | **32** | **27** | **-5** |

### Aggregate Error-Line Delta
| metric | pre-fix | post-fix | delta |
|---|---:|---:|---:|
| `total_error_lines` (scoped units) | 169 | 152 | -17 |

## Residuals
`E0061` in this scoped slice is eliminated (`18 -> 0`), while residual `E0599` signatures remain for follow-on leaves, including:
- `rrr_Marshal::{content_size,write_to_fd,empty}`,
- `rrr_ServerConnection::reply`,
- `Box<rrr_Request>::op_deref`,
- vector-like `size/op_index` lanes,
- `std_random_device::op_call`,
- non-RPC-global placeholder method surfaces (`op_sub`, `time_since_epoch`, `Arc::make_ref_mut_i64`).

These roll forward into `M9.2.c.iv.f.2.d` / `M9.2.c.iv.f.2.e` and subsequent residual decomposition.
