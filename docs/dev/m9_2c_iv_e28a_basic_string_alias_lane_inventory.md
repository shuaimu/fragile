# M9.2.c.iv.e.28.a Basic-String Alias Lane Inventory

Date: 2026-03-25
Task: `M9.2.c.iv.e.28.a`

## Scope

Bounded post-e.27 reduction targeting the dominant collate/basic_string degraded alias lane:

- `E0308 expected c_void, found *const ()`
- paired `E0599 no method clone on c_void`

Observed in `std_collate_*::do_compare` bodies across:

- `rrr/base/debugging.cpp`
- `rrr/base/misc.cpp`
- `rrr/base/basetypes.cpp`
- `rrr/base/logging.cpp`

## Design Decision

Rejected approach (do not ship): rewrite

- `pub use std::ffi::c_void as basic_string_type_parameter_*;`

into a pointer alias target.

Reason: strict replay failed before rustc with parser-output mapping completeness errors (`string` family canonical alias target check).

Shipped approach:

- Keep alias declarations unchanged.
- Rewrite only degraded local bindings in `do_compare` lanes:
  - `let mut __one/__two: basic_string_type_parameter_* = __lo1/__lo2;`
  - to `let mut __one/__two: *const () = __lo1/__lo2;`

Implementation: `normalize_e28_basic_string_c_void_alias_lane` in `crates/fragile-clang/src/ast_codegen.rs`.

## Wrong-Approach Check

Checked against `docs/dev/wrong.md` and Section 1.3 in `docs/fragile-dev-book.md`:

- No target-specific (`mako`/`rpc`) conditional logic added.
- No native fallback, bypass path, or force-native usage introduced.
- No fake semantic method stubs added for this slice.
- Fix is generic post-processing normalization over emitted code shape.

## Replay Runs

Baseline (pre-change):

- `/tmp/fragile_e28_before_t14i_7pd`

After (post-change):

- `/tmp/fragile_e28a_after2_lr35te_2`

Profile used for after replay:

- compile commands: `vendor/mako/build_rpc_fragilec_make_20260311/compile_commands.json`
- mapping: `vendor/mako/build_rpc_fragilec_make_20260311/parser_output_mapping.json`
- env: `FRAGILEC_MODE=strict`, `FRAGILEC_PARSER_BACKEND=fragile-parser-clang`, `FRAGILE_PARSER_OUTPUT_MAPPING=<path>`

## Before vs After

Per-file totals:

- debugging: `135 -> 126`
- misc: `131 -> 122`
- basetypes: `120 -> 114`
- logging: `165 -> 157`

Aggregate class deltas:

- total: `551 -> 519` (`-32`)
- `E0308`: `268 -> 252` (`-16`)
- `E0599`: `85 -> 69` (`-16`)
- `E0609`: `16 -> 16` (non-increase)
- `E0425`: `42 -> 42` (non-increase)
- `E0530`: `12 -> 12` (non-increase)
- `E0277`: `47 -> 47` (non-increase)
- `E0428`: `26 -> 26` (non-increase)

## Non-Increase Evidence

- Maintained non-increase for `E0609`, `E0425`, `E0530`, `E0277`, and `E0428`.
- Maintained parser-output mapping completeness by preserving canonical `string` family alias declarations.

Targeted marker check:

- `expected c_void, found *const ()`: eliminated across all four files (`16 -> 0`).

## Test Evidence

Focused unit tests added:

- `test_e28_rewrites_basic_string_local_bindings_from_lo_params`
- `test_e28_preserves_basic_string_alias_declaration`
- `test_e28_preserves_non_lo_binding_shapes`

Command:

- `cargo test -p fragile-clang test_e28_ -- --nocapture`
