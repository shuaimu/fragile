# M9.2.c.iv.e.34.b — event.cc Mapping-Completeness Canonical Target Normalization

Date: 2026-03-26

## Task sizing analysis
- Target leaf: `M9.2.c.iv.e.34.b`.
- Scope is bounded to parser-output mapping-completeness validation for covered families.
- Estimated change size: small (<1000 LOC), isolated to validation logic/tests and task evidence docs.

## Plan before execution
1. Reproduce the `event.cc` mapping-completeness failure shape from strict replay evidence.
2. Implement a bounded canonical-target normalization pass for covered-family checks.
3. Add focused unit tests:
- accept `std___map_iterator...` map-family alias targets.
- still reject non-family `std___...` fallback targets.
4. Re-run focused compile replay for `event.cc` and verify mapping-completeness diagnostics are absent.

## Wrong-Approach Check
- Re-read `docs/dev/wrong.md`.
- No rollback-pattern additions.
- No target-specific bypasses or force-native fallback.
- No fake success markers; validation still fails on real downstream rustc errors after gate closure.

## Implementation
- File: `crates/fragile-clang/src/lib.rs`
- Added helper:
  - `parser_output_normalize_covered_family_target_spelling(target)`
  - canonicalizes leading `std___` to `std_` for covered-family completeness checks.
- Applied normalization in both alias-target and struct-name completeness checks before prefix/family matching.

## Unit-test evidence
- `parser_output_mapping_completeness_validation_accepts_std_triple_underscore_map_targets`
- `parser_output_mapping_completeness_validation_still_rejects_nonfamily_std_triple_underscore_targets`

Command:

```bash
cargo test -p fragile-clang std_triple_underscore -- --nocapture
```

Result: pass.

## Focused strict replay evidence (`event.cc`)
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
  vendor/mako/src/rrr/reactor/event.cc \
  -o /tmp/fragile_e34b_event_compile_after_WfIQoE/event.cc.o
```

Run root: `/tmp/fragile_e34b_event_compile_after_WfIQoE`

Observed markers:
- `status=1`
- `mapping_completeness_present=0`
- no `active parser-output handoff mapping completeness checks failed` markers.

Downstream blockers now surfaced in `event.cc` lane (expected next work):
- `E0425` (`void`, intrinsic helpers, unresolved functions)
- `E0308`
- `E0599`
- `E0609`

Conclusion: `e.34.b` mapping-completeness gate is closed; failures have moved to downstream typed rustc/codegen/runtime-surface classes tracked by subsequent leaves.
