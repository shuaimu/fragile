# RPC Compile Blocker Inventory Leaf 2.1 Design (2026-03-12)

## Objective

Implement deterministic compile-blocker inventory capture for RPC harness build lanes.
The inventory must extract, per lane:

- first failing compile blocker class
- first failing compile file (if available)
- unresolved-name diagnostic count (`error[E0425]`)

The output must be stable and machine-readable for follow-up leaves (`2.2+`).

## Scope Sizing

Estimated implementation size was small (<500 LOC total):

- script: ~170 LOC (`scripts/mako_rpc_compile_blocker_inventory.py`)
- fixture tests: ~230 LOC (`tests/python/test_mako_rpc_compile_blocker_inventory.py`)
- docs/TODO updates: small

No additional TODO decomposition was required for leaf `2.1`.

## Scope

Included:

- deterministic lane artifact reads (`lane_<lane>/build.status`, `lane_<lane>/build.stderr`)
- deterministic blocker-family classification
- first failing compile-file extraction from known fragilec log markers
- unresolved-name (`E0425`) counting
- emitted lane artifacts:
  - `first_failing_compile_class.txt`
  - `first_failing_compile_file.txt`
  - `first_failing_compile_e0425_count.txt`
- emitted run-root manifest:
  - `rpc_compile_blocker_inventory_manifest.txt`
- fixture regressions for success, skipped, transpile failure, rustc failure family, and missing required artifacts

Not included:

- blocker ranking/fix logic (`2.2+`)
- direct compiler/codegen changes

## Wrong-Approach Check

Aligned with `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md`:

- no RPC-target-specific transpiler/codegen hacks
- no semantic fallback stubs or fake method bodies
- no force-native bypass path
- inventory reflects real harness artifacts only; it does not fabricate blocker outcomes

## Test Execution

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/python/test_mako_rpc_compile_blocker_inventory.py -v`
- full workspace suite:
  - `cargo test`
