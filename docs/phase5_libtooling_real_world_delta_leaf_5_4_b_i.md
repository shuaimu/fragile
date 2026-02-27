# Phase 5.4.b.i: LibTooling-Primary Real-World Delta Classification

Date: 2026-02-27

## Scope

Record first-failure class/code deltas and generated-surface diffs (including fallback-stub inventory) for the pinned RapidJSON strict replay used by Phase 5.4.

Pinned upstream commit:

- `rapidjson` `f54b0e47a08782a6131cc3d60f94d038fa6e0a51` (v1.1.0)

## Replay evidence set

Matrix run root used for this analysis:

- `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_2524958_1772228600902022761`

Observed state from this run:

- `backend_libclang/cmake_configure.status=0`
- `backend_libclang/cmake_build.status=0`
- `backend_libclang/first_failing_compile_class.txt=none`
- `backend_libclang/first_failing_compile_stderr.txt=<none>` (no first-failure compile)
- `backend_libtooling/cmake_configure.status=0`
- `backend_libtooling/cmake_build.status` is missing (build did not complete)
- `backend_libtooling/fragilec_driver.log` stops at first real TU:
  - `example/capitalize/capitalize.cpp`

Attempted ignored replay command:

```bash
cargo test -p fragile-clang --test real_world_rapidjson_tests \
  test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure \
  -- --ignored --nocapture
```

This run was terminated after the libtooling leg remained in the same first-TU compile state for >10 minutes (uninterruptible I/O state, no additional backend artifact progress).

## Direct TU reproduction for generated-surface comparison

To capture deterministic generated-surface evidence for the blocking TU (`capitalize.cpp`), direct strict compiles were run with sidecar retention:

```bash
FRAGILEC_MODE=strict FRAGILEC_PARSER_BACKEND=libclang FRAGILEC_KEEP_RS=1 fragilec ... -c capitalize.cpp
timeout 180s FRAGILEC_MODE=strict FRAGILEC_PARSER_BACKEND=libtooling FRAGILEC_KEEP_RS=1 fragilec ... -c capitalize.cpp
```

Output directory:

- `/tmp/fragile_phase5_4b1_capitalize_surface_1772230345`

Results:

- `libclang`: status `0`, sidecar emitted:
  - `capitalize_libclang.fragile.rs` (`39066` lines)
- `libtooling`: status `124` (timeout), no sidecar emitted.

## First-failure class/code delta classification

### Baseline (`libclang`)

- Configure/build both succeed (`0/0`) in strict no-tests CMake replay.
- No first failing compile command.
- First-failure class is `none`.
- `error[E0425]` count in first-failure stderr: `0`.

### LibTooling-primary (`libtooling`)

- Configure succeeds (`0`), build does not reach a first failing compile artifact in the current replay because the first TU compile does not complete in bounded time.
- First-failure class/code is therefore unavailable from replay artifacts.

Blocking delta classification:

1. `non_terminating_or_pathological_compile_time` at first real TU (`capitalize.cpp`) in LibTooling-primary strict replay.
2. Because 1 blocks first-failure capture, class/code parity deltas are currently dominated by runtime behavior (completion vs non-completion), not by `E0425`/class drift.

## Generated-surface diff and fallback-stub inventory

### Surface diff status

- `libclang`: full generated surface available (`capitalize_libclang.fragile.rs`).
- `libtooling`: no generated sidecar available under 180s timeout for the same TU.
- Effective delta: LibTooling-primary fails to produce a comparable generated surface for the first replay TU.

### Baseline fallback-stub inventory (`capitalize_libclang.fragile.rs`)

Quantitative markers:

- `/// Placeholder for C++ ...` blocks: `56`
- RapidJSON/Generic placeholder blocks: `10`
- `= std::ffi::c_void;` aliases: `172`
- `kParseErrorUnspecificSyntaxError` references: `18`

Representative placeholder/fallback surfaces:

- `rapidjson::GenericReader::Token`
- `rapidjson::GenericReader::IterativeParsingState`
- `GenericReader::UTF8::::::UTF8::`
- `GenericInsituStringStream::UTF8::`
- `GenericStringStream::UTF8::`
- `GenericStringBuffer::UTF8::`

Representative runtime fallback behavior present in baseline sidecar:

- `GenericReader_UTF8___UTF8_::Parse(...)` fallback path using:
  - `fragile_extract_input_bytes_from_stream`
  - `fragile_rapidjson_render_to_stdout_for_handler`
  - error return `ParseErrorCode::kParseErrorUnspecificSyntaxError` when extraction/render fails.

## Blocking regression summary for 5.4.c intake

Top-ranked blockers from this 5.4.b.i capture:

1. **Completion blocker**: LibTooling-primary strict replay does not complete first TU compile (`capitalize.cpp`) in practical bounded time.
2. **Observability blocker**: Without completion, the replay cannot emit first-failure class/code artifacts or generated sidecars for parity diffing.

These blockers are now explicitly classified and can be used as entry criteria for `5.4.c.i`.
