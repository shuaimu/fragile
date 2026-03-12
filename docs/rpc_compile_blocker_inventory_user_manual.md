# RPC Compile Blocker Inventory User Manual (Leaf 2.1)

## Purpose

`mako_rpc_compile_blocker_inventory.py` derives deterministic compile-blocker inventory
artifacts from a completed RPC benchmark harness run root.

It is intended to summarize first-failure compile blockers before targeted replay/fixes
in leaves `2.2+`.

## Script

- Path: `scripts/mako_rpc_compile_blocker_inventory.py`

## Input Contract

For each requested lane (`clang`, `fragilec` by default), the run root must contain:

- `lane_<lane>/build.status`
- `lane_<lane>/build.stderr`

These are produced by `scripts/mako_rpcbench_harness.py` execution mode.

## Example

```bash
python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_mako_rpcbench_leaf_1_5 \
  --lanes clang,fragilec
```

The script prints the resolved `run_root` on success.

## Output Artifacts

Per lane (`lane_<lane>/`):

- `first_failing_compile_class.txt`
- `first_failing_compile_file.txt`
- `first_failing_compile_e0425_count.txt`

Run root:

- `rpc_compile_blocker_inventory_manifest.txt`

## Blocker Class Values

- `none`
- `build_not_executed`
- `transpile_failure`
- `unresolved_name_or_type_e0425`
- `missing_method_e0599`
- `arity_mismatch_e0061`
- `type_mismatch_e0308`
- `other_rustc_error`
- `other_build_failure`

Notes:

- lanes with `build.status` `0` or `-1` always emit:
  - `first_failing_compile_file=none`
  - `first_failing_compile_e0425_count=0`
- first failing compile file is extracted from known fragilec markers:
  - rustc compile-failed marker
  - transpile-failed marker

## Regression Gate

```bash
PYTHONDONTWRITEBYTECODE=1 \
python3 -m unittest tests/python/test_mako_rpc_compile_blocker_inventory.py -v
```
