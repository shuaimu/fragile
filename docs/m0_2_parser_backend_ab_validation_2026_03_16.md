# M0.2 Parser Backend A/B Harness Validation (2026-03-16)

Task leaf: `M0.2` (parser-backend A/B harness with deterministic comparable manifests)

## Scope and LOC estimate
- Scope is already implemented in repository (`scripts/mako_rpc_parser_backend_ab.py`).
- Validation/maintenance effort for this run is small (<1000 LOC) and does not require decomposition.

## Design/Policy checks
- Reviewed wrong-approach guidance in `docs/dev/wrong.md` and `docs/fragile-dev-book.md`.
- Confirmed no fake/fallback method bodies were introduced for this task.

## Execution summary
- Verified harness behavior and deterministic comparable-manifest diffing via focused tests:
  - `python3 -m unittest tests/python/test_mako_rpc_parser_backend_ab.py -v`
  - `python3 -m unittest tests/python/test_mako_rpc_strict_baseline.py -v`

## Regression checks
- Python suite:
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- Workspace Rust suite:
  - `cargo test --workspace --all-targets`

Observed status in local workspace:
- Python suite passed.
- Rust workspace run reported failures in `fragile-clang` integration tests unrelated to M0.2 harness behavior.

Baseline check:
- Compared selected failing integration tests in clean `origin/main` worktree.
- A subset of those failures is baseline-red on `origin/main`; additional local failures are attributable to unrelated in-progress local modifications outside M0.2 scope.

## Artifacts
- Local failing-test logs captured under `/tmp/fragile_fail_logs/`.
- Baseline comparison logs captured under `/tmp/fragile_origin_failcheck/`.
