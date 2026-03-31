# M9.2.c.iv.f.6.a deterministic unresolved-type invariant manifest

## Scope

- Leaf: `M9.2.c.iv.f.6.a`.
- Goal: capture a deterministic unresolved-type invariant blocker manifest from the f.5.e replay artifacts with exact signature counts and compile-unit mapping.
- Bounded scope: evidence extraction and documentation only (no codegen/normalization edits).

## Wrong-Approach Check

Reviewed before execution:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no force-native bypass,
- no target-specific conditional hacks,
- no semantic stubs.

## Execution Plan

1. Use f.5.e replay artifacts as the single source of truth.
2. Extract unresolved-type invariant signatures into a stable TSV manifest.
3. Re-run extraction and require identical pass1/pass2 output.
4. Map each signature to its failing compile unit/object marker from `build.stderr`.

## Artifact Inputs

- Replay root: `/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116`
- Manifest files:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116/strict_runtime_replay_manifest.txt`
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116/strict_runtime_replay_blocker_inventory_manifest.txt`
- Source stderr:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116/lane_fragilec/build.stderr`

## Deterministic Extraction

Command (run twice, compare outputs):

```bash
RUN_ROOT=/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116
STDERR="$RUN_ROOT/lane_fragilec/build.stderr"
PASS1=/tmp/fragile_f6a_unresolved_manifest_pass1.tsv
PASS2=/tmp/fragile_f6a_unresolved_manifest_pass2.tsv

extract() {
  rg -n "fragile unresolved-type invariant failed" "$STDERR" \
    | sed -E 's@^([0-9]+):\[fragilec\] fragile unresolved-type invariant failed for .*/reactor/([a-z_]+\.cc): (.*)$@\2\t\3\t\1@' \
    | sort -k1,1 > "$1"
}

extract "$PASS1"
extract "$PASS2"
cmp -s "$PASS1" "$PASS2" && echo DIFF_STATUS=identical
wc -l "$PASS1" "$PASS2"
```

Determinism result:

- `DIFF_STATUS=identical`
- `PASS1_ROWS=4`
- `PASS2_ROWS=4`

Extracted rows (`compile_unit\tunresolved_symbol\tbuild_stderr_line`):

- `event.cc\trrr_Future_State\t1`
- `fiber_context_runtime.cc\trrr_Future_State\t4`
- `fiber_impl.cc\trrr_Future_State\t8`
- `quorum_event.cc\trrr_Future_State\t6`

## Exact Signature Counts and Compile-Unit Mapping

| compile_unit | unresolved_type_symbol | signature_count | build_stderr_line | gmake object marker |
| --- | --- | --- | --- | --- |
| `event.cc` | `rrr_Future_State` | `1` | `1` | `CMakeFiles/rrr.dir/src/rrr/reactor/event.cc.o` |
| `fiber_context_runtime.cc` | `rrr_Future_State` | `1` | `4` | `CMakeFiles/rrr.dir/src/rrr/reactor/fiber_context_runtime.cc.o` |
| `fiber_impl.cc` | `rrr_Future_State` | `1` | `8` | `CMakeFiles/rrr.dir/src/rrr/reactor/fiber_impl.cc.o` |
| `quorum_event.cc` | `rrr_Future_State` | `1` | `6` | `CMakeFiles/rrr.dir/src/rrr/reactor/quorum_event.cc.o` |

Aggregates for the unresolved-type invariant subset:

- `unresolved_invariant_signature_total=4`
- `unresolved_invariant_unique_compile_units=4`
- `unresolved_invariant_symbol_set={rrr_Future_State}`

## Replay Context Anchors

From `strict_runtime_replay_manifest.txt`:

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `blocker_first_error_key=error:fragilec:[fragilec] fragile unresolved-type invariant failed for /home/shuai/workspace/fragile/vendor/mako/src/rrr/reactor/event.cc: rrr_Future_State`

From `strict_runtime_replay_blocker_inventory_manifest.txt`:

- `rustc_error_total_count=12`
- `rustc_error_unique_count=12`
- `error_key_001_count=1`
- `error_key_002_count=1`
- `error_key_003_count=1`
- `error_key_004_count=1`

## Next Leaf

- Next execution leaf remains `M9.2.c.iv.f.6.b` (bounded unresolved-type rehydration slice, `<=300 LOC`).
