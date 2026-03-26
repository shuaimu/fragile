# M9.2.c.iv.e.34.f.3 event/fiber container + surface closure inventory

Date: 2026-03-26  
Leaf: `M9.2.c.iv.e.34.f.3`

## Scope sizing (<1000 LOC)

- `ast_codegen` compatibility changes stayed in bounded late normalization passes.
- No parser architecture or runtime harness refactors were required.
- Added focused unit tests in `ast_codegen` for each targeted regression lane.
- Net touched implementation size remains well below the 1000-LOC task bound.

## Wrong-approach check

- Re-reviewed:
  - `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
  - `docs/dev/wrong.md`
- No target-specific `mako` conditionals were added.
- No force-native bypasses, fallback-path broadening, or fake semantic stubs were introduced.

## Design decision

From the `e.34.f.1`/`e.34.f.2` inventory, the dominant shared container/surface regressions in `event.cc` + `fiber_impl.cc` were:

1. unordered-map field-lane mismatch:
  - generated impl code called `.__tree_` / `__tree_:`
  - emitted struct lane uses `__table_`
2. degraded `basic_filebuf` shape:
  - empty struct emitted while methods referenced canonical field lanes
  - follow-on ctor/body artifacts surfaced (`codecvt` lane, bool lane, width/state lane)
3. pointer operator surfaces on shared ownership wrappers:
  - `.op_arrow()` / `.op_deref()` markers on `std::rc::Rc`, `std::cell::Ref`, `std::cell::RefMut`

Chosen bounded fixes:

- Added `normalize_rpc_container_surface_artifacts` at pipeline tail:
  - rewrite unordered-map impl `.__tree_` and `__tree_:` lanes to `.__table_` and `__table_:`,
  - rehydrate empty `basic_filebuf` with canonical field lanes,
  - normalize degraded ctor/body lane fragments,
  - append `FragileBasicFilebufCompat::setbuf` when `.setbuf(...)` markers are emitted.
- Extended `normalize_final_rpc_straggler_artifacts` compatibility surface:
  - `FragileRcArrowCompat`
  - `FragileCellRefArrowCompat`
  - `FragileCellRefDerefCompat`
  - `FragileCellRefMutDerefCompat`

## Focused strict compile evidence

Harness-equivalent strict command shape:

```bash
FRAGILEC_MODE=strict FRAGILEC_KEEP_RS=1 ./target/release/fragilec -c <source> \
  -I vendor/mako/src -I vendor/mako/src/rrr -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1 -std=gnu++23 -w
```

Run roots:

- Baseline (post-f.2):
  - event: `/tmp/fragile_e34f2_event_after_dvuKne`
  - fiber: `/tmp/fragile_e34f2_fiber_after_Bfhem5`
- After f.3:
  - event: `/tmp/fragile_e34f3_event_after_WWZSOj`
  - fiber: `/tmp/fragile_e34f3_fiber_after_qFvJjH`

Marker deltas:

- event
  - typed rustc errors: `311 -> 263`
  - unordered-map `__tree_` markers: `4 -> 0`
  - `basic_filebuf` E0560 marker cluster: `15 -> 0`
  - `setbuf` missing-surface marker: `1 -> 0`
  - Rc/Ref pointer-surface markers (`op_arrow`/`op_deref`): all cleared
- fiber
  - typed rustc errors: `271 -> 220`
  - unordered-map `__tree_` markers: `4 -> 0`
  - `basic_filebuf` E0560 marker cluster: `15 -> 0`
  - `setbuf` missing-surface marker: `1 -> 0`
  - Rc/Ref pointer-surface markers (`op_arrow`/`op_deref`): all cleared

Post-f.3 residuals newly surfaced (tracked for next leaves):

- `missing_hash_emplace_unique=4`
- map-tree constructor mismatch markers (`__tree_` ctor lane mismatch)=4

## Tests added/updated

- `crates/fragile-clang/src/ast_codegen.rs`
  - `test_normalize_rpc_container_surface_artifacts_rewrites_unordered_map_tree_field_lanes`
  - `test_normalize_rpc_container_surface_artifacts_rehydrates_basic_filebuf_lanes`
  - `test_normalize_final_rpc_straggler_artifacts_adds_rc_and_cell_ref_pointer_compat`

Focused test runs:

- `cargo test -p fragile-clang normalize_rpc_container_surface_artifacts -- --nocapture`
- `cargo test -p fragile-clang normalize_final_rpc_straggler_artifacts_adds_rc_and_cell_ref_pointer_compat -- --nocapture`

## Residual scope

- `e.34.f.3` closes shared container/surface lanes only.
- Remaining replay blockers continue under:
  - `M9.2.c.iv.e.34.f.4`
  - `M9.2.c.iv.e.34.f.5`
