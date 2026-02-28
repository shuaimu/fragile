# Phase 4.1.a: local tests-on configure failure fixture/classification

## Scope
- Phase 4 top item (`P1`): harden CMake compiler-ID/feature checks for default RapidJSON configure path.
- Executed leaf: `4.1.a` local deterministic coverage only (no product fix yet).

## Design rationale
- The existing strict CMake local fixture only exercised build-time first-failure capture (`RAPIDJSON_BUILD_TESTS=OFF`).
- Phase 4 needs explicit visibility into configure-time failures that happen before build (compiler-ID / simple-test probes).
- Added a separate local fixture that intentionally fails CMake compiler checks during configure, then classifies the configure failure family.

## What was added
- New local fixture runner for `RAPIDJSON_BUILD_TESTS=ON` configure replay:
  - `run_local_strict_cmake_tests_on_configure_failure_capture_fixture`
  - fake `CXX` wrapper fails on `CMakeCXXCompilerId.cpp` / `CMakeCXXCompilerABI.cpp` / `testCXXCompiler.cxx`.
- New configure-failure classifier:
  - `classify_cmake_configure_failure`
  - class families: `none`, `cmake_compiler_check_failed`, `cmake_missing_dependency_or_compiler`, `other_configure_error`.
- New fixture artifact set:
  - `cmake_configure.{status,stdout,stderr}`
  - `fragilec_driver.log`
  - `configure_failure_class.txt`
  - `strict_cmake_tests_on_configure_local_fixture_manifest.txt`

## Regression coverage
- `test_classify_cmake_configure_failure_covers_known_error_families`
- `test_rapidjson_strict_cmake_tests_on_configure_local_fixture_classifies_compiler_check_failure`

## Evidence
- `cargo test -p fragile-clang --test real_world_rapidjson_tests`
  - Result: pass (new tests included).
