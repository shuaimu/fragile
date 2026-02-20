#!/usr/bin/env bash
set -euo pipefail

cmd=(
  cargo test -p fragile-clang --test real_world_zlib_tests
  test_real_world_zlib_make_test_command_subset_replay
  -- --ignored --nocapture --test-threads=1
)

if [[ "${1:-}" == "--print" ]]; then
  printf '%q ' "${cmd[@]}"
  printf '\n'
  exit 0
fi

"${cmd[@]}"
