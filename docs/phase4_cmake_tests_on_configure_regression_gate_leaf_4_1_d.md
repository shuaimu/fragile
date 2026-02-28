# Phase 4.1.d: default-configure regression gate lock

## Scope
- Phase 4 hardening (`P1`), leaf `4.1.d`.
- Goal: convert the tests-on real-world capture lane from observational status/class coherence into a hard regression gate.

## Change
- Tightened ignored test:
  - `test_real_world_rapidjson_strict_cmake_tests_on_configure_capture`
- New required assertions:
  - `configure_status == 0`
  - `configure_failure_class == none`
  - configure stdout does not include `CXX compiler identification is unknown`
  - manifest encodes success (`configure_status=0`, `configure_failure_class=none`)

## Evidence
- Re-ran:
  - `cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_cmake_tests_on_configure_capture -- --ignored --nocapture --test-threads=1`
- Result: pass.
- Captured manifest:
  - `configure_status=0`
  - `configure_failure_class=none`
