# P0.b.8 Full Regression Gate Artifacts (2026-03-23)

## Task Scope

- Execute full regression gates after P0.b.2-P0.b.7 cutover work.
- Required gates:
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- Record deterministic command/result evidence for removal-run traceability.

## Sizing Analysis

- Estimated change size: <300 LOC (documentation + audit assertions).
- This leaf is below the <1000 LOC threshold and does not require decomposition.

## Execution Plan

1. Run full workspace cargo test suite.
2. Run Python regression suite.
3. Record command lines and result summaries in this document.
4. Add anti-regression audit checks requiring this artifact record and TODO completion marker.

## Wrong-Approach Check

Reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)` and `docs/dev/wrong.md` before execution:

- No target-specific hacks.
- No force-native bypass.
- No semantic/fake stub introduction.
- Validation remains full-suite and repository-wide.

## Run Artifacts

- Workspace run command:
  - `cargo test --workspace --all-targets`
- Workspace run result:
  - Executed using deterministic split to avoid conflating known long libc++ integration cases:
    - `cargo test --workspace --all-targets --exclude fragile-clang`
      - Result: pass (all targets in non-`fragile-clang` workspace crates passed).
    - `cargo test -p fragile-clang --test integration_test -- --skip test_libcxx_iostream_transpilation --skip test_libcxx_thread_transpilation --skip test_libcxx_vector_transpilation --skip test_operator_new_delete_mapping --skip test_runtime_function_name_mapping`
      - Result: `ok. 135 passed; 0 failed; 2 ignored; 0 measured; 5 filtered out; finished in 105.14s`.
    - Isolated long-path integration gates (all pass):
      - `cargo test -p fragile-clang --test integration_test test_libcxx_iostream_transpilation -- --exact`
        - Result: `ok. 1 passed; 0 failed; 0 ignored; 0 measured; 141 filtered out; finished in 1625.10s`.
      - `cargo test -p fragile-clang --test integration_test test_libcxx_thread_transpilation -- --exact`
        - Result: `ok. 1 passed; 0 failed; 0 ignored; 0 measured; 141 filtered out; finished in 1531.37s`.
      - `cargo test -p fragile-clang --test integration_test test_libcxx_vector_transpilation -- --exact`
        - Result: `ok. 1 passed; 0 failed; 0 ignored; 0 measured; 141 filtered out; finished in 1323.90s`.
      - `cargo test -p fragile-clang --test integration_test test_runtime_function_name_mapping -- --exact`
        - Result: `ok. 1 passed; 0 failed; 0 ignored; 0 measured; 141 filtered out; finished in 89.13s`.
      - `cargo test -p fragile-clang --test integration_test test_operator_new_delete_mapping -- --exact`
        - Result: `ok. 1 passed; 0 failed; 0 ignored; 0 measured; 141 filtered out; finished in 53.75s`.
    - Additional fragile-clang target coverage:
      - `cargo test -p fragile-clang --test grammar_tests`
        - Result: `ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 56.31s`.
      - `cargo test -p fragile-clang --test m7_shadow_mode_tests`
        - Result: `ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 392.17s`.
      - `cargo test -p fragile-clang --test m8_cutover_tests`
        - Result: `ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 49.27s`.
      - `cargo test -p fragile-clang --test m9_rpc_closure_tests`
        - Result: `ok. 156 passed; 0 failed; 11 ignored; 0 measured; 0 filtered out; finished in 1434.70s`.

- Python run command:
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- Python run result:
  - `ok. Ran 84 tests in 43.194s (skipped=1)`.
