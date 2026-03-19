# M8.3 Parser Backend Migration Notes (Developer + CI)

Date: 2026-03-19
Status: published migration guidance for strict parser-backend cutover

## Scope

These notes describe how to migrate local developer workflows and CI jobs after
the strict parser backend default flip (`M8.1`) and hardening-window policy
enforcement (`M8.2`).

## Effective Behavior

1. Default strict parser backend:
   - `fragile-parser-clang` is used when `FRAGILEC_PARSER_BACKEND` is unset or
     empty.
2. Supported explicit parser backend values:
   - `fragile-parser-clang`
   - `libtooling` (temporary hardening escape hatch only)
3. Temporary codegen escape hatch:
   - `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling`
4. Unsupported legacy aliases:
   - `libclang`
   - `hybrid`

## Hardening Window Policy

The hardening window expiry is encoded in `fragile-driver` as:

- `ESCAPE_HATCH_HARDENING_EXPIRY=2026-04-18`

Policy details:

1. Escape hatch usage emits a deprecation warning on stderr immediately.
2. Escape hatch usage is optionally logged when
   `FRAGILEC_ESCAPE_HATCH_LOG_PATH` is set.
3. Escape hatch usage is rejected after expiry with actionable diagnostics.

Escapes covered by policy:

1. `FRAGILEC_PARSER_BACKEND=libtooling`
2. `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling`

## Developer Migration

1. Stop pinning parser backend for normal strict development:
   - remove `FRAGILEC_PARSER_BACKEND=libtooling` from shell scripts and local
     aliases.
2. Use default strict path:

```bash
FRAGILEC_MODE=strict fragilec -c file.cpp -o file.o
```

3. Use explicit `libtooling` only for short-term unblock during hardening:

```bash
FRAGILEC_MODE=strict FRAGILEC_PARSER_BACKEND=libtooling fragilec -c file.cpp -o file.o
```

4. If parser-core parse succeeds but codegen fallback is temporarily required:

```bash
FRAGILEC_MODE=strict \
FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling \
fragilec -c file.cpp -o file.o
```

## CI Migration

1. Required/default CI lanes should rely on default backend (do not set
   `FRAGILEC_PARSER_BACKEND`).
2. Keep any explicit-`libtooling` compatibility lanes optional/non-required
   during the hardening window.
3. Collect escape-hatch usage telemetry in CI:

```bash
export FRAGILEC_ESCAPE_HATCH_LOG_PATH="$RUNNER_TEMP/fragilec_escape_hatch.log"
```

4. Publish the telemetry file as a CI artifact for trend monitoring.
5. After 2026-04-18, remove remaining explicit `libtooling` lanes and fail on
   any escape-hatch usage.

## Troubleshooting

1. Error: unsupported `FRAGILEC_PARSER_BACKEND` value
   - Use `fragile-parser-clang` (default) or temporary `libtooling`.
2. Error: escape hatch rejected after hardening expiry
   - Remove `FRAGILEC_PARSER_BACKEND=libtooling` and
     `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling`.
3. Warning: deprecation warning spam in logs
   - Expected while escape hatches are still in use; treat as migration debt.

## Validation Commands (M8.3 publication iteration)

1. Focused cutover checks:
   - `cargo test -p fragile-driver strict_parser_backend_validation -- --nocapture`
   - `cargo test -p fragile-cli --bin fragilec strict_parser_backend_validation -- --nocapture`
2. Full regression:
   - `cargo test --workspace --all-targets` (executed in split form with
     isolated long libc++ tests)
   - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
