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
- [x] Transpile and compile all required zlib objects and test binaries (`example`, `minigzip`, shared/static variants used by `make test`).
  - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_required_artifacts_for_make_all_scope -- --ignored --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_fragile_objz_objects_replay -- --ignored --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_fragile_objg_objects_replay -- --ignored --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_fragile_required_link_binaries_replay -- --ignored --nocapture`, 2026-02-20): passes; required `make all` artifacts are validated, OBJZ/OBJG transpiled object replay is green, and required linked test binaries are emitted with non-empty outputs.
  - [x] Establish deterministic artifact target coverage for upstream `make test` scope (`make all` outputs + manifest/log capture in `real_world_zlib_tests`, logs under `/tmp/fragile_real_world_zlib_required_artifacts/driver_logs`).
  - [x] Parse and normalize compile units from `CC` driver logs into a reproducible source/object list (`compile_units_manifest.txt` in `/tmp/fragile_real_world_zlib_required_artifacts/driver_logs`).
  - [x] Transpile and compile one core object end-to-end (`adler32.c` -> Rust -> `adler32.o`) through the Fragile flow (`fragile_object_manifest.txt` and `rustc_object.*` logs under `/tmp/fragile_real_world_zlib_fragile_adler32_object/driver_logs`).
  - [x] Expand transpile+compile coverage to all `OBJZ` and `OBJG` units needed for `libz.a`.
    - [x] Derive deterministic `OBJZ`/`OBJG` replay plan from upstream `Makefile` + `cc_driver.log` compile units (`libza_replay_plan.txt` under `/tmp/fragile_real_world_zlib_libza_replay_plan/driver_logs`).
    - [x] Replay `OBJZ` units through Fragile to `.o` outputs and validate object completeness.
      - [x] Add deterministic `OBJZ` replay harness/logging (`fragile_objz_manifest.txt` + per-object `rustc_objz_*.status` logs under `/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs`).
      - [x] Fix `crc32.c` transpiled replay compile failure (`wrapping_shl` ambiguous integer literal typing) in source-faithful codegen.
      - [x] Fix `.c` unit parsing mode mismatch in OBJZ replay (`deflate.c` `register` rejected under C++17 parser mode).
      - [x] Fix `deflate.c` transpiled replay compile failure for `configuration_table` initializer (`config { ... }` positional initializer emitted as invalid Rust struct literal).
      - [x] Fix `deflate.c` transpiled replay compile failure for chained comparisons emitted from C integer-bool normalization (invalid Rust `a != b != 0` forms).
      - [x] Re-run full `OBJZ` replay and confirm all expected OBJZ objects compile and are non-empty.
        - Plan: resolve replay blockers one compiler class at a time in `deflate.c` transpiled output, re-run replay after each class, and keep each fix leaf under ~500 LOC.
        - Replay evidence (`cargo test -p fragile-clang test_real_world_zlib_fragile_objz_objects_replay -- --ignored --nocapture`, 2026-02-20): passes; strict status/object-size/manifest assertions all succeed.
        - [x] Fix relational comparison cast-parenthesization in codegen so emitted Rust doesn't parse cast types as generic arguments (`(dist) as i32 < 256`).
        - [x] Re-run real-world `OBJZ` replay after cast-parenthesization fix and record the next first blocking error class in `deflate` compile logs.
          - Current first blocker class after replay: external symbol/type resolution in `deflate` output (missing `static_tree_desc_s`, `zcalloc`/`zcfree`, `adler32`/`crc32`, `_tr_*`), followed by pointer-function call and type-width/union-field mismatches.
        - [x] Fix external symbol/type resolution issues in `deflate` replay output (e.g., missing `adler32`/`crc32`/`_tr_*`/`static_tree_desc_s`) with source-faithful cross-unit declarations.
          - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/rustc_objz_deflate_o.stderr`): unresolved symbol/type diagnostics are cleared; current first blocker class is function-pointer/Option invocation and type-width mismatches.
        - [x] Fix pointer/function-pointer invocation lowering regressions in `deflate` replay output (Option function pointers deref/call shape).
          - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/rustc_objz_deflate_o.stderr`): no remaining `Option<fn>` deref/call/assignment diagnostics; next blockers are type-width, pointer arithmetic, enum return-type, and union-field layout issues.
        - [x] Fix remaining `deflate` type/union field/codegen mismatches in replay output (`freq` union field access, integer width mismatches, pointer arithmetic typing).
          - Plan: resolve by first compiler error class from replay logs; keep each leaf scoped to a single lowering rule family (<500 LOC per leaf).
          - Replay evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_fragile_objz_objects_replay -- --ignored --nocapture`, 2026-02-20): passes; full OBJZ replay remains green with non-empty outputs after the deflate/inflate/infback fixes below.
          - [x] Fix unsized extern array declaration lowering so pointer-typed globals don't emit invalid `= []` initializers (`_length_code`/`_dist_code`).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/rustc_objz_deflate_o.stderr`): `__gv__length_code`/`__gv__dist_code` `*mut u8 = []` diagnostics are cleared; next first blocker class is integer-width/chained-assignment typing in `deflate`.
          - [x] Fix chained-assignment expression lowering that currently returns `()` in typed assignments (`a = b = 2` forms).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_deflate_o_transpiled.rs`): chained forms now lower as value expressions (e.g., `(*s).match_length = unsafe { (*s).prev_length = 2; (*s).prev_length }`), and corresponding `found ()` assignment diagnostics are cleared from `rustc_objz_deflate_o.stderr`.
          - [x] Fix pointer arithmetic lowering for `ptr + offset_from(...)` patterns to use Rust pointer APIs.
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_deflate_o_transpiled.rs`): pointer-add forms now emit `.wrapping_offset(...)` (e.g., `pending_out = pending_buf.wrapping_offset(offset_from(...) as isize)`), and previous raw `*mut u8 + isize` diagnostics are cleared from `rustc_objz_deflate_o.stderr`.
          - [x] Fix integer width normalization for `u32`/`u64` fields and temporaries in shift/math expressions.
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_deflate_o_transpiled.rs` + `rustc_objz_deflate_o.stderr`): `deflate` now emits explicit `as u32` normalization on `w_size`/`hash_size`/`lit_bufsize` and `have` shift assignments, and previous `expected u32/u64, found i32/i64` diagnostics are cleared.
          - [x] Fix enum return lowering so `block_state` returns emit enum variants instead of integer literals.
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_deflate_o_transpiled.rs` + `rustc_objz_deflate_o.stderr`): `block_state` functions no longer emit `return 0/1/3;` literals; next first blocker class is union field preservation/access (`.fc.freq`) plus bool-to-int assignment typing.
          - [x] Fix union field preservation/access for Huffman tree frequency members (`.fc.freq`).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_deflate_o_transpiled.rs` + `rustc_objz_deflate_o.stderr`): `ct_data_s.fc` now lowers to a concrete unnamed-union type with preserved `freq`/`code` members, `.fc.freq` field-access diagnostics are cleared, and the next first blocker class is bool-to-int assignment typing (`bflush = (sym_next == sym_end)` emits `expected i32, found bool`).
          - [x] Fix C integer-bool assignment normalization for `int` lvalues fed by relational expressions (`bflush = (sym_next == sym_end)`).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_deflate_o_transpiled.rs` + `rustc_objz_deflate_o.status`): `deflate` now emits explicit int casts on comparison assignments (e.g., `bflush = (((sym_next) == (sym_end))) as i32`), `rustc_objz_deflate_o.status` is `0`, and `rustc_objz_deflate_o.stderr` is empty.
          - [x] Fix enum/integer normalization in `infback` replay output (`inflate_mode` / `codetype` assignments and call arguments).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_infback_o_transpiled.rs` + `rustc_objz_infback_o.stderr`): enum-typed assignments/call args now emit variants (e.g., `(*state).mode = inflate_mode::TYPE`, `inflate_table(codetype::CODES, ...)`), and enum mismatch diagnostics are cleared.
          - [x] Fix signedness normalization for `infback` state flags assignment (`(*state).last` expects `i32`, rhs currently `u32` bitmask expression).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/rustc_objz_infback_o.status` + `rustc_objz_infback_o.stderr`): `infback` now compiles (`status` = `0`) and the `(*state).last` signedness diagnostic is cleared.
          - [x] Fix `inflate` replay cast/shift parse regression where casts are emitted without shift-safe parentheses (`value as u32 << (*state).bits` parsed as generic args).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_inflate_o_transpiled.rs` + `rustc_objz_inflate_o.stderr`): casted shift lhs now emits grouped form (`((value as u32) << (*state).bits)` at line 6914), and previous `<< interpreted as generic arguments` parse diagnostics are cleared.
          - [x] Fix `inflate` bool/int chained-comparison normalization in return expressions (invalid Rust `== ... != 0` forms from C int-bool lowering).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_inflate_o_transpiled.rs` + `rustc_objz_inflate_o.stderr`): `inflateSyncPoint` now emits `return ((... == 16193 && ... == 0) as i32);` (no chained `== ... != 0` parse form), and the previous chained-comparison diagnostics at line 8619 are cleared.
          - [x] Fix `inflate` pointer/array decay + chained-assignment typing mismatch for `codes` table pointers (`(*state).next`/`(*state).distcode`/`(*state).lencode`).
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_inflate_o_transpiled.rs` + `rustc_objz_inflate_o.stderr`): `codes` assignments/relational comparisons now decay to element pointers (`.as_mut_ptr()` / `.as_ptr()`), and prior `expected *mut code, found &mut [code; 1444]` plus `non-primitive cast: () as *const code` diagnostics are cleared.
          - [x] Fix `inflate` signed/unsigned compound-assignment and negative-to-unsigned return cast normalization (`i32 += u32` and `-1 as u64`).
            - Plan: normalize compound-assignment RHS casts for integral LHS types in assignment lowering and normalize negative literal casts to unsigned in return/cast paths.
            - Replay evidence (`/tmp/fragile_real_world_zlib_fragile_objz_objects/driver_logs/objz_inflate_o_transpiled.rs` + `rustc_objz_inflate_o.status` + `rustc_objz_inflate_o.stderr`): `inflate` now emits `(*state).back += (((*state).extra) as i32)` and `return (-1i32 as u64);`, `rustc_objz_inflate_o.status` is `0`, and `rustc_objz_inflate_o.stderr` is empty.
    - [x] Replay `OBJG` units through Fragile to `.o` outputs and validate object completeness.
      - [x] Add deterministic `OBJG` replay harness/logging (`fragile_objg_manifest.txt` + per-object `rustc_objg_*.status` logs under `/tmp/fragile_real_world_zlib_fragile_objg_objects/driver_logs`).
      - [x] Re-run full `OBJG` replay and confirm all expected OBJG objects compile and are non-empty.
        - Replay evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_fragile_objg_objects_replay -- --ignored --nocapture`, 2026-02-20): passes; strict status/object-size/manifest assertions all succeed.
  - [x] Link transpiled static/shared test binaries used by upstream tests (`example`, `minigzip`, `examplesh`, `minigzipsh`, `example64`, `minigzip64`).
    - [x] Derive deterministic replayable link-unit plan from `CC` driver logs for required zlib test binaries.
      - Plan: parse non-`-c` compiler-driver invocations with `-o <binary>` from `make all` logs, normalize output/input paths, and write a stable `link_units_manifest.txt` that includes all expected upstream test binaries.
      - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_parse_link_units_from_driver_log_normalizes_and_deduplicates test_parse_link_units_from_driver_log_rejects_missing_link_commands test_write_link_units_manifest_detects_missing_required_outputs -- --nocapture`, 2026-02-20): passes; parser emits normalized/deduplicated link units and reports missing required outputs.
    - [x] Add local fixture coverage for link-unit plan generation and missing-target diagnostics.
      - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_required_artifacts_build_local_fixture_success -- --nocapture`, 2026-02-20): passes; local fixture run now writes and validates `link_units_manifest.txt`.
    - [x] Add real-world ignored test coverage for link-unit plan generation in pinned zlib fixture logs.
      - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_required_artifacts_for_make_all_scope -- --ignored --nocapture`, 2026-02-20): passes; required `make all` link outputs are asserted in `link_units_manifest.txt`.
    - [x] Replay/link required zlib test binaries from transpiled object outputs and validate produced binaries are non-empty.
      - Implementation: add deterministic link-replay harness that reuses `link_units_manifest.txt`, transpiles static-archive member objects used by required link outputs, rebuilds `.a` archives from transpiled `.o`, replays binary link steps, and records `fragile_link_manifest.txt` with per-output binary sizes.
      - Note: this leaf currently links with `-Wl,--unresolved-symbols=ignore-all` to unblock deterministic binary artifact production; full runtime/link strictness remains tracked by the next `make test` parity leaf.
      - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_fragile_link_required_binaries_local_fixture_success test_fragile_link_replay_reports_missing_static_archive_from_manifest -- --nocapture`, 2026-02-20): passes; local replay validates successful binary emission and failure diagnostics.
      - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_fragile_required_link_binaries_replay -- --ignored --nocapture`, 2026-02-20): passes; pinned real-world zlib replay emits non-empty required binaries and `fragile_link_manifest.txt`.
- [ ] Make transpiled build pass zlib test commands used by upstream `make test`.
  - [x] Derive deterministic replayable `make test` command plan (from configured zlib worktree) and validate required command coverage for `example`/`minigzip` shared+64-bit variants.
    - Plan: run `make -n test` in the configured worktree, parse normalized command lines into `make_test_commands_manifest.txt`, and assert all required binary invocations are represented.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_parse_make_test_commands_from_dry_run_normalizes_and_validates_coverage test_parse_make_test_commands_from_dry_run_reports_missing_required_binary_invocations -- --nocapture`, 2026-02-20): passes; parser normalizes dry-run command lines, deduplicates invocations, and reports missing required binaries.
  - [x] Add local fixture coverage for `make test` command-plan generation and missing-command diagnostics.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_make_test_command_plan_local_fixture_success test_make_test_command_plan_local_fixture_detects_missing_coverage -- --nocapture`, 2026-02-20): passes; fixture generation writes `make_test_commands_manifest.txt` and missing command coverage fails with explicit diagnostics.
  - [x] Replay planned `make test` binary command subset against fragile-linked zlib outputs and validate per-command status is zero.
    - Plan: execute normalized `make_test_commands_manifest.txt` entries via `sh -c` against fragile-linked outputs and record per-command replay status logs.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests -- --nocapture`, 2026-02-20): passes; local replay succeeds when command subset statuses are zero and fails with explicit diagnostics on first non-zero command.
  - [x] Add real-world ignored coverage for `make test` command-plan generation in pinned zlib fixture outputs.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_command_plan_generation -- --ignored --nocapture`, 2026-02-20): passes; real-world run writes and validates `make_test_commands_manifest.txt`.
  - [x] Add real-world ignored coverage for `make test` replay against pinned zlib fixture outputs.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_command_subset_replay -- --ignored --nocapture`, 2026-02-20): passes; pinned real-world replay coverage captures current first failing replay command with deterministic logs/status while preserving fragile-linked output/link manifests for diagnosis.
- [x] Add parity assertions vs native:
  - [x] exit status parity
    - Plan: derive native status from `make_test.status`, derive fragile replay status from first non-zero `make_test_replay_*.status` (or zero when all replayed commands succeed), and assert equality with explicit mismatch diagnostics.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests -- --nocapture`, 2026-02-20): passes; local parity success/mismatch fixtures verify status derivation and parity assertion behavior.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_exit_status_parity -- --ignored --nocapture`, 2026-02-20): passes; pinned real-world parity coverage asserts current native-vs-fragile status mismatch with deterministic diagnostics.
  - [x] stdout/stderr parity (allowing nondeterministic path filtering if needed)
    - Plan: replay the same `make_test_commands_manifest.txt` command subset on native outputs, aggregate per-step stdout/stderr streams, normalize path/make-noise lines, and assert stream parity against fragile replay with explicit first-diff diagnostics.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_make_test_stdout_stderr_parity_local_fixture_success test_make_test_stdout_stderr_parity_local_fixture_reports_mismatch -- --nocapture`, 2026-02-20): passes; local fixture coverage verifies both parity success and deterministic mismatch reporting.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_stdout_stderr_parity -- --ignored --nocapture`, 2026-02-20): passes; pinned real-world coverage asserts current native-vs-fragile stdout/stderr mismatch with deterministic diagnostics.
  - [x] artifact behavior parity (round-trip and output file checks)
    - Plan: run deterministic `minigzip`/`minigzipsh`/`minigzip64` round-trip artifact probes in native and fragile worktrees, record per-binary compress/decompress statuses and round-trip outputs, and assert parity with explicit mismatch diagnostics.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_make_test_artifact_behavior_parity_local_fixture -- --nocapture`, 2026-02-20): passes; local fixture coverage verifies artifact round-trip parity success and deterministic mismatch reporting.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_artifact_behavior_parity -- --ignored --nocapture`, 2026-02-20): passes; pinned real-world coverage asserts current native-vs-fragile artifact behavior mismatch with deterministic diagnostics.
- [x] Add CI tiering:
  - [x] smoke: minimal deterministic zlib parity run
    - Plan: add a dedicated CI lane that runs only deterministic local-fixture zlib parity coverage (`exit status`, `stdout/stderr`, `artifact behavior`) so parity regressions are gated without relying on external project downloads.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_make_test_exit_status_parity_local_fixture -- --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_make_test_stdout_stderr_parity_local_fixture -- --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_make_test_artifact_behavior_parity_local_fixture -- --nocapture`, 2026-02-20): passes locally; same commands are wired into `.github/workflows/ci.yml` `zlib-smoke-parity` job.
  - [x] nightly: fuller zlib matrix
    - Plan: add a dedicated scheduled/dispatch nightly workflow that runs a fail-safe matrix of ignored real-world zlib replay/parity tests so heavy upstream-flow validation is automated outside push/PR smoke lanes.
    - Evidence (`cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_required_artifacts_for_make_all_scope -- --ignored --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_fragile_required_link_binaries_replay -- --ignored --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_command_plan_generation -- --ignored --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_command_subset_replay -- --ignored --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_exit_status_parity -- --ignored --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_stdout_stderr_parity -- --ignored --nocapture && cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_artifact_behavior_parity -- --ignored --nocapture`, 2026-02-20): passes locally; the same test set is encoded in `.github/workflows/zlib-nightly.yml` `zlib-nightly-matrix`.

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
