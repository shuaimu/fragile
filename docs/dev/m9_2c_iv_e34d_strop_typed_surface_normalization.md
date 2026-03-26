# M9.2.c.iv.e.34.d - strop.cpp Typed-Surface Normalization

Date: 2026-03-26

## Task sizing analysis
- Target leaf: `M9.2.c.iv.e.34.d`.
- Scope is bounded to post-processing normalization and compat-surface completion for `strop.cpp` typed blockers.
- Estimated and actual change size: small (<1000 LOC).

## Plan
1. Reproduce `strop.cpp` strict blockers with harness-equivalent compile flags and capture dominant markers.
2. Add bounded generic normalizations for swap/degraded-lane artifacts and malformed string-stream call patterns.
3. Rehydrate missing method surfaces on generated `std_string`/`std_ostringstream` types only when absent.
4. Add focused unit tests for each normalization family.
5. Re-run focused strict compile until the `strop.cpp` error set is clean.

## Wrong-Approach Check
- Re-read `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md` before implementation.
- No rollback-pattern expansion.
- No force-native fallback.
- No semantic type remapping (`std::*` to Rust std types).
- No target-file-only conditional rewrites; all passes are generic string/code-shape normalizations.

## Baseline blockers (`strop.cpp`)
Focused strict compile baseline run root:
- `/tmp/fragile_e34d_strop_compile_before_lW9Xpf`

Observed typed blockers:
- `error_code_counts={'E0425': 3, 'E0277': 4, 'E0308': 6, 'E0599': 14}`
- `cannot find type \`void\``: 3
- missing methods: `op_add_assign`, `reserve`, `op_shl`, `precision_1`, `find`, `find_first_not_of`
- `trait bound \`c_void: Default\``: 2
- malformed cast-call shape: `as *const i8.clone()`
- `exception_ptr::new_1(Default::default())` and `&mut &mut exception_ptr` artifacts

## Implementation
File touched:
- `crates/fragile-clang/src/ast_codegen.rs`

Added/updated bounded passes:
- `normalize_swap_template_stub_bodies`
  - rewrites degraded pointer swap stubs that used `Default::default()`/`void` into guarded `std::ptr::swap` bodies with `std::ffi::c_void` lane normalization.
- `normalize_rpc_string_stream_usage_artifacts`
  - repairs malformed cast-call tokens (`as *const i8.clone()`),
  - rewrites string-literal pointer assignments into `std_string` constructor lanes,
  - normalizes degraded `exception_ptr` assignment/call forms,
  - rewrites `swap_void(&mut __x.__ptr_, &mut __y.__ptr_)` to `std::mem::swap(...)`,
  - adds bounded numeric-cast normalization in known `size()` comparison/arithmetic patterns.
- `append_std_string_stream_compat_stubs`
  - appends missing `std_string` methods only when absent: `reserve(i32)`, `op_add_assign<T>`, `op_eq<T>`, `find(i8,u64)`, `find_first_not_of(i8,u64)`.
  - appends missing `std_ostringstream` methods only when absent: `precision_1(i64)`, `op_shl<T>`.

## Unit-test evidence
New tests in `ast_codegen.rs`:
- `test_append_std_string_stream_compat_stubs_adds_missing_methods`
- `test_normalize_swap_template_stub_bodies_rewrites_default_swap_stubs`
- `test_normalize_rpc_string_stream_usage_artifacts_rewrites_strop_patterns`

Focused command checks:
```bash
cargo test -p fragile-clang append_std_string_stream_compat_stubs -- --nocapture
cargo test -p fragile-clang normalize_swap_template_stub_bodies -- --nocapture
cargo test -p fragile-clang normalize_rpc_string_stream_usage_artifacts -- --nocapture
```

Result: pass.

## Focused strict replay evidence (`strop.cpp`)
Harness-equivalent command profile:
```bash
FRAGILEC_MODE=strict FRAGILEC_KEEP_RS=1 ./target/release/fragilec -c \
  -I vendor/mako/src \
  -I vendor/mako/src/rrr \
  -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako \
  -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1 -std=gnu++23 -w \
  vendor/mako/src/rrr/base/strop.cpp \
  -o /tmp/fragile_e34d_strop_compile_after3b_WdYN31/strop.cpp.o
```

Run roots:
- Baseline: `/tmp/fragile_e34d_strop_compile_before_lW9Xpf`
- Intermediate: `/tmp/fragile_e34d_strop_compile_after2_7xsL8U`
- Final: `/tmp/fragile_e34d_strop_compile_after3b_WdYN31`

Delta summary:
- Baseline: `E0425=3`, `E0277=4`, `E0308=6`, `E0599=14`
- Intermediate: only `E0308=1` (residual `swap_void` mismatch)
- Final: `error_code_counts={}`

Conclusion: `e.34.d` closes the dominant `strop.cpp` typed mismatch/missing-surface cluster and removes the immediate build-lane blocker for this file.
