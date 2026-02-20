# tinyxml2 fragile xmltest build blockers

Date: 2026-02-20

## Scope

This note tracks the first real-world blocker found while implementing:

- Build a transpiled tinyxml2 `xmltest` binary from captured CXX-driver compile/link plan.

## Repro

```bash
cargo test -p fragile-clang --test real_world_tinyxml2_tests \
  test_real_world_tinyxml2_fragile_xmltest_build_from_cxx_driver_plan \
  -- --ignored --nocapture --test-threads=1
```

## Current failing artifacts

- log root: `/tmp/fragile_real_world_tinyxml2_fragile_xmltest_build/fragile_build_logs`
- status: `rustc_fragile_tinyxml2_o.status = 1`
- stderr: `rustc_fragile_tinyxml2_o.stderr`

## First blocker classes

1. C++ cast syntax not normalized in array type contexts:
   - `static_cast<size_t>(ITEM_SIZE)` currently emitted in Rust type positions.
2. Missing type references in generated tinyxml2 output:
   - `XMLNode`, `XMLElement`, `XMLText`, `XMLComment`, `XMLDocument`, `XMLDeclaration`, `XMLUnknown`.

## Next implementation direction

1. Fix cast normalization for `static_cast<...>(...)` in type-sized array contexts before Rust emission.
2. Fix nested tinyxml2 type declaration/reference ordering and cross-reference emission so type names resolve in the single-TU transpiled output.
