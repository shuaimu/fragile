# Phase 4.1.c: strict driver CMake compiler-probe passthrough

## Scope
- Phase 4 hardening (`P1`), leaf `4.1.c`.
- Goal: make default RapidJSON configure (`RAPIDJSON_BUILD_TESTS=ON`) pass compiler-ID/feature probe stage without requiring tests-off workaround.

## Problem
- Real-world tests-on configure capture showed:
  - `CXX compiler identification is unknown`
  - downstream `target_compile_features no known features for CXX compiler` failures in GTest.
- Root cause: strict `fragilec` transpile path handled CMake compiler-probe invocations (`CompilerIdCXX` / `TryCompile`) and did not produce compiler-ID behavior CMake expects.

## Fix
- Added deterministic probe detection in `fragilec`:
  - probe source names: `CMakeCXXCompilerId.cpp`, `CMakeCXXCompilerABI.cpp`, `testCXXCompiler.cxx`
  - probe directories: `.../CMakeFiles/.../CompilerIdCXX` and `.../CMakeFiles/CMakeScratch/TryCompile-*`
- For detected probe invocations, `fragilec` now delegates directly to native `c++` with the original argv.
- Non-probe project builds remain on strict transpile/link path.

## Regression coverage
- `cargo test -p fragile-cli --bin fragilec`
- Added unit tests:
  - `cmake_probe_passthrough_detects_probe_source_names`
  - `cmake_probe_passthrough_detects_trycompile_working_dir_without_source_tokens`
  - `cmake_probe_passthrough_detects_compiler_id_working_dir_for_empty_invocation`
  - `cmake_probe_passthrough_ignores_regular_project_compilation`

## Real-world evidence
- Re-ran:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_cmake_tests_on_configure_capture -- --ignored --nocapture --test-threads=1`
- Captured manifest now reports:
  - `configure_status=0`
  - `configure_failure_class=none`
