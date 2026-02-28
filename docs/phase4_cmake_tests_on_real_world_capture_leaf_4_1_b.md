# Phase 4.1.b: real-world tests-on configure capture lane

## Scope
- Phase 4 top hardening item (`P1`), leaf `4.1.b`.
- Goal: add an ignored real-world RapidJSON strict CMake capture lane for default configure path (`RAPIDJSON_BUILD_TESTS=ON`) with stable manifest artifacts.

## Design rationale
- Existing real-world strict CMake capture focused on `RAPIDJSON_BUILD_TESTS=OFF` build path.
- Existing local fixture (`4.1.a`) classifies configure failures deterministically but does not capture real RapidJSON configure output.
- This leaf adds a real-world capture lane without asserting current success/failure, so later `4.1.c/4.1.d` can tighten behavior gates.

## Implementation
- Added runner:
  - `run_rapidjson_strict_cmake_tests_on_configure_capture`
  - clones pinned RapidJSON checkout and runs strict CMake configure with `RAPIDJSON_BUILD_TESTS=ON`.
- Added stable artifacts under:
  - `/tmp/fragile_real_world_rapidjson_strict_cmake_tests_on_configure/strict_cmake_tests_on_configure_logs`
  - files: `cmake_configure.{status,stdout,stderr}`, `fragilec_driver.log`, `configure_failure_class.txt`, `strict_cmake_tests_on_configure_manifest.txt`.
- Added ignored real-world test:
  - `test_real_world_rapidjson_strict_cmake_tests_on_configure_capture`
  - validates artifact presence and status/class coherence.
- Added nightly coverage hook:
  - `.github/workflows/rapidjson-nightly.yml` matrix now includes `test_real_world_rapidjson_strict_cmake_tests_on_configure_capture`.

## Validation
- `cargo test -p fragile-clang --test real_world_rapidjson_tests`
- `cargo test`
