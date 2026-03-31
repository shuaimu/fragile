# M9.2.c.iv.e.34.f.5.e.5.e.4.c.2 Reactor Command-Map / Container-Rc Surface Inventory

Date: 2026-03-28
Task leaf: `M9.2.c.iv.e.34.f.5.e.5.e.4.c.2`

## Scope
Resolve the dominant `quorum_event.cc` / `reactor.cc` replay cluster from baseline run-root
`/tmp/fragile_m9_2_strict_runtime_replay_20260328T160304Z_p2263426`:

- `E0599` missing container/Rc surfaces
- `E0277` degraded default/trait lanes
- `E0308` command/event callshape mismatches

without target-specific conditionals.

## Design and Implementation Notes
All edits were bounded to generic late normalizers in `crates/fragile-clang/src/ast_codegen.rs`.

Implemented surfaces and rewrites:

1. Container method/surface completion
- Extended `normalize_rpc_container_internal_node_artifacts` for:
  - `set` / `unordered_set`: `insert`, `erase`, `clear`, `swap`
  - `unordered_map`: missing method lanes and idempotent method detection
  - `IntoIterator` compat for unordered-set style wrappers

2. Reactor/quorum command-map callshape cleanup
- Extended `normalize_rpc_event_surface_artifacts` for:
  - `Fiber::create_run_impl` callshape rehydration
  - `JoinHandle<_>` default-return artifacts -> explicit concrete return lanes
  - `Result<(), TrySendError<()>> = Default::default()` -> `Ok(())`
  - xids/poll/remove/close residual callshape drift rewrites
  - `rrr::Cmd*` Debug compat + `rrr::Fiber` Ord/Eq/PartialOrd/PartialEq compat

3. Borrow/deref compat completion for Rc/Arc/RefCell/MaybeUninit lanes
- Extended `normalize_final_rpc_straggler_artifacts` with missing compat surfaces used by residual reactor fragments.

## Focused Regression Coverage
Added/updated focused unit tests in `ast_codegen.rs`:

- `test_normalize_rpc_container_internal_node_artifacts_adds_unordered_set_extended_methods_and_into_iter`
- `test_normalize_rpc_container_internal_node_artifacts_adds_std_set_insert_erase_methods`
- `test_normalize_rpc_event_surface_artifacts_rewrites_reactor_pollthread_callshape_artifacts`
- `test_normalize_rpc_event_surface_artifacts_adds_reactor_command_debug_and_fiber_ord_compat`
- `test_normalize_final_rpc_straggler_artifacts_adds_reactor_pointer_and_borrow_compat_surfaces`

All focused tests passed locally.

## Strict Replay Evidence
Replay command:

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --skip-fragilec-build \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260328T160304Z_p2263426
```

Run-root:
- `/tmp/fragile_m9_2_strict_runtime_replay_20260328T162346Z_p2277812`

Manifest delta (from `strict_runtime_replay_blocker_inventory_manifest.txt`):
- baseline: `rustc_error_total_count=38`, `rustc_error_unique_count=24`
- after c.2: `rustc_error_total_count=12`, `rustc_error_unique_count=12`
- `non_increase_verdict=true`

Outcome shift:
- prior c.2 typed cluster (`E0599`/`E0277`/`E0308` in `quorum_event.cc`/`reactor.cc`) no longer appears in `lane_fragilec/build.stderr`
- residual lane failure is now unresolved-type invariant stops in:
  - `event.cc`
  - `fiber_context_runtime.cc`
  - `fiber_impl.cc`
  - `quorum_event.cc`

## Wrong-Approach Check
Conforms to `docs/dev/wrong.md`:

- no target-specific (`mako`-only) branch logic
- no force-native bypass
- no fake semantic bodies added solely to force pass
- all changes are generic normalizer/compat surfaces with focused regression tests
