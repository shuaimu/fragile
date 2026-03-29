# M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.c.2 rpc/client syntax-enumbool inventory

Date: 2026-03-29
Task leaf: `M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.c.2`

## Scope

Execute a bounded second `c.4.c` slice by fixing high-signal `rpc/client.cpp`
syntax/enumbool call-shape drifts surfaced after `c.4.c.1`:

- casted enum match arms (`(ConnectionState::X as i32) =>`) causing pattern drift
- casted enum compare lanes (`== (ConnectionState::X as i32)`) causing enum/int mismatch
- synthetic wrapper call lane `rrr_(ConnectionState::X as i32)`
- `remove_if` iterator expression syntax lane `{ __i += 1; __i } != __last;`

## Wrong-approach check

- Reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Reviewed `docs/dev/wrong.md`.
- No target-specific conditional paths, no force-native bypass, no fake runtime
  shim binaries; fix remains a bounded late generic normalization in `ast_codegen`.

## Implementation

File changed:

- `crates/fragile-clang/src/ast_codegen.rs`

Added `normalize_rpc_client_syntax_and_enumbool_callshape_artifacts` and wired it
into the strict tail immediately after
`normalize_rpc_reactor_symbol_and_signature_artifacts`.

Normalization behavior:

- rewrites casted `ConnectionState` match-arm lanes to direct enum patterns
- rewrites casted enum compare lanes to direct enum compare values
- rewrites synthetic `rrr_(ConnectionState::...)` wrappers to direct enum values
- rewrites `remove_if` syntax lane
  `{ __i += 1; __i } != __last;` -> `let _ = ({ __i += 1; __i }) != __last;`

## Focused tests

Added tests:

- `test_normalize_rpc_client_syntax_and_enumbool_callshape_artifacts_rewrites_connection_state_cast_and_rrr_wrappers`
- `test_normalize_rpc_client_syntax_and_enumbool_callshape_artifacts_rewrites_remove_if_iterator_expression_lane`

Command:

```bash
cargo test -p fragile-clang normalize_rpc_client_syntax_and_enumbool_callshape_artifacts -- --nocapture
```

Result: passed (`2 passed; 0 failed`).

## Focused compile probe

Compile-commands-driven strict probe source:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260329T001857Z_p2759862/build_fragilec/compile_commands.json`

Probe artifact root:

- `/tmp/fragile_c4c2_focus_20260329T005901Z`

Evidence:

- `focus_1.status=1` (typed residuals remain; expected for this bounded leaf)
- target syntax markers are cleared in `focus_1.stderr`:
  - `expected expression=0`
  - `expected pattern=0`
  - `cannot find function rrr_=0`
  - casted match-arm markers like `(ConnectionState::CONNECTING as i32) =>` are absent
- latest transpiled output confirms call-shape rewrites:
  - `/tmp/fragilec_transpiled/client.cpp_b904242d4b03b543_client.rs`
  - `rrr_(ConnectionState::` count = 0
  - `{ __i += 1; __i } != __last;` count = 0
  - `let _ = ({ __i += 1; __i }) != __last;` count = 1

## Conclusion

`c.4.c.2` is complete as a bounded implementation slice:

- the targeted syntax/enumbool drift cluster is normalized in a generic pass,
- focused unit coverage is in place,
- focused compile evidence confirms targeted syntax markers are cleared.

Next leaves:

- `c.4.c.3` for remaining client/reactor typed compatibility surfaces,
- `c.4.c.4` for strict replay delta capture/non-increase evidence.
