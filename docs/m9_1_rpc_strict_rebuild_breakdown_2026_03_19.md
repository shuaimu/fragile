# M9.1 RPC strict rebuild breakdown (2026-03-19)

## Scope analysis

`M9.1` (`test_rpc` + `rpcbench` strict rebuild with new parser backend and no force-native paths)
is broader than a single <1000 LOC leaf because it mixes:

- strict environment policy,
- build-only replay closure,
- blocker-log policy gating,
- and concrete replay evidence capture.

To keep leaves bounded and reviewable, `TODO.md` now decomposes `M9.1` into `M9.1.a`..`M9.1.d`.

## First leaf selected

`M9.1.a` was selected as the first executable leaf:

- enforce strict env contract (`FRAGILEC_MODE=strict`),
- pin parser backend (`FRAGILEC_PARSER_BACKEND=fragile-parser-clang`),
- enforce no-bypass policy (`FRAGILEC_FORCE_NATIVE_SOURCES` disabled/unset),
- ensure parser-core/libtooling escape hatch vars are unset,
- record deterministic manifest evidence.

This leaf is small and testable in script-level unit tests, and it establishes policy correctness
before spending long replay time on build/runtime closure leaves.

## Wrong-approach alignment

Checked against `docs/dev/wrong.md` and `fragile-dev-book.md` wrong-approach section:

- no target-specific parser/codegen hacks,
- no force-native bypass use,
- no semantic stubs/fake runtime bodies,
- no rollback-pattern expansion.
