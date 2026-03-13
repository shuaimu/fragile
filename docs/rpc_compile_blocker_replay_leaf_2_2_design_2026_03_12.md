# RPC Compile Blocker Replay Leaf 2.2 Design (2026-03-12)

## Objective

Implement a deterministic focused replay hook for top-ranked blocker translation units
from leaf `2.1` inventory output.

Leaf `2.2` must:

- pick replay candidates deterministically from inventory artifacts
- capture deterministic replay command materialization
- execute replay commands with bounded timeouts
- persist first-failure artifacts for follow-up fix leaves (`2.3+`)

## Scope Sizing

Estimated implementation size is small and within target:

- replay script: ~430 LOC
- fixture regressions: ~320 LOC
- docs/TODO updates: small

This remains below the “not too many LOC” threshold for a single leaf.

## Scope

Included:

- inventory-driven deterministic candidate ranking:
  - blocker-class priority
  - unresolved-name count (`E0425`) descending tie-break
  - stable lane/file lexical tie-break
- replay command resolution path:
  - prefer `build_<lane>/compile_commands.json` entry matching selected blocker file
  - fall back to deterministic lane compiler from `benchmark_harness_manifest.txt`
- deterministic artifact emission:
  - `rpc_compile_blocker_replay_plan.txt`
  - `rpc_compile_blocker_replay_manifest.txt`
  - per replay `replay_<NN>/...` artifacts (command/status/stdout/stderr/first failure)
- bounded replay execution (`--timeout-seconds`)
- fixture tests for ranking, compile-db path, fallback path, zero-candidate behavior, and required-input failures

Not included:

- blocker-family fix logic (`2.3+`)
- strict build success gate (`2.6`)

## Wrong-Approach Check

Aligned with `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md`:

- no RPC-target-name conditionals in compiler/codegen behavior
- no semantic stub/fake method-body injection
- no force-native bypass path
- replay hook captures true command/output artifacts; it does not synthesize successful outcomes

## Test Execution

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/python/test_mako_rpc_compile_blocker_replay.py -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/python/test_mako_rpc_compile_blocker_inventory.py -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/python/test_mako_rpcbench_harness.py -v`
- full workspace suite:
  - `cargo test`
