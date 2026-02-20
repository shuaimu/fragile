# Fragile TODO (Active)

This file tracks the current real-world target sequence only.
Previous backlog items are deprecated and intentionally removed.

## Goal
Run upstream-style tests through the Fragile transpiler with runtime parity:
1. `zlib`
2. `tinyxml2` (only after `zlib` success)

## Global Constraints
- No manual Rust stubs for missing transpilation in target code paths.
- No semantic type remapping shortcuts.
- Keep source-faithful behavior first; optimize later.

## Phase 1: zlib (Current Priority)
- [x] Add pinned `zlib` fixture checkout in the real-world test harness (`madler/zlib` pinned to `51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf`, v1.3.1).
- [x] Capture native baseline with upstream flow (`./configure && make test`) via `real_world_zlib_tests` (logs captured under `/tmp/fragile_real_world_zlib_native_baseline/native_logs`).
- [x] Add transpiler driver path so `CC` in zlib build invokes Fragile flow via a logging `CC` wrapper in `real_world_zlib_tests` (driver logs under `/tmp/fragile_real_world_zlib_cc_driver/driver_logs`).
- [ ] Transpile and compile all required zlib objects and test binaries (`example`, `minigzip`, shared/static variants used by `make test`).
  - [x] Establish deterministic artifact target coverage for upstream `make test` scope (`make all` outputs + manifest/log capture in `real_world_zlib_tests`, logs under `/tmp/fragile_real_world_zlib_required_artifacts/driver_logs`).
  - [x] Parse and normalize compile units from `CC` driver logs into a reproducible source/object list (`compile_units_manifest.txt` in `/tmp/fragile_real_world_zlib_required_artifacts/driver_logs`).
  - [x] Transpile and compile one core object end-to-end (`adler32.c` -> Rust -> `adler32.o`) through the Fragile flow (`fragile_object_manifest.txt` and `rustc_object.*` logs under `/tmp/fragile_real_world_zlib_fragile_adler32_object/driver_logs`).
  - [ ] Expand transpile+compile coverage to all `OBJZ` and `OBJG` units needed for `libz.a`.
    - [x] Derive deterministic `OBJZ`/`OBJG` replay plan from upstream `Makefile` + `cc_driver.log` compile units (`libza_replay_plan.txt` under `/tmp/fragile_real_world_zlib_libza_replay_plan/driver_logs`).
    - [ ] Replay `OBJZ` units through Fragile to `.o` outputs and validate object completeness.
      - [x] Add deterministic `OBJZ` replay harness/logging (`fragile_objz_manifest.txt` + per-object `rustc_objz_*.status` logs under `/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs`).
      - [x] Fix `crc32.c` transpiled replay compile failure (`wrapping_shl` ambiguous integer literal typing) in source-faithful codegen.
      - [x] Fix `.c` unit parsing mode mismatch in OBJZ replay (`deflate.c` `register` rejected under C++17 parser mode).
      - [x] Fix `deflate.c` transpiled replay compile failure for `configuration_table` initializer (`config { ... }` positional initializer emitted as invalid Rust struct literal).
      - [x] Fix `deflate.c` transpiled replay compile failure for chained comparisons emitted from C integer-bool normalization (invalid Rust `a != b != 0` forms).
      - [ ] Re-run full `OBJZ` replay and confirm all expected OBJZ objects compile and are non-empty.
        - Plan: resolve replay blockers one compiler class at a time in `deflate.c` transpiled output, re-run replay after each class, and keep each fix leaf under ~500 LOC.
        - [x] Fix relational comparison cast-parenthesization in codegen so emitted Rust doesn't parse cast types as generic arguments (`(dist) as i32 < 256`).
        - [x] Re-run real-world `OBJZ` replay after cast-parenthesization fix and record the next first blocking error class in `deflate` compile logs.
          - Current first blocker class after replay: external symbol/type resolution in `deflate` output (missing `static_tree_desc_s`, `zcalloc`/`zcfree`, `adler32`/`crc32`, `_tr_*`), followed by pointer-function call and type-width/union-field mismatches.
        - [x] Fix external symbol/type resolution issues in `deflate` replay output (e.g., missing `adler32`/`crc32`/`_tr_*`/`static_tree_desc_s`) with source-faithful cross-unit declarations.
          - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/rustc_objz_deflate_o.stderr`): unresolved symbol/type diagnostics are cleared; current first blocker class is function-pointer/Option invocation and type-width mismatches.
        - [x] Fix pointer/function-pointer invocation lowering regressions in `deflate` replay output (Option function pointers deref/call shape).
          - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/rustc_objz_deflate_o.stderr`): no remaining `Option<fn>` deref/call/assignment diagnostics; next blockers are type-width, pointer arithmetic, enum return-type, and union-field layout issues.
        - [ ] Fix remaining `deflate` type/union field/codegen mismatches in replay output (`freq` union field access, integer width mismatches, pointer arithmetic typing).
          - Plan: resolve by first compiler error class from replay logs; keep each leaf scoped to a single lowering rule family (<500 LOC per leaf).
          - [x] Fix unsized extern array declaration lowering so pointer-typed globals don't emit invalid `= []` initializers (`_length_code`/`_dist_code`).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/rustc_objz_deflate_o.stderr`): `__gv__length_code`/`__gv__dist_code` `*mut u8 = []` diagnostics are cleared; next first blocker class is integer-width/chained-assignment typing in `deflate`.
          - [x] Fix chained-assignment expression lowering that currently returns `()` in typed assignments (`a = b = 2` forms).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_deflate_o_transpiled.rs`): chained forms now lower as value expressions (e.g., `(*s).match_length = unsafe { (*s).prev_length = 2; (*s).prev_length }`), and corresponding `found ()` assignment diagnostics are cleared from `rustc_objz_deflate_o.stderr`.
          - [ ] Fix pointer arithmetic lowering for `ptr + offset_from(...)` patterns to use Rust pointer APIs.
          - [ ] Fix integer width normalization for `u32`/`u64` fields and temporaries in shift/math expressions.
          - [ ] Fix enum return lowering so `block_state` returns emit enum variants instead of integer literals.
          - [ ] Fix union field preservation/access for Huffman tree frequency members (`.fc.freq`).
    - [ ] Replay `OBJG` units through Fragile to `.o` outputs and validate object completeness.
  - [ ] Link transpiled static/shared test binaries used by upstream tests (`example`, `minigzip`, `examplesh`, `minigzipsh`, `example64`, `minigzip64`).
- [ ] Make transpiled build pass zlib test commands used by upstream `make test`.
- [ ] Add parity assertions vs native:
  - [ ] exit status parity
  - [ ] stdout/stderr parity (allowing nondeterministic path filtering if needed)
  - [ ] artifact behavior parity (round-trip and output file checks)
- [ ] Add CI tiering:
  - [ ] smoke: minimal deterministic zlib parity run
  - [ ] nightly: fuller zlib matrix

### Phase 1 Exit Criteria (Must all be true)
- [ ] `zlib` transpiled pipeline passes target `make test` scope with no manual stubs.
- [ ] Parity checks are automated and committed.
- [ ] Failures are reproducible via one documented command.

## Phase 2: tinyxml2 (Starts only after Phase 1 exit)
- [ ] Add pinned `tinyxml2` fixture checkout.
- [ ] Capture native baseline (`make test` / upstream equivalent test command).
- [ ] Reuse the zlib harness pattern for transpiler-vs-native parity.
- [ ] Ensure transpiled `tinyxml2` test binary passes upstream test scope.
- [ ] Add parity assertions (exit code + output + generated files).
- [ ] Add CI coverage (smoke or nightly based on runtime cost).

### Phase 2 Exit Criteria
- [ ] `tinyxml2` transpiled pipeline passes target upstream test scope.
- [ ] Parity checks are stable in CI.
- [ ] No regressions introduced to zlib parity coverage.
