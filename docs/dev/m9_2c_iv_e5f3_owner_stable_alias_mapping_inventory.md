# M9.2.c.iv.e.5.f.3 Owner-Stable Function-Static Alias Mapping

## Scope

Implement owner-stable alias rewriting in `normalize_unprefixed_function_static_symbol_refs`
to stop generic alias leakage (`__x`, `__y`, etc.) across unrelated items.

## Compile Profile

- Driver: `target/release/fragilec`
- Mode: `FRAGILEC_MODE=strict`
- Flags: `-std=gnu++23 -DGTEST_HAS_PTHREAD=1 -w`
- Include tree: `mako_compile_args()` equivalent (`src`, `src/rrr`, `src/memdb`, `src/mako`, `test`, `third-party/rusty-cpp/include`, `third-party/googletest/googletest/include`, `third-party/googletest/googletest`)
- Targets:
  - `vendor/mako/src/rrr/base/debugging.cpp`
  - `vendor/mako/src/rrr/base/misc.cpp`

## Before/After Inventory

### Baseline (pre-f.3)

- Run root: `/tmp/fragile_m9_e5f3_baseline_20260321T052858Z`
- `debugging.cpp`: `total=288`, `E0425=194`, unresolved `__fsv___func___x_0=186`
- `misc.cpp`: `total=289`, `E0425=194`, unresolved `__fsv___func___x_0=186`

Representative generated artifact issue:

- `decl_count=1`, `use_count=189`
- leaked struct field rewrite present:
  - `pub unsafe { __fsv___func___x_0 }: [u16; 3]`

### After f.3 owner-stable mapping

- Run root: `/tmp/fragile_m9_e5f3_after2_20260321T055716Z`
- `debugging.cpp`: `total=141`, `E0425=48`, unresolved `__fsv___func___x_0=40`
- `misc.cpp`: `total=142`, `E0425=48`, unresolved `__fsv___func___x_0=40`

Generated artifact checks:

- `decl_count=1`, `use_count=42`
- leaked struct field rewrite removed:
  - `bad_struct_field=0`

## Delta

- Total errors: `288/289 -> 141/142` (`-147` per TU)
- E0425: `194 -> 48` (`-146`)
- unresolved `__fsv___func___x_0`: `186 -> 40` (`-146`)

## What Changed

`normalize_unprefixed_function_static_symbol_refs` now:

1. Recognizes visibility-qualified headers (`pub(crate)`, `pub(super)`, `pub(self)`, `pub(...)`) and qualifier prefixes (`unsafe`, `const`, `async`, `extern "C"`).
2. Tracks multi-line signatures and explicitly rejects declaration-only headers (`fn ...;`) as rewrite owners.
3. Excludes top-level `__fsv___func_*` declarations from alias source collection.
4. Applies owner-stable rewrite gating so alias replacement cannot apply to lines that appear before the owning static declaration line.

## Residual Work

Residual unresolved `__fsv___func___x_0` remains `40/file`; this is tracked as `M9.2.c.iv.e.5.f.4`
for further boundary tightening in malformed function-block recoveries.
