# M9.2.c.iv.e.5.f.2 Live Strict Inventory and `__fsv` Producer Trace

**Date**: 2026-03-21
**Task**: Re-capture live strict compile inventory after f.1 guards and locate upstream producer still emitting unresolved `__fsv___func___x_0`.

## Compile Profile

- Driver: `target/release/fragilec`
- Mode: `FRAGILEC_MODE=strict`
- Flags: `-std=gnu++23 -DGTEST_HAS_PTHREAD=1 -w`
- Include tree: full `mako_compile_args()` profile (`src`, `src/rrr`, `src/memdb`, `src/mako`, `test`, `third-party/rusty-cpp/include`, `third-party/googletest/googletest/include`, `third-party/googletest/googletest`)
- Sources:
  - `vendor/mako/src/rrr/base/debugging.cpp`
  - `vendor/mako/src/rrr/base/misc.cpp`
- Run root: `/tmp/fragile_m9_e5f2_inventory_20260321T035513Z`

## Live Inventory Result

| TU | rc | total errors | E0425 | unresolved `__fsv___func___x_0` |
|---|---:|---:|---:|---:|
| `debugging.cpp` | 1 | 275 | 194 | 186 |
| `misc.cpp` | 1 | 276 | 194 | 186 |

Distribution matches the latest e.5.e refresh (E0425 still dominant, not resolved).

## Producer Trace Evidence

Generated Rust artifacts referenced by rustc diagnostics:

- `/tmp/fragilec_transpiled/debugging.cpp_695bcbbea7452b91_debugging.rs`
- `/tmp/fragilec_transpiled/misc.cpp_4060cb12acfda3b2_misc.rs`

For both generated files:

- `static mut __fsv___func___x_0` declarations: `1`
- total `__fsv___func___x_0` token uses: `189`
- rewritten struct field occurrences: `pub unsafe { __fsv___func___x_0 }: [u16; 3]` -> `1`

Example (`debugging`):

- first unresolved use: `4841: return __builtin_log(unsafe { __fsv___func___x_0 });`
- only declaration: `30408: static mut __fsv___func___x_0: i8 = 0;`

This is not a local missing declaration; it is cross-item alias leakage.

## Upstream Producer (Located)

Primary producer is `normalize_unprefixed_function_static_symbol_refs` in `crates/fragile-clang/src/ast_codegen.rs`.

Why this pass is implicated:

1. It performs alias rewrite of short names like `__x` to `unsafe { __fsv___func___x_0 }`.
2. The leaked output appears outside function bodies (for example struct field identifiers), which can only happen if function ownership tagging is too broad.
3. The helper `is_fn_def_line` treats any `fn ...` prefix as a function start, including declaration-only signatures (`fn ...;`) in trait/extern contexts; that can widen `function_of_line` scopes and allow alias maps to apply outside intended function bodies.

## f.3 Design Direction

Implement owner-stable alias mapping to eliminate generic alias leakage:

- Only rewrite aliases for function bodies with verified body ownership.
- Exclude declaration-only `fn ...;` lines from function-start detection.
- Keep signature/field/type lanes rewrite-free by construction, not by string exclusion alone.
