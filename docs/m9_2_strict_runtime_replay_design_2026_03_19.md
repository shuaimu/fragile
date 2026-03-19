# M9.2 strict runtime replay design (2026-03-19)

## Scope and LOC estimate

Top undone leaf after M9.1 was `M9.2`:

- run strict runtime replay for RPC targets,
- capture deterministic runtime manifests,
- keep strict no-bypass policy.

This is implementable in under 1000 LOC by adding one small orchestration script,
reusing the existing harness runtime execution, and adding focused Python tests.

## Implementation choice

Implemented `scripts/mako_rpc_strict_runtime_replay.py` as a strict wrapper around
`scripts/mako_rpcbench_harness.py`:

- forces strict env (`FRAGILEC_MODE=strict`, `FRAGILEC_PARSER_BACKEND=fragile-parser-clang`),
- rejects parent-env bypass patterns (`FRAGILEC_FORCE_NATIVE_SOURCES`, parser-core escape hatch),
- runs runtime replay on `fragilec` lane,
- records deterministic manifest `strict_runtime_replay_manifest.txt`,
- emits required-artifact contract manifest for runtime evidence.

## Contract details

`M9.2` runtime success requires:

- `lane_fragilec_build_status=0`,
- `lane_fragilec_test_rpc_status=0`,
- `lane_fragilec_failure_class=none`,
- completed trial count equals requested trials,
- all trial `rpc_server.status` and `rpc_client.status` files are `0`.

## Wrong-approach alignment

Checked against `docs/fragile-dev-book.md` and `docs/dev/wrong.md`:

- no target-specific parser/codegen hack was introduced,
- no native-force bypass path was used,
- no semantic stubs/fake runtime behavior added,
- runtime replay evidence is manifest-driven and deterministic.
