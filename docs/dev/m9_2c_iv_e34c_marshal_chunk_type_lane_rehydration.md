# M9.2.c.iv.e.34.c — marshal.cpp `chunk` Type-Lane Rehydration

Date: 2026-03-26

## Task sizing analysis
- Target leaf: `M9.2.c.iv.e.34.c`.
- Scope is bounded to unresolved-type candidate heuristics and regression tests.
- Estimated change size: small (<1000 LOC).

## Wrong-Approach Check
- Re-read `docs/dev/wrong.md` before implementation.
- No rollback-pattern additions.
- No force-native fallback.
- No fake success/stub-only bypass marker for this task.

## Root cause
- `chunk` is a bare lowercase type spelling used in `rrr::Marshal` pointer lanes.
- `looks_like_stub_candidate_type_name()` rejected plain lowercase identifiers unless they matched a curated lowercase type allowlist or had other type-like markers.
- `chunk` was not in the allowlist, so unresolved-type closure ignored it.
- Result: no placeholder/alias materialized for `chunk`, causing `E0425` bursts in `marshal.cpp`.

## Implementation
- File: `crates/fragile-clang/src/ast_codegen.rs`
- Added `"chunk"` to `COMMON_LOWERCASE_TYPE_NAMES` in `looks_like_stub_candidate_type_name()`.
- This keeps the fix bounded and generic within the existing unresolved-type closure design.

## Unit-test evidence
Updated tests:
- `test_stub_candidate_type_heuristics_reject_plain_lowercase_identifiers`
- `test_collect_unresolved_type_like_names_treats_common_lowercase_names_as_type_like`

Commands:

```bash
cargo test -p fragile-clang test_stub_candidate_type_heuristics_reject_plain_lowercase_identifiers -- --nocapture
cargo test -p fragile-clang test_collect_unresolved_type_like_names_treats_common_lowercase_names_as_type_like -- --nocapture
```

Result: pass.

## Focused strict replay evidence (`marshal.cpp`)
Command profile (harness-equivalent):

```bash
FRAGILEC_MODE=strict ./target/release/fragilec -c \
  -I vendor/mako/src \
  -I vendor/mako/src/rrr \
  -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako \
  -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1 -std=gnu++23 -w \
  vendor/mako/src/rrr/misc/marshal.cpp \
  -o /tmp/fragile_e34c_marshal_compile_after_kgPOWa/marshal.cpp.o
```

Run root: `/tmp/fragile_e34c_marshal_compile_after_kgPOWa`

Observed markers:
- `status=1` (expected; downstream typed blockers remain)
- `cannot find type \`chunk\`` count: `0`
- `cannot find type \`void\`` count: `0`
- first `E0425`: `cannot find type \`bookmark\` in this scope`

Baseline reference from `e.34.a` inventory:
- `cannot find type \`chunk\`` key count: `31`

Conclusion: `e.34.c` closes the dominant `chunk` unresolved-type cluster and moves failure surface to downstream typed lanes.
