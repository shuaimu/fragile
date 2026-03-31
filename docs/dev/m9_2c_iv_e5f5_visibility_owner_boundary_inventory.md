# M9.2.c.iv.e.5.f.5 Visibility/Owner-Boundary Recovery Inventory

**Date**: 2026-03-21
**Task**: Fix function-owner detection in `normalize_unprefixed_function_static_symbol_refs` for visibility-qualified signatures (`pub(crate)`, `pub(super)`, etc.), declaration-only headers, and declaration-order rewrite boundaries.

## Compile Profile

- Driver: `target/release/fragilec`
- Mode: `FRAGILEC_MODE=strict`
- Flags: `-std=gnu++23 -DGTEST_HAS_PTHREAD=1 -w`
- Include tree: full `mako_compile_args()` profile (`src`, `src/rrr`, `src/memdb`, `src/mako`, `test`, `third-party/rusty-cpp/include`, `third-party/googletest/googletest/include`, `third-party/googletest/googletest`)
- Sources:
  - `vendor/mako/src/rrr/base/debugging.cpp`
  - `vendor/mako/src/rrr/base/misc.cpp`

## Baseline (Before f.5)

Run root: `/tmp/fragile_m9_e5f4_probe_20260321T085816Z`

- `debugging.cpp`: `total=288`, `E0425=194`, unresolved `__fsv___func___x_0=186`
- `misc.cpp`: `total=289`, `E0425=194`, unresolved `__fsv___func___x_0=186`

## After f.5

Run root: `/tmp/fragile_m9_e5f4_after_20260321T092048Z`

- `debugging.cpp`: `total=127`, `E0425=22`, unresolved `__fsv___func___x_0=14`
- `misc.cpp`: `total=128`, `E0425=22`, unresolved `__fsv___func___x_0=14`

## Delta

- `debugging.cpp`: total `-161`, E0425 `-172`, unresolved `__fsv___func___x_0` `-172`
- `misc.cpp`: total `-161`, E0425 `-172`, unresolved `__fsv___func___x_0` `-172`

## Implementation Notes

`normalize_unprefixed_function_static_symbol_refs` now:

1. Detects visibility-qualified signatures (`pub(crate)`, `pub(super)`, `pub(self)`, `pub(...)`) plus qualifier forms (`unsafe`, `const`, `async`, `extern "C"`).
2. Tracks multiline signatures and rejects declaration-only headers (`fn ...;`) as function owners.
3. Ignores top-level `__fsv___func_*` declarations when collecting alias sources.
4. Applies declaration-order gating (`line >= decl_line`) so alias rewrite cannot retroactively affect lines before the owning static declaration.
5. Keeps signature lines rewrite-free while still rewriting owned body uses.

## Residual

Residual unresolved `__fsv___func___x_0` remains `14` per TU and should be closed under a follow-up leaf focused on malformed block boundary recovery.
