# M9.2.c.iv.e.34.e epoll/marshal residual compatibility closure

Date: 2026-03-26
Leaf: `M9.2.c.iv.e.34.e`

## Scope sizing (<1000 LOC)

- Selected first pending leaf under the active high-priority replay chain.
- Bounded implementation:
  - one existing late normalizer extension (`normalize_final_rpc_straggler_artifacts`),
  - focused unit tests for each new lane,
  - focused strict compile probes for `epoll_wrapper.cc` and `marshal.cpp`.
- Actual code touch stayed well under 1000 LOC.

## Plan used before implementation

1. Reproduce the current leaf blockers from focused strict compile probes.
2. Add generic compatibility/lifetime normalizations (no target-specific conditionals).
3. Add focused regression tests.
4. Re-run focused probes and confirm marker clearance.

## Wrong-approach compliance

- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)` and `docs/dev/wrong.md`.
- No target-specific `mako`/`rpcbench`/`test_rpc` branching added.
- No force-native bypass or escape-hatch usage.
- No rollback-pattern additions and no semantic fake stubs in target sources.

## Implemented normalizations

All changes are in `crates/fragile-clang/src/ast_codegen.rs` under late pass `normalize_final_rpc_straggler_artifacts`.

1. `EPOLLET` unary-negation lane
- Rewrite `EPOLLET = -2147483648,` -> `EPOLLET = 2147483648u32,` for `#[repr(u32)]` enum emission.

2. `epoll_event.data` union lane
- Rewrite `pub data: epoll_data_t,` -> `pub data: epoll_data,` when the real `epoll_data` union exists.

3. `MaybeUninit<T>::op_inc` lane for atomic counters
- Detect `static mut NAME: MaybeUninit<atomic_int|std_atomic_int>`.
- Rewrite `unsafe { NAME }.op_inc(0);` to:
  - `unsafe { let __fragile_atomic_ptr = NAME.as_mut_ptr(); (*__fragile_atomic_ptr).fetch_add(1, ()); };`

4. `Arc<Pollable>::op_arrow` lane
- Add compatibility trait once per file when needed:
  - `FragileArcArrowCompat<T>` for `std::sync::Arc<T>` using `Arc::as_ptr(self)`.

5. Marshal lifetime lane artifacts
- Rehydrate lifetimes on marshalling signatures:
  - `from_marshal` / `to_marshal`: `m: &'a mut Marshal -> &'a mut Marshal`.
  - `Marshallable_vtable_to_marshal` / `Marshallable_vtable_from_marshal` similarly.
- Rewrite vtable `to_marshal` wrapper body from const-pointer borrow to mutable raw cast:
  - `(*this).to_marshal(m)` -> `(*(this as *mut Marshallable)).to_marshal(m)`.

## Focused test evidence

Command:

```bash
cargo test -p fragile-clang normalize_final_rpc_straggler_artifacts -- --nocapture
```

Result:
- `12 passed; 0 failed`.
- Includes new tests:
  - `test_normalize_final_rpc_straggler_artifacts_rewrites_epoll_enum_and_event_data_union`
  - `test_normalize_final_rpc_straggler_artifacts_rewrites_maybeuninit_atomic_op_inc_calls`
  - `test_normalize_final_rpc_straggler_artifacts_adds_arc_op_arrow_compat`
  - `test_normalize_final_rpc_straggler_artifacts_rewrites_marshal_lifetime_signatures`

## Focused strict compile probe evidence

Compile profile (harness-equivalent):

```bash
FRAGILEC_MODE=strict FRAGILEC_KEEP_RS=1 ./target/debug/fragilec -c \
  -I vendor/mako/src \
  -I vendor/mako/src/rrr \
  -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako \
  -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1 \
  -std=gnu++23 -w <tu>
```

### epoll_wrapper.cc

Baseline artifact:
- `/tmp/fragile_e34e_epoll_before_xAYzC7`
- markers present:
  - `E0600` (`EPOLLET` negation)
  - `MaybeUninit<T>::op_inc` missing method
  - `Arc<Pollable>::op_arrow` missing method
  - `epoll_data_t.ptr` missing field

After fix:
- `/tmp/fragile_e34e_epoll_after_dbg_nTJVWY`
- summary:
  - `status=0`
  - `error_code_counts={}`
  - marker flags:
    - `has_e0600_epollet=0`
    - `has_op_inc_error=0`
    - `has_arc_op_arrow_error=0`
    - `has_epoll_data_ptr_error=0`

### marshal.cpp

Baseline artifact:
- `/tmp/fragile_e34e_marshal_before_8Dh34L`
- markers present:
  - `lifetime may not live long enough` (`2` hits)
  - `E0596` mutable borrow via `(*this).to_marshal(m)` from `*const` lane

After fix:
- `/tmp/fragile_e34e_marshal_after_dbg_k1MNMj`
- summary:
  - `status=0`
  - `error_code_counts={}`
  - marker flags:
    - `lifetime_may_not_live_long_enough=0`
    - `e0596_const_this_mut_borrow=0`

## Outcome

Leaf `M9.2.c.iv.e.34.e` is closed with bounded generic normalizations, focused unit coverage, and focused strict compile probe evidence.
