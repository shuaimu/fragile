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
- [ ] Capture native baseline with upstream flow (`./configure && make test`).
- [ ] Add transpiler driver path so `CC` in zlib build invokes Fragile flow.
- [ ] Transpile and compile all required zlib objects and test binaries (`example`, `minigzip`, shared/static variants used by `make test`).
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
