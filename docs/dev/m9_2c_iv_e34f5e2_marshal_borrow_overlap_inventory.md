# M9.2.c.iv.e.34.f.5.e.2 marshal `track_write_2` borrow-overlap closure inventory

Date: 2026-03-27  
Leaf: `M9.2.c.iv.e.34.f.5.e.2`

## Scope sizing (<1000 LOC)

- One bounded late-pass normalization update in `crates/fragile-clang/src/ast_codegen.rs`.
- One focused unit test for the borrow-overlap rewrite shape.
- TODO + inventory documentation updates only.
- Implementation stayed below 1000 LOC.

## Wrong-approach check

Re-reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Conformance:

- no target-specific `mako`/`rpcbench`/`test_rpc` conditionals,
- no force-native bypass,
- no rollback and no fake-success stubs.

## Baseline blocker signature

From replay inventory leaf `e.1`:

- run-root: `/tmp/fragile_m9_2_strict_runtime_replay_20260327T064414Z_p402022`
- failing signature in `lane_fragilec/build.stderr`:
  - `error[E0499]: cannot borrow self.kind_ as mutable more than once at a time`
  - line shape: `self.track_write_2(... ((&mut self.kind_ as *mut i32) as *const _ as *const ()), ...)`

## Design and implementation

Updated `normalize_rpc_marshal_fiber_context_artifacts` with a bounded rewrite for
`track_write_2` field-pointer callshape artifacts:

- detect `self.track_write_2(...)` calls that pass field pointers as
  `((&mut self.FIELD as *mut T) as *const _ as *const ())`,
- rewrite to a hoisted temporary raw pointer before call:

```rust
{ let __fragile_track_write_ptr: *const () = std::ptr::addr_of_mut!((*self).FIELD) as *const ();
  self.track_write_2(..., __fragile_track_write_ptr, ...)
}
```

This keeps behavior and argument order intact while removing receiver/field mutable-borrow overlap (`E0499`).

## Regression coverage executed

Focused unit command:

```bash
cargo test -p fragile-clang --lib normalize_rpc_marshal_fiber_context_artifacts -- --nocapture
```

Focused assertion coverage includes:

- `test_normalize_rpc_marshal_fiber_context_artifacts_hoists_track_write_field_pointer_before_call`

## Focused strict marshal probe evidence

Harness-equivalent strict compile command:

```bash
FRAGILEC_MODE=strict ./target/release/fragilec -c \
  -I vendor/mako/src \
  -I vendor/mako/src/rrr \
  -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako \
  -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1 -std=gnu++23 -w \
  vendor/mako/src/rrr/misc/marshal.cpp \
  -o /tmp/fragile_e34f5e2_marshal_compile_after_20260327T082121Z_p490835/marshal.cpp.o
```

Run-root:

- `/tmp/fragile_e34f5e2_marshal_compile_after_20260327T082121Z_p490835`

Observed marker deltas:

- `E0499` count: `0`
- `cannot borrow self.kind_` marker count: `0`
- residual typed errors: `5` (non-`E0499` families: `E0282`, `E0308`, `E0599`)

Transpiled output marker confirms rewrite shape:

- `/tmp/fragilec_transpiled/marshal.cpp_023fd4199abd24b2_marshal.rs`
- contains: `let __fragile_track_write_ptr: *const () = std::ptr::addr_of_mut!((*self).kind_) as *const ();`

## Residual scope

This closes `M9.2.c.iv.e.34.f.5.e.2`.

Remaining leaves under `e.34.f.5.e`:

- `M9.2.c.iv.e.34.f.5.e.3`
- `M9.2.c.iv.e.34.f.5.e.4`
- `M9.2.c.iv.e.34.f.5.e.5`
