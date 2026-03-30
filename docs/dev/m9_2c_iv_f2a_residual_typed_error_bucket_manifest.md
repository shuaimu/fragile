# M9.2.c.iv.f.2.a Residual Typed-Error Bucket Manifest

## Scope
Leaf `M9.2.c.iv.f.2.a` captures a deterministic typed-error bucket manifest in the shape:
- `error_key -> compile_unit -> count -> exemplar signature`

Compile units in scope:
- `reactor.cc`
- `rpc/client.cpp`
- `rpc/server.cpp`
- `rpc/utils.cpp`

Target buckets from `f.2` decomposition:
- `E0308`
- `E0061`
- `E0599`
- `E0282`
- `E0605`

## Inputs
Replay roots from `M9.2.c.iv.f.1`:
- baseline: `/tmp/fragile_m9_2_strict_runtime_replay_20260329T040328Z_p2989433`
- current: `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053`

Data source file in each run root:
- `lane_fragilec/build.stderr`

## Wrong-Approach Check
Re-reviewed before extraction:
- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No shortcuts used:
- no force-native bypass,
- no target-specific conditionals,
- no semantic stubs,
- no suppression-only accounting.

## Deterministic Extraction Contract
Extraction command classifies the active compile-unit section from:
- `[fragilec] fragile rustc object compile failed for ... (parser-output-handoff)`

Then counts only target buckets (`E0308`, `E0061`, `E0599`, `E0282`, `E0605`) and records first-seen exemplar signatures.

Determinism check outcome:
- `DIFF_STATUS=identical`
- line counts: `14` rows in baseline manifest, `14` rows in current manifest

## Manifest (key -> compile-unit -> count -> exemplar)
| compile_unit | error_key | count | exemplar signature |
|---|---:|---:|---|
| `reactor.cc` | `E0282` | 1 | `error[E0282]: type annotations needed` |
| `reactor.cc` | `E0308` | 5 | `error[E0308]: mismatched types` |
| `rpc/client.cpp` | `E0061` | 16 | `error[E0061]: this method takes 0 arguments but 2 arguments were supplied` |
| `rpc/client.cpp` | `E0282` | 3 | `error[E0282]: type annotations needed` |
| `rpc/client.cpp` | `E0308` | 43 | `error[E0308]: mismatched types` |
| `rpc/client.cpp` | `E0599` | 18 | error[E0599]: no method named `reset` found for struct `chunk` in the current scope |
| `rpc/server.cpp` | `E0061` | 1 | `error[E0061]: this function takes 3 arguments but 5 arguments were supplied` |
| `rpc/server.cpp` | `E0282` | 1 | `error[E0282]: type annotations needed` |
| `rpc/server.cpp` | `E0308` | 16 | `error[E0308]: mismatched types` |
| `rpc/server.cpp` | `E0599` | 12 | error[E0599]: no method named `reset` found for struct `chunk` in the current scope |
| `rpc/server.cpp` | `E0605` | 4 | error[E0605]: non-primitive cast: `rrr_ServerConnection__unnamed_at__home_shuai_workspace_fragile_vendor_mako_src_rrr_rpc_server_hpp_251_5_` as `i32` |
| `rpc/utils.cpp` | `E0061` | 1 | `error[E0061]: this function takes 3 arguments but 4 arguments were supplied` |
| `rpc/utils.cpp` | `E0308` | 2 | `error[E0308]: mismatched types` |
| `rpc/utils.cpp` | `E0599` | 2 | error[E0599]: no method named `op_arrow` found for struct `rrr_AddrInfo` in the current scope |

## Target-Bucket Totals (Scoped to Four Compile Units)
- `E0308 = 66`
- `E0061 = 18`
- `E0599 = 32`
- `E0282 = 5`
- `E0605 = 4`

This preserves the `f.2` ordering signal for implementation leaves:
1. `E0308`
2. `E0061` / `E0599`
3. `E0282` / `E0605`

## Handoff
This manifest is the bounded input for:
- `M9.2.c.iv.f.2.b` (`E0308` bucket-B1 slice)
- `M9.2.c.iv.f.2.c` (`E0061`/`E0599` compatibility slice)
- `M9.2.c.iv.f.2.d` (`E0282`/`E0605` inference-cast slice)
