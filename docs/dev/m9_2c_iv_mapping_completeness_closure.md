# M9.2.c.iv.b/c: Mapping-Completeness Closure for RPC Base Files

## Summary

M9.2.c.iv.b (optional/string mapping-completeness) and M9.2.c.iv.c (tuple/variant
mapping-completeness) were **already resolved** by the M9.2.c.ii/iii fix
(commit `91fb21e`). This document records the analysis and evidence.

## Background

The M9.2.c.iv.a blocker inventory (run-root
`/tmp/fragile_m9_2_strict_runtime_replay_20260319T160717Z_p1608468`) captured
12 unique errors, of which 4 were mapping-completeness failures in:
- `rrr/base/debugging.cpp`
- `rrr/base/logging.cpp`
- `rrr/base/misc.cpp`
- `rrr/base/basetypes.cpp`

## Specific Blocker Patterns (Now Resolved)

### Optional family (M9.2.c.iv.b)
| Alias | Target | Resolution |
|-------|--------|------------|
| `optional_basic_string_wchar` | `optional_basic_string_wchar_t` | `optional_` prefix match |
| `optional_construct_from_invoke` | `__optional_construct_from_invoke_tag` | `__` internal prefix |
| `optional_construct_from` | `__optional_construct_from_invoke_tag` | `__` internal prefix |
| `optional_construct` | `__optional_construct_from_invoke_tag` | `__` internal prefix |
| `optional_std` | `optional_std_locale` | `optional_` prefix match |

Unresolved structs: `optional_basic_string_char`, `optional_basic_string_wchar_t`,
`optional_std_locale` — all match `optional_` family prefix.

### String family (M9.2.c.iv.b)
| Alias | Target | Resolution |
|-------|--------|------------|
| `basic_string_char16` | `basic_string_char16_t` | `basic_string_` prefix match |
| `basic_string_char32` | `basic_string_char32_t` | `basic_string_` prefix match |
| `basic_string_char8` | `basic_string_char8_t` | `basic_string_` prefix match |
| `basic_string_char_char_traits_*` | `basic_string_char__char_traits_char__allocator_char` | `basic_string_` prefix match |
| `basic_string_wchar` | `basic_string_wchar_t` | `basic_string_` prefix match |
| `string_impl` | `__string_impl_base` | `__` internal prefix |

Unresolved structs: `basic_string_char16_t`, `basic_string_char32_t`,
`basic_string_char8_t`, `basic_string_char__char_traits_char__allocator_char`,
`basic_string_char`, `basic_string_wchar_t` — all match `basic_string_` family prefix.

### Tuple/Variant family (M9.2.c.iv.c)
| Struct | Resolution |
|--------|------------|
| `tuple_DefaultType_____` | `tuple_` prefix match |
| `variant__Types___` | `variant_` prefix match |

## How the Fix Works

The M9.2.c.ii/iii fix (`parser_output_alias_target_matches_family`) relaxed
the mapping-completeness check to accept alias targets that match ANY detection
prefix for the same family, not just the canonical pre-generated prefix. The
`__` internal helper prefix was also accepted unconditionally.

The key configuration is `PARSER_OUTPUT_MAPPED_FAMILY_ALIAS_PREFIX_SPECS` in
`crates/fragile-clang/src/lib.rs`, which defines detection prefixes per family.

## Current Blocker Taxonomy (Post-Closure)

With mapping-completeness resolved, the 4 blocker files now fail with downstream
rustc/codegen errors (M9.2.c.iv.d scope):

| File | Error Class | Description |
|------|-------------|-------------|
| `debugging.cpp` | rustc syntax + E0428 | Nested struct init syntax, duplicate type definitions |
| `logging.cpp` | Missing headers | `rusty/box.hpp`, `rusty/result.hpp` etc. (needs `-I` flag) |
| `misc.cpp` | rustc syntax | Nested struct init syntax |
| `basetypes.cpp` | Unresolved type invariant | `byte___memory_order_modifier` type unknown |

These are generic transpiler codegen issues, not STL mapping issues.
