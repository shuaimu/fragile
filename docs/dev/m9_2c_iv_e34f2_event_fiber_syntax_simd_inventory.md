# M9.2.c.iv.e.34.f.2 event/fiber syntax + SIMD surface closure inventory

Date: 2026-03-26  
Leaf: `M9.2.c.iv.e.34.f.2`

## Scope sizing (<1000 LOC)

- `ast_codegen` normalization: one bounded rewrite pass for malformed fiber literals.
- `fragile-stl` file-header helpers: four `_mm_*` helper functions.
- Regression tests: focused `ast_codegen` + `fragile-stl` coverage and closure-contract tests.
- Net implementation size is bounded and does not require broad refactors.

## Wrong-approach check

- Re-reviewed:
  - `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
  - `docs/dev/wrong.md`
- No rollback-pattern expansion, no fake target-specific behavior stubs, and no force-native bypasses were added.
- `_mm_*` helper additions are generic shared preamble surface completion, not per-file rewrites.

## Design decision

Targeted signatures from `e.34.f.1` were:

1. malformed parser-output syntax:
  - `FiberContext { , ..Default::default() }`
  - `rrr::boost_coro_task_t::State { State::NEW }`
2. unresolved SIMD helper surfaces:
  - `_mm_set1_epi8`
  - `_mm_cmpeq_epi8`
  - `_mm_movemask_epi8`
  - `_mm_and_si128`

Chosen bounded fixes:

- `normalize_rpc_fiber_context_state_artifacts` in `ast_codegen`:
  - rewrites malformed fiber-context spread literals,
  - rewrites degraded state-enum struct-literal shape to enum variant form.
- `crates/fragile-stl/src/file_header.rs`:
  - added missing shared helper definitions for the four unresolved `_mm_*` symbols.

## Focused strict compile evidence

Harness-equivalent command shape used for both files:

```bash
FRAGILEC_MODE=strict FRAGILEC_KEEP_RS=1 ./target/release/fragilec -c <source> \
  -I vendor/mako/src -I vendor/mako/src/rrr -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1 -std=gnu++23 -w
```

Before/after run roots:

- event: `/tmp/fragile_e34f2_event_before_G7itaN` -> `/tmp/fragile_e34f2_event_after_dvuKne`
- fiber: `/tmp/fragile_e34f2_fiber_before_24R9G7` -> `/tmp/fragile_e34f2_fiber_after_Bfhem5`

Marker deltas:

- `event.cc`
  - typed rustc errors: `318 -> 311`
  - malformed `FiberContext { , ..Default::default() }` markers: `2 -> 0`
  - missing `_mm_*` function markers: `6 -> 0`
  - `E0223` (`State { State::NEW }` related lane): `1 -> 0`
- `fiber_impl.cc`
  - typed rustc errors: `278 -> 271`
  - malformed `FiberContext { , ..Default::default() }` markers: `2 -> 0`
  - missing `_mm_*` function markers: `6 -> 0`
  - `E0223` (`State { State::NEW }` related lane): `1 -> 0`

Post-fix generated output confirmation:

- state assignment shape becomes `__self.state_ = State::NEW;`
- malformed `FiberContext { , ..Default::default() }` no longer appears.
- `super::_mm_*` callsites still emit, but now resolve via shared helper preamble.

## Tests added/updated

- `crates/fragile-clang/src/ast_codegen.rs`
  - `test_normalize_rpc_fiber_context_state_artifacts_rewrites_malformed_literals`
  - `test_normalize_rpc_fiber_context_state_artifacts_noop_without_markers`
- `crates/fragile-stl/tests/compilation_tests.rs`
  - extended `sse_intrinsics_compile` to cover `_mm_set1_epi8`, `_mm_cmpeq_epi8`, `_mm_movemask_epi8`, `_mm_and_si128`.

## Residual scope

- `e.34.f.2` closed only syntax/SIMD helper surfaces.
- Remaining runtime-replay blockers are tracked under:
  - `M9.2.c.iv.e.34.f.3`
  - `M9.2.c.iv.e.34.f.4`
  - `M9.2.c.iv.e.34.f.5`
