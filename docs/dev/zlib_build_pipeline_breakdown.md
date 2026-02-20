# zlib build pipeline breakdown (Phase 1)

## Why the original task was split

The TODO item "Transpile and compile all required zlib objects and test binaries" is too broad for a single safe change (<500 LOC). It spans target discovery, compile command extraction, transpilation, object compilation, and multi-binary linking.

## Chosen leaf order

1. Deterministic artifact target coverage (`make all`) with reproducible logs/manifests.
2. Compile-unit extraction from `CC` driver logs.
3. One-object transpile+compile replay (`adler32.c`).
4. Expand replay to full `OBJZ`/`OBJG`.
5. Link all test-scope binaries.

This order makes failures diagnosable and keeps each leaf independently testable.

## First leaf scope

The first leaf validates that the harness can consistently build all `make test`-relevant binaries in native mode and capture an artifact manifest plus command logs. This confirms target coverage before introducing Fragile object replay.

## Second leaf scope

The second leaf adds deterministic compile-unit extraction from the `CC` driver log. It normalizes paths relative to the zlib worktree, de-duplicates units, and writes `compile_units_manifest.txt` so later transpile replay can consume a stable source/object plan.

## Third leaf scope

The third leaf replays a single real compile unit (`adler32.c`) through Fragile end-to-end. The harness now selects that unit from driver logs, carries forward compile command include/define flags into `ClangParser`, emits transpiled Rust, compiles it to `adler32.o` via `rustc --emit=obj`, and records replay artifacts in `fragile_object_manifest.txt`.

## Fourth leaf scope (4a)

The fourth leaf is split so each step stays under a few hundred LOC. Subleaf 4a derives a deterministic replay target plan for static `libz.a` coverage: it parses `OBJZ` and `OBJG` from zlib's generated `Makefile`, maps those targets to normalized compile units from `cc_driver.log`, and writes `libza_replay_plan.txt`. This creates a stable contract before bulk object replay (`OBJZ` then `OBJG`).
