# M9.2.c.iv.f.2.d - E0282/E0605 Inference-Cast Inventory

## Scope
Leaf `M9.2.c.iv.f.2.d` executes the bounded inference/cast slice from `f.2`:
- `E0282` inference cleanup for redundant `Default::default().clone()` assignment/return lanes.
- `E0605` non-primitive cast cleanup for placeholder `status_ as i32` lanes.

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
1. Added bounded pass `normalize_e0282_e0605_inference_cast_slice` and wired it near pipeline tail.
2. Rewrites `= Default::default().clone();` and `return Default::default().clone();` to non-clone forms.
3. Detects placeholder status lanes (`status_` + `__unnamed_at_` + `_opaque`) and rewrites `(self.status_ as i32)` to:
   - `(unsafe { std::mem::transmute_copy::<_, i32>(&self.status_) })`

## Unit Tests
`cargo test -p fragile-clang normalize_e0282_e0605_inference_cast_slice -- --nocapture`
- `test_normalize_e0282_e0605_inference_cast_slice_rewrites_default_clone_assignments`
- `test_normalize_e0282_e0605_inference_cast_slice_rewrites_placeholder_status_casts`
- `test_normalize_e0282_e0605_inference_cast_slice_preserves_non_placeholder_status_casts`
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
  - `/tmp/fragile_f2c_probe_after_tailfix_release_20260330_txlog/summary.txt`
- post-fix with rebuilt release `fragilec`:
  - `/tmp/fragile_f2d_probe_after_20260330T092623Z_txlog/summary.txt`

## Focused Probe Results

### E0282 by Compile Unit
| compile unit | pre-fix E0282 | post-fix E0282 | delta |
|---|---:|---:|---:|
| `reactor.cc` | 1 | 0 | -1 |
| `rpc/client.cpp` | 3 | 1 | -2 |
| `rpc/server.cpp` | 1 | 0 | -1 |
| `rpc/utils.cpp` | 0 | 0 | 0 |
| **total** | **5** | **1** | **-4** |

### E0605 by Compile Unit
| compile unit | pre-fix E0605 | post-fix E0605 | delta |
|---|---:|---:|---:|
| `reactor.cc` | 0 | 0 | 0 |
| `rpc/client.cpp` | 0 | 0 | 0 |
| `rpc/server.cpp` | 4 | 0 | -4 |
| `rpc/utils.cpp` | 0 | 0 | 0 |
| **total** | **4** | **0** | **-4** |

## Residuals
Targeted `E0605` in this scoped slice is eliminated (`4 -> 0`).

Residual `E0282` remains in `rpc/client.cpp` at the `notify_ready(Default::default())` lane tied to non-iterator `list_rusty_Arc_Future` usage (`for fu in futures`), and rolls into `M9.2.c.iv.f.2.e` inventory refresh.
