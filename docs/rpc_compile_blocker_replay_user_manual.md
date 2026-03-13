# RPC Compile Blocker Replay User Manual (Leaf 2.2)

## Purpose

`mako_rpc_compile_blocker_replay.py` replays top-ranked blocker translation units from
leaf `2.1` inventory artifacts and records deterministic first-failure evidence.

This is a focused triage hook for follow-up generic fix leaves (`2.3+`).

## Script

- Path: `scripts/mako_rpc_compile_blocker_replay.py`

## Required Inputs

Under `run_root`:

- `rpc_compile_blocker_inventory_manifest.txt` (from leaf `2.1`)

Optional but recommended:

- `benchmark_harness_manifest.txt` (compiler/workspace metadata)
- `build_<lane>/compile_commands.json` (exact compile command replay source)

## Example

```bash
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_mako_rpcbench_leaf_1_5 \
  --max-replays 1 \
  --timeout-seconds 300
```

Lane filter example:

```bash
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_mako_rpcbench_leaf_1_5 \
  --lanes fragilec \
  --max-replays 2
```

The script prints the resolved `run_root` on success.

## Ranking and Selection

Candidates are extracted from inventory lanes whose blocker class/file represent a real
replay candidate (not `none` or `build_not_executed`).

Deterministic sort key:

1. blocker-class priority
2. unresolved-name count (`E0425`) descending
3. lane name
4. blocker file path

Top `N` are selected by `--max-replays`.

## Command Source Resolution

For each selected candidate:

- first try matching `build_<lane>/compile_commands.json` by translation-unit file
- if unavailable, use fallback:
  - lane compiler from `benchmark_harness_manifest.txt` (`clang_cxx` / `fragile_cxx`)
  - command shape: `<compiler> -std=gnu++17 -c <file> -o <replay object>`

`command_source` is recorded in replay artifacts/manifest.

## Output Artifacts

Run root:

- `rpc_compile_blocker_replay_plan.txt`
- `rpc_compile_blocker_replay_manifest.txt`

Per replay (`replay_<NN>/`):

- `lane.txt`
- `blocker_class.txt`
- `blocker_file.txt`
- `command_source.txt`
- `command_directory.txt`
- `command.txt`
- `replay.status`
- `replay.stdout`
- `replay.stderr`
- `first_failure_class.txt`
- `first_failure_excerpt.txt`

## Regression Gate

```bash
PYTHONDONTWRITEBYTECODE=1 \
python3 -m unittest tests/python/test_mako_rpc_compile_blocker_replay.py -v
```
