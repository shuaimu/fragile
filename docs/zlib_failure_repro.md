# zlib failure reproducer command

Use this single command to reproduce the current Phase 1 zlib replay/parity failures in one place:

```bash
./scripts/repro_zlib_failure.sh
```

The wrapper runs this exact test command:

```bash
cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_command_subset_replay -- --ignored --nocapture --test-threads=1
```

Why this command:
- It executes the strict required-link replay plus make-test command replay path in one run.
- It fails immediately on the first deterministic failing stage with log pointers.
- It emits deterministic artifacts under `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs`.

Quick usage:
1. Run `./scripts/repro_zlib_failure.sh`.
2. If it fails, inspect `.status`, `.stdout`, and `.stderr` files in `/tmp/fragile_real_world_zlib_make_test_replay/driver_logs`.
3. Re-run the same command after each fix to verify regression closure.
