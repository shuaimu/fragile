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
