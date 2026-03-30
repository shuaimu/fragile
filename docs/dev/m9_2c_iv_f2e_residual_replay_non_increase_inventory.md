# M9.2.c.iv.f.2.e - Residual Probe + Replay Non-Increase Inventory

## Scope
Leaf `M9.2.c.iv.f.2.e` closes the residual-refresh contract from `f.2`:
- re-run scoped residual compile probes after `f.2.d`,
- re-run strict runtime replay with `f.1` baseline comparison,
- record deterministic non-increase evidence and next dominating bucket.

Bound from decomposition:
- probe + replay + inventory/docs/tests only; no new parser/codegen fix slice.

## Wrong-Approach Check
Re-reviewed before replay/inventory updates:
- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrail confirmation:
- no force-native bypass,
- no target-specific hacks,
- no semantic stubs/fake runtime behavior,
- no suppression-only accounting.

## Scoped Residual Compile-Probe Refresh
Compile-command source:
- `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053/build_fragilec/compile_commands.json`

Scoped compile units:
- `reactor.cc`
- `rpc/client.cpp`
- `rpc/server.cpp`
- `rpc/utils.cpp`

Environment:
- `FRAGILEC_MODE=strict`

Artifacts:
- pre-rerun baseline:
  - `/tmp/fragile_f2d_probe_after_20260330T092623Z_txlog/summary.txt`
- post-rerun:
  - `/tmp/fragile_f2e_probe_after_20260330T104700Z_txlog/summary.txt`

### Targeted Residual Buckets (`E0282`, `E0605`)
| compile unit | f.2.d E0282 | f.2.e E0282 | delta | f.2.d E0605 | f.2.e E0605 | delta |
|---|---:|---:|---:|---:|---:|---:|
| `reactor.cc` | 0 | 0 | 0 | 0 | 0 | 0 |
| `rpc/client.cpp` | 1 | 1 | 0 | 0 | 0 | 0 |
| `rpc/server.cpp` | 0 | 0 | 0 | 0 | 0 | 0 |
| `rpc/utils.cpp` | 0 | 0 | 0 | 0 | 0 | 0 |
| **total** | **1** | **1** | **0** | **0** | **0** | **0** |

### Dominant Typed Buckets (stability check)
| bucket | f.2.d total | f.2.e total | delta |
|---|---:|---:|---:|
| `E0308` | 29 | 29 | 0 |
| `E0599` | 27 | 27 | 0 |

Notes:
- `total_error_lines` in scoped stderr changed (`13 -> 145`) while targeted typed buckets stayed stable; this leaf treats typed bucket counts and replay manifest deltas as the deterministic gate.

## Strict Runtime Replay Baseline Comparison
Replay command:
- `python3 scripts/mako_rpc_strict_runtime_replay.py --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053 --trials 1 --skip-masstree-perf-target --skip-clean-step`

Artifacts:
- replay run root:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T110921Z_p518218`
- replay manifest:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T110921Z_p518218/strict_runtime_replay_manifest.txt`
- blocker inventory manifest:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T110921Z_p518218/strict_runtime_replay_blocker_inventory_manifest.txt`
- baseline run root:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053`

### Lane Contract Snapshot
- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0`

Lane remains red (expected in this leaf), but deterministic blocker non-increase is preserved.

### Baseline Non-Increase Verdict
| metric | baseline (f.1) | f.2.e replay | verdict |
|---|---:|---:|---|
| `rustc_error_total_count` | 218 | 157 | non-increase (`157 <= 218`) |
| `rustc_error_unique_count` | 89 | 85 | non-increase (`85 <= 89`) |
| `non_increase_verdict` | n/a | true | pass |

## Next Dominating Residual Bucket
From blocker manifest key inventory:
- `E0308:mismatched types` (`error_key_018_count=29`) is the next dominant typed residual bucket.

This bucket is selected as the first fix target for the next bounded residual-fix leaf (`M9.2.c.iv.f.3`).
