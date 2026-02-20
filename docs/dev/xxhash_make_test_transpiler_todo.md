# xxHash `make test` Through Transpiler TODO

## Goal
- Run the logical scope of upstream `make test` against transpiled binaries, not only native `cc/g++` outputs.

## Completed
- [x] Stabilize real-world xxHash checkout reuse and local non-STL `sort.cc` patching in test setup.
- [x] Full xxhsum CLI transpile+compile runtime parity smoke for stdin and single-file input.
- [x] Add default hash/output-format runtime parity matrix (`default`, `--tag`, `--little-endian`).
- [x] Add `make check`-style command parity coverage:
  - stdin hashing
  - multiple file hashing
  - invalid-option failure parity
- [x] Fix short-option argument parsing parity in transpiled `xxhsum` for hash-selection paths (`-H#`).
- [x] Add `test-xxhsum-c` checksum generation parity coverage.
- [x] Add define-variant transpile+compile coverage for:
  - `XXH_NO_XXH3`
  - `XXH_NO_LONG_LONG`
  - `XXH_NO_STDLIB`
- [x] Fix benchmark option/runtime parity in transpiled `xxhsum` (`-b#`, `-i#`, `--benchmark-all`).
- [x] Fix check-mode runtime parity for `xxhsum -c` paths:
  - `xxhsum -c -` streamed checksums
  - `xxhsum -c <checksum-file>`
  - malformed-line and missing-file behavior
- [x] Fix `--filelist` parity in transpiled `xxhsum`.
- [x] Add a single orchestrated `make check` / `test-xxhsum-c` parity matrix harness
  (`test_real_world_xxhash_cli_make_check_and_test_xxhsum_c_matrix_matches_native`).
- [x] Add a drop-in `xxhsum` harness test proving upstream `make test` passes with transpiled runtime binary
  (`test_real_world_xxhash_make_test_passes_with_transpiled_xxhsum_dropin`).

## Remaining
- [ ] Build a drop-in `CC/CXX` driver mode (or wrapper) so upstream `make` can invoke transpilation directly.
- [ ] Add symbol-level parity checks equivalent to upstream `nm` assertions (`namespaceTest`, `c90test`, `noxxh3test`, `nostdlibtest`).
- [ ] Add transpiler-run parity for `test-tools` binaries (`tests/bench`, `tests/collisions`) beyond native build status.
- [ ] Promote high-value ignored parity tests into CI tiers (smoke vs nightly), with deterministic filtering for benchmark output.
