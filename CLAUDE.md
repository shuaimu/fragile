# CLAUDE.md

Guidance for working in this repository.

## Project Overview

**Fragile** currently has two active C++ lanes:
- `fragile` CLI: transpile C++ to Rust source.
- `fragilec` driver shim: strict-mode C++ compiler-driver replacement that transpiles to Rust, then invokes `rustc`.

Pipeline:

```
C++ Source -> Clang AST -> Rust Source (unsafe) -> rustc -> Binary
```

## Current Status (as of 2026-02-28)

- `fragilec` is **strict-only** (`FRAGILEC_MODE=auto/pass` removed).
- Phase 5 cutover date: **2026-02-28**.
- M8.1 cutover date: **2026-03-18**.
  - strict default parser backend is now `fragile-parser-clang` (`FRAGILEC_PARSER_BACKEND` unset/empty -> `fragile-parser-clang`).
  - explicit escape hatch to legacy backend: `FRAGILEC_PARSER_BACKEND=libtooling`.
- M8.2 escape hatch hardening window: expires **2026-04-18**.
  - escape hatches emit deprecation warnings and are rejected after expiry.
  - usage logging via `FRAGILEC_ESCAPE_HATCH_LOG_PATH` env var.
- Latest strict backend-matrix parity evidence (run root: `/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_484423_1772259242504942926`):
  - `backend=libtooling` and baseline both report `configure_status=0`, `build_status=2`, `build_timed_out=false`, `first_failure_class=other_rustc_error`, `first_failure_e0425_count=0`.
  - runtime parity markers are locked: `runtime_parity_vs_baseline=true`, `condense_run_status_delta_vs_baseline=0`, `pretty_run_status_delta_vs_baseline=0`.
- Latest strict `capitalize.cpp` sidecar inventory delta evidence (run root: `/tmp/fragile_real_world_rapidjson_strict_capitalize_backend_surface_delta_3476060_1772238977899700230`):
  - baseline (`libclang`) inventory: `surface_line_count=39146`, `surface_placeholder_count=56`, `surface_rapidjson_placeholder_count=2`, `surface_c_void_alias_count=172`, `surface_parse_unspecific_count=18`.
  - LibTooling inventory delta vs baseline: `surface_line_count_delta_vs_baseline=-34053`, `surface_placeholder_delta_vs_baseline=-44`, `surface_rapidjson_placeholder_delta_vs_baseline=-2`, `surface_c_void_alias_delta_vs_baseline=0`, `surface_parse_unspecific_delta_vs_baseline=-1`.
- RapidJSON with `RAPIDJSON_BUILD_TESTS=ON` is not yet supported in strict mode (configure fails during CXX feature detection / gtest `target_compile_features`).
- Authoritative status and blocker ledger live in `TODO.md` (not `docs/transpiler-status.md`).

## Goal and Development Philosophy

The goal is a correct, general C++ compiler path via transpilation to Rust source.

Guidelines:
- Prefer root-cause transpiler fixes (parse/type/codegen) over one-off hacks.
- Use real-world fixtures as bug drivers.
- Keep strict-mode behavior deterministic and logged.
- Temporary constrained fallbacks are acceptable only when:
  - narrow in scope,
  - regression-tested,
  - tracked in `TODO.md` and real-world harnesses.

## Do Not Use Rustc Internals

Do not introduce:
- rustc private crates (`rustc_driver`, `rustc_interface`, etc.),
- MIR injection/conversion paths,
- custom rustc query overrides.

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `fragile-clang` | Clang AST parsing + Rust code generation |
| `fragile-cli` | `fragile` and `fragilec` binaries |
| `fragile-build` | Build config parsing |
| `fragile-common` | Shared utilities |
| `fragile-runtime` | Runtime helpers used by generated code |
| `fragile-ast-exporter` | AST exporter utilities |
| `fragile-stl` | Hand-written STL stub implementations (string, vector, map, etc.) |

## Build Commands

### Prerequisites
- Rust 1.75+
- LLVM/libclang 19-compatible

```bash
sudo apt install libclang-dev libclang-cpp-dev llvm-dev
export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu
```

### Build

```bash
# Workspace
cargo build

# Build strict compiler driver
cargo build -p fragile-cli --bin fragilec
```

## Test Commands

```bash
# Full workspace
cargo test --workspace

# RapidJSON harness (local + non-ignored)
cargo test -p fragile-clang --test real_world_rapidjson_tests -- --nocapture

# RapidJSON real-world ignored matrix (long-running)
cargo test -p fragile-clang --test real_world_rapidjson_tests -- --ignored --nocapture --test-threads=1

# Single strict CMake no-tests capture replay
cargo test -p fragile-clang --test real_world_rapidjson_tests \
  test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure \
  -- --ignored --nocapture --test-threads=1
```

## CLI Usage

```bash
# Transpile C++ to Rust
fragile transpile file.cpp -o output.rs

# With includes/defines
fragile transpile file.cpp -I /path/to/include -DMACRO=1 -o output.rs

# Enable LibTooling method-body enrichment
fragile transpile file.cpp --use-libtooling -o output.rs
```

```bash
# strict compile-only
FRAGILEC_MODE=strict ./target/debug/fragilec -c file.cpp -o file.o

# strict link
FRAGILEC_MODE=strict ./target/debug/fragilec file.o -o app

# driver help
./target/debug/fragilec --fragilec-help
```

## RapidJSON Working Commands

```bash
# Configure (tests off)
CXX=/home/shuai/workspace/fragile/target/debug/fragilec FRAGILEC_MODE=strict \
  cmake -DRAPIDJSON_BUILD_TESTS=OFF ..

# Build
CXX=/home/shuai/workspace/fragile/target/debug/fragilec FRAGILEC_MODE=strict \
  cmake --build . -j4
```

## Key Files

- `crates/fragile-clang/src/parse.rs` - parser + diagnostic handling
- `crates/fragile-clang/src/types.rs` - C++ type normalization/mapping
- `crates/fragile-clang/src/ast_codegen.rs` - AST -> Rust codegen + fallback surfaces
- `crates/fragile-cli/src/main.rs` - `fragile` CLI
- `crates/fragile-cli/src/bin/fragilec.rs` - strict compiler-driver shim
- `crates/fragile-clang/tests/real_world_rapidjson_tests.rs` - RapidJSON harness
- `TODO.md` - authoritative active status and blocker ledger
- `docs/dev/plan_rapidjson_strict.md` - strict RapidJSON plan and scope
