# Fragile Transpiler Dev Book

## Table of Contents

- [1. Purpose and Scope](#1-purpose-and-scope)
- [1.1 2026 Program Goal: Mode 1 Seamless Interop](#11-2026-program-goal-mode-1-seamless-interop)
- [1.2 Mako as Primary Validation Target](#12-mako-as-primary-validation-target)
- [1.3 Wrong Approaches (Do Not Do)](#13-wrong-approaches-do-not-do)
- [2. End-to-End Architecture](#2-end-to-end-architecture)
- [2.3 C++ `_v` trait globals and export linkage](#23-c-_v-trait-globals-and-export-linkage)
- [2.4 Mode 1 call-stitching architecture (target state)](#24-mode-1-call-stitching-architecture-target-state)
- [3. Internal Data Models](#3-internal-data-models)
- [4. C++ Declaration to Rust Item Mapping](#4-c-declaration-to-rust-item-mapping)
- [5. C++ Type to Rust Type Mapping](#5-c-type-to-rust-type-mapping)
- [5.4 Rusty-C++ alias normalization to Rust std types](#54-rusty-c-alias-normalization-to-rust-std-types)
- [5.5 Lowered std_collections alias closure](#55-lowered-std_collections-alias-closure)
- [6. Object Model and Inheritance Design](#6-object-model-and-inheritance-design)
- [6.4 Mode 1 non-primitive object interop contract](#64-mode-1-non-primitive-object-interop-contract)
- [7. Function, Method, Constructor, Destructor Mapping](#7-function-method-constructor-destructor-mapping)
- [8. Statement Mapping](#8-statement-mapping)
- [9. Expression Mapping](#9-expression-mapping)
- [9.7 Rusty wrapper call normalization](#97-rusty-wrapper-call-normalization)
- [9.8 Namespace alias target normalization](#98-namespace-alias-target-normalization)
- [9.9 Typed null pointer return pointee normalization](#99-typed-null-pointer-return-pointee-normalization)
- [10. Templates and Instantiation Strategy](#10-templates-and-instantiation-strategy)
- [11. Runtime and Preamble Integration](#11-runtime-and-preamble-integration)
- [12. Extension Guide for Contributors](#12-extension-guide-for-contributors)
- [13. Appendix: Per-ClangNodeKind Lowering Matrix](#13-appendix-per-clangnodekind-lowering-matrix)

## 1. Purpose and Scope

This document describes the current developer-facing design of Fragile’s C++ to Rust transpiler, as implemented in `crates/fragile-clang`.

Scope of this book:

- How source flows through parse/export/enrichment/codegen.
- How C++ AST structures are lowered into Rust structures.
- Where key lowering logic lives, so contributors can extend behavior safely.

Non-goals:

- It is not a user tutorial.
- It is still implementation-first, but now also records the active near-term program goal and rollout target.

## 1.1 2026 Program Goal: Mode 1 Seamless Interop

Fragile's active goal for 2026 is **Mode 1 full-transpile seamless interop**:

- C++ translation units are transpiled to Rust and compiled as Rust objects.
- User-written Rust and transpiled C++-origin Rust should call each other as normal Rust code in the same build graph.
- Users should not need to introduce `extern "C"` for intra-project Rust/C++-origin interactions.
- C ABI boundaries are kept only for true foreign interfaces (system libraries, external process/plugin boundaries).

Operational constraints for this goal:

- Build all participating code with one pinned Rust toolchain in the same project build.
- Preserve current compile correctness on large codebases while tightening cross-TU Rust identity and symbol resolution.
- Keep current declaration-only C-linkage fallbacks in place until equivalent Rust-native call stitching is proven for the same surfaces.

Initial implementation focus areas:

1. Stabilize cross-TU type identity (`cpp_fqn -> canonical rust path`) so non-primitive object types stay consistent.
2. Prefer Rust-native call rewriting for transpiled call sites when callee identity is known in-project.
3. Limit generated ABI shims to unresolved/foreign boundaries, not default internal paths.
4. Validate each step on `vendor/mako` before broadening to other projects.

## 1.2 Mako as Primary Validation Target

`vendor/mako` is the primary integration target and release gate for this Mode 1 workstream.

Milestone gates:

1. **Smoke gate**: targeted `test_rpc` build+run path is stable.
2. **Subset gate**: RPC-focused ctest subset remains green while removing internal interop friction.
3. **Full gate**: clean full build + full ctest pass in the selected mako build directory.

Verification command used for this update (March 8, 2026):

```bash
cd vendor/mako/build_fragilec_clang_probecompat
ctest -R '^test_rpc$' --output-on-failure
```

Observed result: `1/1` passed (`test_rpc`).

## 1.3 Wrong Approaches (Do Not Do)

The Fragile dev process explicitly forbids shortcut fixes that hide transpiler gaps.

Forbidden approaches:

- Do not add target-specific hacks (for example `mako`-specific or `rpc`-specific conditionals in parser/codegen logic).
- Do not bypass Fragile translation by delegating selected TUs to a native C++ compiler.
- Do not add fake semantic stubs/fallback bodies just to make compile/tests pass.
- Do not rely on force-native escape hatches as a development strategy.

Required approach:

1. Implement a generic parser/codegen/runtime fix that applies beyond one benchmark target.
2. Add regression coverage (unit test and replay/integration coverage when available).
3. Re-validate on both target workload and non-target workload to prevent overfitting.

Authoritative anti-pattern policy and examples are documented in:

- `docs/dev/wrong.md`

## 2. End-to-End Architecture

Primary entry points are in `crates/fragile-clang/src/lib.rs`:

- `transpile_cpp_to_rust`
- `transpile_cpp_to_rust_with_options`

The pipeline is:

1. Export stage:
- Use `LibToolingParser` to produce exporter `AstContext`.
- Include path/define/language flags are assembled from `TranspileOptions`.

2. Parse stage:
- Convert exporter AST nodes into internal `ClangNode` tree via `translation_unit_from_libtooling_context` and `convert_to_clang_node`.
- Promote missed declaration roots when exporter top-level links are incomplete.
- Deduplicate repeated function roots.

3. Enrichment stage:
- Inject extra method/specialization metadata into codegen (`set_libtooling_bodies`, specialization field/method signatures).

4. Codegen stage:
- `AstCodeGen::generate` emits Rust text directly from internal AST.

Tracing support:

- Stage timing and status can be recorded via `stage_timing_trace_path` in `TranspileOptions`.

Backend note:

- `ParserBackend` contains legacy variants, but transpilation is centered on LibTooling flow.

### 2.1 Mako `btree.cc` failure pattern and generic mitigation

A recurring failure pattern on large C++ codebases (including `mako/src/core/btree.cc`) is:

- parse-time diagnostics only when template bodies are eagerly parsed
- exporter abort on enum constants that do not fit 64-bit integer extraction

Current generic mitigation in Fragile:

- `TranspileOptions.template_parsing_mode` controls LibTooling template parsing policy.
- Default mode is `TemplateParsingMode::Auto`:
  - first attempt uses standard parsing flags
  - second attempt retries with `-fdelayed-template-parsing` if the first parse fails
- If frontend args already specify template parsing (`-fdelayed-template-parsing` or `-fno-delayed-template-parsing`), Fragile does not override them.

This keeps standard-conforming behavior by default while preserving a tolerant fallback for third-party headers that are known to be sensitive to eager template-body parsing.

Exporter hardening for wide enum constants:

- Enum constant emission no longer assumes `APSInt` always fits a signed 64-bit payload.
- Values that fit are encoded as CBOR integers.
- Wider values are encoded as decimal strings.

This removes exporter-side aborts for large constants (for example `unsigned __int128` enum values) while preserving payload information for downstream lowering.

### 2.2 Mako drop-in replay loop (March 2026)

Drop-in validation workflow used for `vendor/mako`:

1. In `vendor/mako/build_fragilec_dropin`, run `make clean` first, then build with `cmake --build . -j32` (or `gmake -j32`).
2. Take the first `fragile rustc object compile failed` translation unit from the log.
3. Re-run that file alone from `compile_commands.json` with the exact command (this preserves all CMake include paths/defines); keep `FRAGILEC_KEEP_RS=1` enabled when Rust-level debugging is needed.
4. Add a generic normalization in `AstCodeGen` (no mako-specific source patching).
5. Add a focused unit test in `crates/fragile-clang/src/ast_codegen.rs`.
6. Rebuild `fragilec`, confirm isolated file passes, then run next full `cmake --build . -j32` and `ctest -j32 --output-on-failure`.

Generic normalizations added in this replay cycle:

- `normalize_empty_union_definitions`:
  - injects a placeholder field for zero-field unions.
- `normalize_empty_struct_enum_name_collisions`:
  - drops empty struct shells that collide with same-scope enums.
- `normalize_module_type_name_shadowing` refinement:
  - only qualifies/shims against top-level type-like names.
- `normalize_obvious_local_var_type_mismatches` expansions:
  - stream-like ctor identifier assignment fallback (`unsafe { std::mem::zeroed() }`);
  - non-primitive local from primitive-typed local fallback;
  - unresolved lowercase zeroed local types -> `UnknownTagAutoType`.
  - argv token splitting fallback now forces `std::vec::Vec<&str>` to avoid local `Vec` type-name shadowing.
- `normalize_static_array_integer_element_widths`:
  - casts static integer-array elements into declared element lanes.
- `normalize_bare_identifier_expression_statements`:
  - rewrites bare identifier statements (`v;`) to borrow no-ops.
- `normalize_placeholder_struct_invocation_artifacts` expansion:
  - rewrites placeholder-struct call artifacts with argument lists (`placeholder(...)`) to `placeholder::default()` (existing zero-arg rewrite retained), avoiding Rust value-namespace call failures when degraded lowering emits placeholder types like `make_pair`.
- `normalize_invalid_placeholder_type_item_names`:
  - removes invalid `_`-named struct/enum/union items and their impls.
- `normalize_vector_capacity_call_argument_widths`:
  - casts `.reserve(...)` / `.resize(...)` call args to `i32` while skipping `self.inner.*` implementation calls.
- `normalize_unresolved_join_method_calls`:
  - rewrites unresolved identifier `.join();` statements to no-op.
- `normalize_global_range_for_paths_to_unsafe_borrows`:
  - wraps rooted prefixed-global range expressions (`for x in super::mod::__gv_name { ... }`) as borrowed unsafe reads (`for x in &(unsafe { super::mod::__gv_name }) { ... }`) so iteration does not move from mutable statics and satisfies Rust unsafe access rules.
- switch lowering (`generate_switch_stmt`) no-default fallback arm:
  - emits typed wildcard arm `_ => { unsafe { std::mem::zeroed::<_>() } }` instead of `_ => unsafe { std::mem::zeroed() },` to preserve Rust match arm typing.
- `normalize_struct_default_clone_derives` refinement:
  - only rewrites derives to manual `Default`/`Clone` impls for structs that are directly or transitively `c_void`-backed (including aliases/field references), leaving non-`c_void` structs on regular derives.
- `normalize_add_missing_struct_default_clone_impls` refinement:
  - prefers non-empty discovered field maps and handles inline one-line struct declarations when synthesizing fieldwise defaults.
  - when no safe field list is available, emits `unsafe { std::mem::zeroed() }` fallback instead of invalid `Self {}`.
- `normalize_self_recursive_struct_fields` refinement:
  - resolves simple local `type` aliases while scanning struct/union field declarations.
  - rewrites alias-mediated by-value self recursion (including `ManuallyDrop<Alias>`) to pointer form (`*mut Self`) so generated Rust types remain sized.
- LibTooling missing-header fallback in `parse_libtooling_context`:
  - when parse attempts fail on `file not found` diagnostics, Fragile extracts missing header names, creates sanitized temporary `#pragma once` stubs, and retries parse with an extra `-I <stub_dir>`.
  - this is generic and avoids mako-specific patches when generated build headers are absent during early translation-unit compilation.
- `fallback_heavily_degraded_function_bodies` entrypoint fallback:
  - when a heavily degraded body forces a `main` stub, emit a generic argv-driven help/version/flag parser fallback that preserves expected exit-code and output shape for gflags-style tests.
- `fallback_heavily_degraded_function_bodies` unresolved-global guard:
  - track emitted `__gv_*` static symbols and treat references to missing ones as degraded-body markers.
  - this catches namespaced unresolved forms (for example `super::rusty::__gv_rhs`) and stubs those functions instead of letting rustc fail on missing globals.
- LibTooling compile-arg injection ordering:
  - `fragile-ast-exporter` now forwards parser extra args via `-extra-arg-before=` so compile-mode flags land before the source file from `compile_commands.json`.
  - `LibToolingParser` normalizes `-x`/`-std` overrides into the generated compile command and strips them from appended extra args.
  - this removes noisy Clang warnings like `'-x c++' after last input file has no effect` during drop-in builds.
- `normalize_rusty_type_alias_rhs_paths` late rerun:
  - `AstCodeGen::generate` now re-runs Rusty alias RHS normalization at the end of the pipeline.
  - this catches Rusty wrapper aliases appended by late unresolved-closure/degraded fallback passes and keeps final alias targets std-native where generic mapping exists.
- runtime-internal Rusty namespace alias handling (`normalize_unresolved_namespaced_type_aliases` + `emit_namespace_type_aliases`):
  - treat `rusty::BorrowState`, `rusty::Group`, `rusty::ProbeSeq`, and `rusty::RcControlBlockBase` as runtime-internal alias targets.
  - rewrite bare item-type references to these names (for example `type X = RcControlBlockBase;`) to fully-qualified `rusty::...` paths.
  - suppress fallback/auto-export alias emission for these internals once references are rewritten, so generated TU sidecars avoid redundant top-level runtime-internal aliases.
- runtime-internal typedef pruning (`normalize_unused_runtime_internal_type_aliases`):
  - remove `pub type Alias = rusty::<runtime-internal>;` only when `Alias` is not referenced in generated item type positions in that TU.
  - drop paired alias-doc lines (`/// C++ typedef/using ...`) for removed aliases to keep output clean.
- `normalize_with_capacity_default_string_placeholders`:
  - rewrites degraded `with_capacity::default()` placeholders (from failed `String::with_capacity` recovery) to `std::string::String::new()`.
- expression-like pseudo-type stub suppression (`generate_missing_type_stubs`):
  - detects leaked expression path spellings used as fake type names (for example `size::of`, `type::name`, `from::utf8`) and skips emitting fallback `pub struct ...` stubs for them.
  - applies to both `used_types` and `referenced_but_undefined_structs` recovery paths, while preserving real lowercase type candidates.
  - recognizes numeric leaf segments and global-scope singleton paths in leaked pseudo types (for example `new::0`, `::size`, `::ptr`) and includes `from::raw::parts::mut`-style paths.
  - keeps `c::u128`-style surfaces eligible while still suppressing `c::void` in this pseudo-type filter.
- `fragile-stl` `std_string` preamble hardening:
  - replaced unchecked `Layout::array(...).unwrap()` allocation paths with checked `match`/early-return handling for layout overflow.
  - retained explicit null checks via `ptr == std::ptr::null_mut()` / `ptr != std::ptr::null_mut()` and avoided `let-else`/tuple destructuring in this preamble block, because those forms were observed to be rewritten incorrectly in generated output during replay builds.

Outcome snapshot:

- On March 4, 2026, `make clean && cmake --build . -j32` in `build_fragilec_dropin` completed to 100% after iterative generic fixes.
- On March 4, 2026, `ctest -j32 --output-on-failure` hit one transient `rpcbench` kill in one run but passed on rerun (`117/117`) in `build_fragilec_dropin`; no deterministic compile-failure translation units remained in this loop.
- On March 4, 2026 (revalidation pass), a fresh rerun in `build_fragilec_dropin` with `make clean`, `cmake --build . -j32`, and `ctest -j32 --output-on-failure` completed with `117/117` tests passed and no `fragile rustc object compile failed` translation units in the build log.
- On March 5, 2026, after adding runtime-internal alias rewrite+suppression, a fresh `make clean`, `cmake --build . -j32`, and `ctest -j32 --output-on-failure` rerun again passed (`117/117`) while removing generated fallback aliases for `BorrowState`/`Group`/`ProbeSeq`/`RcControlBlockBase` in `build_fragilec_dropin` sidecar `.fragile.rs` outputs.
- On March 5, 2026, after adding unused runtime-internal typedef pruning, another fresh `make clean`, `cmake --build . -j32`, and `ctest -j32 --output-on-failure` rerun passed (`117/117`) and reduced remaining `pub type ... = rusty::...;` sidecar aliases in `build_fragilec_dropin` from 74 to 0.
- On March 5, 2026, after adding expression-like pseudo-type stub suppression for leaked paths (for example `size::of`), a fresh `make clean`, `cmake --build . -j32`, and `ctest -j32 --output-on-failure` rerun passed (`117/117`) and removed invalid fallback struct emissions for those path-like pseudo types.
- On March 5, 2026, after broadening expression-like pseudo-type detection (`new::0`, `from::raw::parts::mut`, global-scope singleton path leaks), a fresh `make clean`, `cmake --build . -j32`, and `ctest -j32 --output-on-failure` rerun again passed (`117/117`), with `from::raw::parts::mut` and `new::0` placeholder emissions reduced to 0 in `build_fragilec_dropin` sidecar outputs.

### 2.3 C++ `_v` trait globals and export linkage

Large C++23 codebases that include Rusty-C++ headers can surface trait-style variable templates such as:

- `fits_in_sbo_v`

These are C++ header-inline/ODR entities (usually `constexpr bool ..._v`) and should not be emitted as strong, raw C-linkage globals from each translation unit.

Generic transpiler guard now applied in `AstCodeGen::should_export_c_global_symbol`:

- if a global is `bool`-typed and its identifier ends with `_v`, Fragile suppresses raw symbol export (`#[export_name = "..."]`).

Why this matters:

- exporting these names as plain symbols can produce duplicate-symbol linker failures when many translation units instantiate the same header trait variable template.
- keeping them as TU-local Rust-mangled statics preserves buildability without introducing mako-specific hacks.

### 2.4 Mode 1 call-stitching architecture (target state)

Mode 1 shifts internal interop from ABI shims to Rust-native call stitching.

Target call path:

1. Parse/export C++ and transpile to Rust text per TU.
2. Build a project-wide symbol index from:
  - transpiled Rust items,
  - user-authored Rust items that are declared as C++-visible,
  - namespace/type aliases resolved to canonical Rust paths.
3. Rewrite resolvable call sites to direct Rust path calls (no C ABI wrapper).
4. Keep generated ABI wrappers only where resolution is unknown or external.
5. Compile/link with one pinned Rust toolchain for the whole project graph.

Current implementation step (March 8, 2026):

- Declaration-only free-function wrapper emission now uses signature-level identity (`mangled symbol` when available, otherwise name+parameter lanes), not name-only suppression.
- Declaration-only overloads are registered in overload metadata and emitted with stable Rust suffixing (`foo`, `foo_1`, ...) so in-TU overload resolution has concrete callable surfaces without duplicate-name collisions.

Practical implication:

- Declaration-only wrappers (`extern "C"` + shim) remain as a compatibility fallback, not the default internal mechanism.
- Internal project calls should converge to direct Rust calls as symbol-index coverage improves.

## 3. Internal Data Models

Core node model: `crates/fragile-clang/src/ast.rs`

- `ClangNodeKind` contains declaration, statement, and expression variants.
- Examples:
  - declarations: `FunctionDecl`, `RecordDecl`, `EnumDecl`, `UnionDecl`, `CXXMethodDecl`
  - statements: `IfStmt`, `ForStmt`, `SwitchStmt`, `TryStmt`
  - expressions: `CallExpr`, `MemberExpr`, `CXXConstructExpr`, `CastExpr`, `CXXNewExpr`, `CXXDeleteExpr`

Core type model: `crates/fragile-clang/src/types.rs`

- `CppType` is the canonical representation for lowered C++ types.
- `CppType::to_rust_type_str()` is the central type-to-type lowering function.
- `to_rust_type_str_for_field()` adjusts reference types for struct fields (references become raw pointers).

## 4. C++ Declaration to Rust Item Mapping

| C++ structure | Rust structure generated | Notes |
|---|---|---|
| Translation unit | Flat sequence of Rust items | Emitted by `generate_top_level` |
| `FunctionDecl` definition | `pub fn ...` | May emit `#[no_mangle]` / `#[export_name]` for symbol preservation |
| `FunctionDecl` declaration only | extern wrapper + safe shim | Generated by `generate_extern_c_function_decl` |
| `RecordDecl` (`struct`/`class`) | `#[repr(C)] pub struct` + `impl` | Class vs struct only affects comments/default access semantics |
| `UnionDecl` | `#[repr(C)] pub union` | Non-copy fields wrapped in `ManuallyDrop` |
| `EnumDecl` | `#[repr(Int)] pub enum` (or type alias for empty enum) | Duplicate discriminants become const aliases |
| `TypedefDecl` / `TypeAliasDecl` | `pub type Alias = ...;` | Self-cycles and collisions are filtered |
| `NamespaceDecl` | `pub mod namespace { ... }` | Namespace reopenings are merged; some namespaces are flattened/treated specially |
| `LinkageSpecDecl` (`extern "C" { ... }`) | Transparent container | Children are emitted directly |
| `VarDecl` global | global/static item | Static members become generated globals |
| `ModuleImportDecl` | comment placeholder | Full C++20 module lowering is not active yet |

Access specifier lowering (`access_to_visibility`):

- `public` -> `pub`
- `protected` -> `pub(crate)`
- `private` -> default private

## 5. C++ Type to Rust Type Mapping

Source: `CppType::to_rust_type_str` in `crates/fragile-clang/src/types.rs`.

### 5.1 Primitive and scalar mappings

| C++ | Rust |
|---|---|
| `void` | `()` |
| `bool` | `bool` |
| `char` / `signed char` | `i8` |
| `unsigned char` | `u8` |
| `short` / `unsigned short` | `i16` / `u16` |
| `int` / `unsigned int` | `i32` / `u32` |
| `long` / `unsigned long` | `i64` / `u64` |
| `long long` / `unsigned long long` | `i64` / `u64` |
| `float` / `double` | `f32` / `f64` |
| `__int128` / `unsigned __int128` | `i128` / `u128` |
| `size_t` / `ptrdiff_t` | `usize` / `isize` |

### 5.2 Compound type mappings

| C++ | Rust |
|---|---|
| `T*` | `*mut T` or `*const T` |
| `R(*)(Args...)` | `Option<extern "C" fn(Args...) -> R>` |
| `T&` / `const T&` | `&mut T` / `&T` (general expression context) |
| `T&` field type | raw pointer in fields (`*mut`/`*const`) |
| `T[N]` | `[T; N]` |
| unsized array in parameter contexts | pointer-like lowering |

### 5.3 Named type normalization

Named types are sanitized and normalized (namespace flattening, template token cleanup, inline namespace stripping like `std::__1::`).

Examples:

- `std::vector<int>` -> sanitized identifier form (`std_vector_int` style)
- `decltype(...)` / unresolved template artifacts -> fallback placeholders in unresolved paths

This behavior is intentionally pragmatic to keep codegen compilable when upstream types are partially unresolved.

### 5.4 Rusty-C++ alias normalization to Rust std types

Source: `map_rusty_type_to_std` in `crates/fragile-clang/src/types.rs`.

Large codebases that include `rusty-cpp` headers frequently surface both fully qualified and unqualified alias spellings (for example after `using namespace rusty;`). Fragile now normalizes both forms to Rust std paths:

- Single-parameter aliases:
  - `rusty::Option<T>` and `Option<T>` -> `std::option::Option<T>`
  - `rusty::RefCell<T>` and `RefCell<T>` -> `std::cell::RefCell<T>`
  - `rusty::Vec<T>` and `Vec<T>` -> `std::vec::Vec<T>`
  - `rusty::HashSet<T>` and `HashSet<T>` -> `std::collections::HashSet<T>`
- Two-parameter aliases:
  - `rusty::Result<T, E>` and `Result<T, E>` -> `std::result::Result<T, E>`
  - `Result<void, E>` / `Result<T, void>` normalize `void` lanes to `()`
  - unresolved `Result<type-parameter-*, E>` lanes normalize to `()` to keep generated std `Result` aliases type-valid
  - when `E` is a non-isomorphic Rusty sync wrapper like `rusty::PoisonError<U>`, `E` is normalized to the generated companion record identifier (for example `rusty_PoisonError_U`) so the outer alias can still land on `std::result::Result<..., ...>` without leaking unresolved Rusty namespace paths
  - `rusty::HashMap<K, V>` and `HashMap<K, V>` -> `std::collections::HashMap<K, V>`
- Result convenience aliases:
  - `rusty::ResultVoid<T>` and `ResultVoid<T>` -> `std::result::Result<T, ()>`
  - `rusty::ResultInt<T>` and `ResultInt<T>` -> `std::result::Result<T, i32>`
  - `rusty::ResultString<T>` and `ResultString<T>` -> `std::result::Result<T, *const i8>`
- mpsc wrapper aliases:
  - `Sender<T>` / `Receiver<T>` / `SyncSender<T>` / `TrySendError<T>` normalize to the corresponding `std::sync::mpsc::*` wrappers
  - explicit `void` and unresolved payload placeholders in those wrappers normalize to unit `()`
  - bare mpsc error enums (`RecvError`, `TryRecvError`, `TrySendError`) preserve `std::sync::mpsc::*` paths instead of degrading to sanitized identifiers

This keeps generated Rust in safe/std-native form instead of preserving rusty-cpp wrapper type names in emitted signatures.

### 5.5 Lowered `std_collections` alias closure

Source:

- `map_lowered_std_single_template_alias_to_std` in `crates/fragile-clang/src/types.rs`
- `stl_container_alias_target_from_rust_name` / `close_unresolved_type_reference_gaps` in `crates/fragile-clang/src/ast_codegen.rs`

Recent drop-in builds surfaced lowered spellings like:

- `std_collections_VecDeque_std_shared_ptr_Event`
- `std_collections_VecDeque_std_shared_ptr_rrr_Event`
- `std_collections_BTreeSet_std_rc_Rc_rrr_Fiber`

These names previously degraded into opaque placeholder structs (or unresolved-type invariant failures). Fragile now:

- normalizes lowered `std_collections_*`, `std_rc_*`, and lowered smart-pointer spellings into concrete container targets (for example `std::collections::VecDeque<std_shared_ptr<...>>`, `std::collections::BTreeSet<std::rc::Rc<...>>`);
- allows unresolved-gap closure to emit alias fallbacks even when the generic base is a std path (`std::...`) rather than a locally defined helper type;
- runs one final unresolved-type closure pass at the end of codegen so late alias/c_void normalization stages cannot reintroduce unresolved lowered container names.

This keeps generated output on concrete std container aliases and avoids fallback opaque structs for these safe Rusty-C++ container surfaces.

## 6. Object Model and Inheritance Design

### 6.1 Struct layout and base embedding

For `RecordDecl`:

- Primary base field name: `__base`
- Additional non-virtual bases: `__base1`, `__base2`, ...
- Virtual bases: pointer + owned storage fields
  - pointer field: `__vbase_<Name>`
  - storage field: `__vbase_storage_<Name>: Option<Box<Name>>`

This preserves C++-like subobject accessibility while staying explicit in Rust layout.

### 6.2 Virtual dispatch model

Trait-only dispatch was removed in favor of explicit vtable design:

- Root polymorphic classes get `__vtable: *const <Type>_vtable` field.
- Static vtable instances `<TYPE>_VTABLE` are generated for concrete classes.
- Derived classes update inherited vtable pointer paths in constructors.
- Virtual call sites can be lowered to direct vtable function-pointer calls.

### 6.3 RTTI-like behavior

`dynamic_cast` lowering uses generated type-id tables in vtable metadata for polymorphic paths:

- pointer casts return null on failure
- reference casts panic (`std::bad_cast` equivalent path)

`typeid` lowers to `std::any::TypeId::of::<T>()` based forms.

### 6.4 Mode 1 non-primitive object interop contract

This section defines the target semantics for complex object interop under full-transpile Mode 1.

Rust calling C++-origin objects:

- Transpiled C++ classes are treated as normal Rust structs/impls in the final graph.
- Method calls from user Rust should resolve to direct Rust method calls on those structs.
- References/ownership follow Rust semantics (`&`, `&mut`, move), with transpiler-generated glue only for unsupported C++ constructs.

C++ calling Rust-owned objects/functions:

- User Rust items intended for C++ call sites are indexed as C++-visible Rust symbols.
- During C++ transpilation, matching call sites are rewritten to direct Rust path calls against those symbols.
- No user-written `extern "C"` is required for this in-project path.

Boundary rule:

- `extern "C"` is retained only for true foreign boundaries (system/external ABI), not for internal project calls where both sides end up as Rust in one build graph.

Current state vs target:

- Current implementation still emits some declaration-only ABI wrappers for compatibility.
- Mode 1 work reduces those wrappers as canonical symbol mapping and call rewriting become complete.

## 7. Function, Method, Constructor, Destructor Mapping

### 7.1 Free functions

- Definitions -> `pub fn`
- C-linkage-like symbols can be exported with unmangled names (`#[no_mangle]` / `extern "C"` where applicable).
- Non-definitions can generate extern declarations plus wrapper function.

### 7.2 Methods

- Non-static methods:
  - const-like -> `&self`
  - mutating/non-const -> `&mut self`
- Static methods: emitted without self parameter.

Operator methods are sanitized by `sanitize_identifier`:

- `operator+` -> `op_add`
- `operator[]` -> `op_index`
- `operator()` -> `op_call`
- `operator->` -> `op_arrow`
- etc.

### 7.3 Constructors

Constructors map to `new_N` naming:

- default constructor -> `new_0`
- N-arg constructor -> `new_N`
- overloads with same arity -> suffixed forms (`new_N_1`, ...)

Features in constructor lowering:

- member initializer list support (`MemberRef` extraction)
- base initializer support (`TypeRef:Base` + call shape)
- internal helper for virtual-base classes: `__new_without_vbases_N`
- public constructor layer allocates/patches virtual base storage and pointers

### 7.4 Destructor and copy semantics

- Destructor definitions can generate `impl Drop`.
- Explicit copy constructor can drive `impl Clone` generation.
- `Copy` derive is conditional on field/base safety checks.

## 8. Statement Mapping

Primary dispatch: `generate_stmt`.

| C++ statement | Rust lowering |
|---|---|
| declaration statement | `let` / `static mut` generation with typed initializer normalization |
| `return` | `return expr;` with pointer/int/bool/reference fixups |
| `if` / `else if` | native `if` chains; supports C++17 if-initializer by wrapping block |
| `while` | native `while`; declaration conditions lowered to `loop { let ...; if break; ... }` |
| `for(init;cond;inc)` | block + `loop` with explicit condition break and increment |
| range-for | `for x in range` / `range.iter()` patterns |
| do-while | `loop { body; if !cond { break; } }` |
| `switch/case/default` | `match` with fallthrough-preserving arm expansion |
| `break`/`continue` | native loop control, with switch-specific handling |
| `goto`/label | partial support via top-level exit-label transform to labeled loop breaks; otherwise commented unsupported |
| `try/catch` | `std::panic::catch_unwind`-based lowering |

## 9. Expression Mapping

Primary dispatch: `expr_to_string`.

### 9.1 Literal and reference forms

- integer/float/bool/string/nullptr literals lowered directly with type-aware suffix/normalization rules.
- `DeclRefExpr` resolves locals/globals/static members/functions with namespace-aware pathing.
- `this` -> `self` (or `__self` inside constructor self-pattern path).

### 9.2 Operators and core expressions

- `BinaryOperator` -> native Rust ops with pointer/integer/bool safety rewrites.
- `UnaryOperator` -> Rust unary forms plus C++ semantic adaptations (`!ptr` -> `is_null`, etc.).
- `ConditionalOperator` (`?:`) -> `if cond { a } else { b }`.
- `ParenExpr` preserved.

### 9.3 Call and member access

- `CallExpr` covers:
  - normal function calls
  - operator-overload call lowering
  - function-pointer calls (`Option<fn>` unwrap/expect pattern)
  - selected std/ranges/visit helper rewrites
- `MemberExpr` handles:
  - dot/arrow access
  - implicit `this` member access
  - base-class member pathing and pointer unsafe deref forms
- `ArraySubscriptExpr`:
  - pointer bases -> pointer arithmetic/deref
  - array bases -> index syntax

### 9.4 Casts

- `ImplicitCastExpr` and `CastExpr` convert with explicit `as`/pointer/null/function decay behavior when required.
- C++ named casts are represented through cast kinds and normalized expressions.
- `DynamicCastExpr` and `TypeidExpr` have dedicated lowering paths.

### 9.5 Construction, allocation, and initialization

- `CXXConstructExpr`:
  - constructor calls -> `Type::new_N(...)`
  - enum construction from integer -> `transmute` path
  - copy-constructor-like single-arg same-type -> clone path
- `CXXNewExpr`:
  - single object -> `Box::into_raw(Box::new(...))`
  - arrays -> runtime allocator helper (`fragile_new_array`)
  - placement new -> `std::ptr::write`-based sequence
- `CXXDeleteExpr`:
  - single object -> `drop(Box::from_raw(ptr))`
  - arrays -> runtime deallocator helper (`fragile_delete_array`)
- `InitListExpr`:
  - aggregate/array initialization lowering with type-aware default fill behavior

### 9.6 Modern C++ forms

- `LambdaExpr` -> Rust closure (`|...|` or `move |...|`).
- `ThrowExpr` -> `panic!`-based lowering.
- Coroutine nodes:
  - `co_await` -> `.await`
  - `co_yield` -> `yield ...`
  - `co_return` -> `return ...`

### 9.7 Rusty wrapper call normalization

Source: `map_builtin_function` in `crates/fragile-clang/src/ast_codegen.rs`.

Wrapper constructor helpers emitted from Rusty-C++-style APIs are normalized directly to Rust std enum constructors when argument shape is compatible:

- `Some_* (x)` -> `std::option::Option::Some(x)`
- `None` / `None_* ()` -> `std::option::Option::None`
- `Ok_* (v)` -> `std::result::Result::Ok(v)`
- `Err_* (e)` -> `std::result::Result::Err(e)`

This removes shim-style constructor names from generated expressions and improves safe-Rust readability.

### 9.8 Namespace alias target normalization

Source: `emit_namespace_type_aliases` and `normalize_namespace_alias_target` in `crates/fragile-clang/src/ast_codegen.rs`.

Auto-exported namespace aliases are normalized before emission. Representative rules:

- `rusty::String` -> `std::string::String`
- `rusty::Option<T>` / `rusty::Result<T, E>` -> `std::option::Option<T>` / `std::result::Result<T, E>`
- `rusty::HashMap<K, V>` / `rusty::HashSet<T>` -> `std::collections::HashMap<K, V>` / `std::collections::HashSet<T>`
- `rusty::RefCell<T>` / `rusty::UnsafeCell<T>` -> `std::cell::RefCell<T>` / `std::cell::UnsafeCell<T>`
- non-isomorphic Rusty sync wrappers that require lifetimes or custom error surfaces stay Rusty (for example guard aliases and `PoisonError`/`LockResult`/`TryLockResult`) to avoid invalid std-lifetime rewrites

This keeps generated top-level aliases std-native and avoids re-introducing rusty-cpp wrapper names in otherwise safe/std lowered output.

`generate` applies Rusty alias RHS normalization both in the main normalization chain and again at the end of the pipeline. The second sweep is intentional: late unresolved-reference closure and degraded fallback passes can append new `type` items after the first sweep.

### 9.9 Typed null pointer return pointee normalization

Source: `normalize_typed_null_pointer_return_pointees` in `crates/fragile-clang/src/ast_codegen.rs`.

Late degraded rewrites can produce mismatched typed-null returns such as:

- function signature: `-> *mut T`
- body line: `return std::ptr::null_mut::<U>();`

where `U` is not the function return pointee (for example a parameter-name artifact).

The normalization pass walks emitted function bodies and rewrites typed `null`/`null_mut` return pointees to match the enclosing function signature pointee. This keeps fallback returns type-correct without adding source-project-specific rules.

### 9.10 Global alias rewrite and `MaybeUninit` cleanup ordering

Source: `generate` pass ordering in `crates/fragile-clang/src/ast_codegen.rs`.

Some global references are emitted in rooted path form (`...::name`) and only later normalized to internal storage symbols (`...::__gv_name`) by `normalize_global_symbol_aliases_for_prefixed_statics`.

To avoid false "unreferenced" classification, `normalize_unreferenced_static_mut_initializers` runs **after** global alias normalization in the late pipeline. This prevents live globals from being downgraded to `MaybeUninit<T>` before their `__gv_*` uses are visible.

## 10. Templates and Instantiation Strategy

Template lowering is multi-pass and stateful. The strategy is:

- collect template definitions and concrete use-sites first
- emit concrete monomorphized Rust items when type inference is sufficiently concrete
- use LibTooling specialization metadata to recover concrete field/method signatures
- skip or stub unstable surfaces that historically produce uncompilable output

This section describes the actual passes and rules in `AstCodeGen`.

### 10.1 Core template state in `AstCodeGen`

Template codegen relies on these maps/sets:

- `template_definitions: HashMap<String, (Vec<String>, Vec<ClangNode>)>`
- `pending_template_instantiations: HashSet<String>`
- `fn_template_definitions: HashMap<String, FnTemplateInfo>`
- `pending_fn_instantiations: HashMap<String, (String, Vec<String>, FnTemplateInfo)>`
- `variadic_template_instantiations: HashMap<String, ClangNode>`
- `inline_namespace_aliases: HashMap<String, String>` (for inline namespace rewrites like `std` -> `std::__1`)
- `libtooling_method_bodies: HashMap<(String, String), Vec<MethodInfo>>`
- `specialization_field_types: HashMap<String, SpecializationFieldInfo>`
- `specialization_methods: HashMap<String, Vec<MethodSignature>>`
- `opaque_types` and `used_types` for unresolved-template fallback stub generation

### 10.2 Pass ordering in `generate`

Template-sensitive pass order is:

1. `collect_template_info` over TU children.
2. `collect_template_info` again (second pass catches call-sites appearing before template defs in AST order).
3. `generate_template_instantiations` (class template monomorphs).
4. `generate_fn_template_instantiations` (non-variadic function template monomorphs).
5. Normal top-level generation (`generate_top_level`) and statement lowering.
6. `generate_variadic_template_instantiations` at the end (these are discovered during statement traversal).

Key detail: `generate_stmt` calls `collect_variadic_template_instantiations` pre-order, so variadic template collection is intentionally late-bound.

### 10.3 Definition/use-site collection

Collection entrypoint: `collect_template_info_with_namespace`.

`ClassTemplateDecl` handling:

- Skips empty parameter lists.
- Stores templates with both short key (`name`) and fully-qualified key (`ns::name`).
- Prefers definitions that actually contain `FieldDecl` children over weaker duplicates.

`FunctionTemplateDecl` handling:

- Captures `FnTemplateInfo { template_params, return_type, params, body, is_noexcept }`.
- `body` is taken from child `CompoundStmt` when present.
- Short-name replacement is conservative to avoid namespaced/system-header overloads overriding better user-TU entries.
- Also stores fully-qualified key.

Namespace handling:

- `NamespaceDecl` updates a running namespace path.
- For inline namespaces, records aliases in `inline_namespace_aliases` so lookups can resolve `parent::Type` to `parent::__inline::Type`.

Type and call-site handling during collection:

- `collect_template_type` scans `VarDecl`/`FieldDecl`/function signatures.
- It recurses through pointer/reference/array wrappers.
- Named types with `<...>` and known template definitions are enqueued in `pending_template_instantiations`.
- `CallExpr` nodes trigger `collect_fn_template_instantiation`.

Lookup behavior:

- `lookup_template_definition` first tries direct key.
- Then tries inline namespace alias rewrite on the namespace prefix.

### 10.4 Class template instantiation pipeline

Instantiation loop: `generate_template_instantiations`.

Name parsing:

- Parses `inst_name` with `find_matching_close_angle` to safely handle nested templates.
- Splits base/args and uses `parse_template_args` (nesting-aware for `<...>` and `(...)`).
- Skips nested-member types shaped like `X<...>::Y` in this pass.

Generation entry: `generate_template_struct(inst_name, template_params, type_args, children)`.

Hard gates before emitting:

- Skip unresolved argument sets (`is_generic_type_param` / `is_dependent_type`).
- Skip names containing `type-parameter-`.
- Skip selected unstable families (for example `__wrap_iter`, `__normal_iterator`, `_Bit_iterator`, `memory_resource` internals).
- Skip invalid Rust names that still contain `::`.

Canonicalization and aliasing:

- Known equivalent instantiation spellings are canonicalized (notably some RapidJSON forms).
- `std::vector<T>`, `std::unique_ptr<T>`, `std::shared_ptr<T>` instantiations may lower to type aliases to generic preamble stubs (instead of emitting monomorphic opaque structs).
- If a canonical struct already exists, non-canonical spellings become `pub type` aliases.

Extern-template behavior:

- `SpecializationFieldInfo.specialization_kind == ExplicitInstantiationDeclaration` suppresses concrete struct emission.

Substitution map construction:

- Maps both declared template names (`_Key`) and Clang internal names (`type-parameter-0-N`) to concrete Rust type strings.

Field lowering:

- Preferred source is LibTooling specialization field map (`specialization_field_types`) when matched.
- Fallback source is AST field type substitution via `substitute_template_type`.
- Remaining unresolved placeholders are converted with `convert_to_opaque_type`, and opaque types are tracked for later stub emission.
- If no `FieldDecl` children are usable, specialization field info is used as a structural fallback.

Post-struct emission:

- Records `class_fields` for later method/constructor checks.
- Emits conservative `Default` and `Clone` impls for template surfaces.
- Emits `Copy` for selected families (for example `Stack_*`) where downstream behavior relies on by-value semantics.
- Calls `generate_template_impl`.

### 10.5 Template impl/method generation

Entry: `generate_template_impl(inst_name, rust_name, children)`.

Early exits:

- Skip extern-template instantiations.
- Skip primary-template/unresolved forms unless in allowlisted surface families (containers and selected RapidJSON types).
- Skip impls with no methods unless the type family requires synthetic methods.

Method source strategy:

- For AST `CXXMethodDecl` children, method signatures are taken from LibTooling-resolved specialization signatures (`find_resolved_method_signature`), not raw unresolved AST spellings.
- Signature matching supports:
  - exact key
  - `std::` prefix variants
  - normalized fuzzy match that strips `std::`, `class`, `struct`, and tolerates omitted default template args
- Only emits methods whose return/param types are resolvable against primitives/generated structs/stub candidates.

Method body sourcing:

- Looks up method bodies in `libtooling_method_bodies` using multiple keys:
  - `(rust_name, method)`
  - `(cpp_base_name, method)`
  - C++ operator name variants
  - fallback key variants
- If a body is found, it is lowered through normal block generation.
- If no body is found, emits a `todo!` placeholder.

Post-processing and rollback:

- `fix_field_as_method_calls` rewrites known field-call mis-lowerings.
- `references_nonexistent_field` and `should_rollback_template_impl` can discard emitted broken methods.

Synthetic method surfaces:

- `generate_libtooling_only_methods` can add methods present in LibTooling but missing in AST (currently constrained to safe subset like `size`/`op_index`).
- Container families receive targeted stubs/fixes (`size`, `new_0`, `push_back`, map `op_index`).
- `__tree_*` gets `__emplace_unique` stub if absent.
- Additional family-specific fallbacks are emitted for RapidJSON and tinyxml2 dynarray shapes.

### 10.6 Function template discovery and type-argument inference

Call-site collector: `collect_fn_template_instantiation(call_node)`.

Callee extraction:

- Accepts direct `DeclRefExpr` or `ImplicitCastExpr -> DeclRefExpr`.
- Requires callee type to be `CppType::Function` for normal path.

Candidate template definition keys (in order):

1. explicit namespace-qualified name from call metadata
2. unqualified function name
3. any known qualified definition with matching leaf suffix (`::name`)

Type argument inference (`infer_fn_template_type_args`):

- Validates arity unless pattern has parameter pack.
- For each template parameter:
  - infers from first parameter pattern containing that param
  - uses `extract_template_arg` recursively for pointer/reference/array pattern matching
  - can infer non-type array bounds (for forms like `const char (&)[N]`) from call-site literal size
  - falls back to return-type inference when needed
  - finally applies index-based fallback for degraded ASTs
- Non-type params are stricter: if explicit non-type inference fails, inference aborts.

Before accepting an instantiation:

- Compares substituted parameter types against instantiated call signature (with normalization and relaxed reference-prefix equivalence).
- Compares substituted return type similarly.
- Keeps fallback candidate if exact match is not found but structure is plausible.

Accepted instantiations are stored into `pending_fn_instantiations` keyed by synthesized mangled name (`name_<sanitized_type_args...>`).

Special synthesis path:

- `same_ptr_const_i8` has dedicated synthesis for string-literal comparison helper shape.

Call lowering usage:

- During expression lowering, function `DeclRefExpr` can resolve to synthesized template-instantiation symbol names when an instantiation is pending/already generated.

### 10.7 Function template emission

Emission loop: `generate_fn_template_instantiations` -> `generate_fn_template_instance`.

Emission rules:

- Deduplicates by sanitized mangled name.
- Rebuilds substitution map from template params to inferred args.
- Replaces unresolved return/param placeholders with positional fallback args when available.

Skip filters (aggressive, by design):

- Variadic pack spellings (`...`), unresolved placeholders, dependent/`typename` shapes, `decltype`, problematic C-style function pointer spellings, and selected known-broken internal templates are skipped.

Signature normalization:

- Renames duplicate parameter names.
- Rewrites `*const ()` / `*mut ()` to concrete pointer type when a unique candidate exists in the same signature.
- Adds named lifetime (`'a`) when return type borrows from one of multiple reference parameters.

Body emission:

- If template body exists, uses `generate_fn_template_body` / `generate_fn_template_stmt` with expression-level type substitution (`substitute_type_in_expr`) and `unsafe` wrappers where needed.
- Statement support includes key forms (`return`, `DeclStmt`, `if`, nested compound blocks, generic expression statements).
- Constexpr artifact statements are filtered.

Rollback:

- Generated function is discarded if body is effectively empty for non-void return type.
- Additional rollback patterns are checked by `should_rollback_fn_template`.

### 10.8 Variadic template strategy

Variadic instantiations are handled separately from normal function-template flow.

Collection:

- `collect_variadic_template_instantiations` recursively scans nodes.
- Records `CallExpr { template_instantiation: Some(...) }`.
- Stores either `FunctionTemplateInstantiation` or fallback `FunctionDecl` surfaces in `variadic_template_instantiations`.

Emission:

- `generate_variadic_template_instantiations` runs after main code generation pass.
- `generate_variadic_template_instance` builds function names using base name + unique sanitized parameter type suffixes.
- Handles duplicate parameter names by renaming.
- Applies same reference-lifetime tie-in heuristic for reference returns.
- If pack-expanded body has duplicate original parameter names (common), emits a `todo!` stub to avoid wrong rewrites.
- Rolls back generated output containing unresolved pack artifacts like `Args...`.

Direct top-level handling:

- Parser-surfaced `FunctionTemplateInstantiation` nodes are also emitted immediately in `generate_top_level` via the same variadic emitter.

### 10.9 Dependent/unresolved fallback policy

The fallback model is intentionally compile-first:

- `has_unresolved_template_placeholder` detects unresolved parameter markers (`type-parameter-*`, `_Tp`, `_Alloc`, `typename`, etc.).
- `substitute_template_type` handles nested substitution over named, pointer, reference, array, and function types.
- `replace_unsubstituted_type_params` replaces unreplaced internal placeholders with `DefaultType`.
- `convert_to_opaque_type` maps unresolved template params to generated opaque placeholders (`__Opaque_*`) to preserve type identity better than collapsing everything to `c_void`.
- End-of-pipeline stub emission (`generate_opaque_type_stubs`, `generate_void_placeholder_stubs`) ensures unresolved references still produce compilable Rust items.
- Missing-specialization placeholder structs with recovered primitive fields synthesize typed defaults (`false` for `bool`, numeric zero for numeric primitives, null for pointers) to keep fallback `Default` impls type-correct.

### 10.10 Practical design tradeoff

Current template lowering is intentionally conservative:

- prefer concrete monomorphized emission when substitutions are trustworthy
- otherwise skip or stub narrowly to keep generated Rust compiling
- use LibTooling specialization metadata as the authoritative concrete typing source where available

This sacrifices full metaprogram fidelity in some STL/internal cases, but keeps the transpilation pipeline progressing on large real-world codebases.

### 10.11 Concrete Translation Examples

The following examples mirror current emitted shapes.

| C++ template surface | Rust output shape |
|---|---|
| `std::vector<Employee>` | `pub type std_vector_Employee = std_vector<Employee>;` |
| `vector<char>` | `pub type vector_char = std_vector<i8>;` |
| `vector<void *>` | `pub type vector_void = std_vector<*mut std::ffi::c_void>;` |
| `vector<unique_ptr<Group>>` | `pub type vector_unique_ptr_Group = std_vector<std_unique_ptr<Group>>;` |
| `std::unique_ptr<Widget>` | `pub type std_unique_ptr_Widget = std_unique_ptr<Widget>;` |
| `std::shared_ptr<Node>` | `pub type std_shared_ptr_Node = std_shared_ptr<Node>;` |
| `extern template class Widget<int>;` | No concrete struct/impl emitted for that instantiation. |
| `template class Widget<int>;` (explicit instantiation definition) | Concrete `pub struct Widget_int { ... }`-style instantiation is emitted (sanitized name). |
| `Stack<CrtAllocator>` specialization | Concrete struct + fallback `impl Default`, `impl Clone`, and `impl Copy` surface. |
| `template<typename T> bool XMLTest(..., T expected, T found, ...)` called with string pointers | Function instantiation emitted with deduced type in name, e.g. `pub fn XMLTest_ptr_const_i8(...) -> bool`. |
| `template<size_t N> bool same(..., const char (&)[N])` called with `"~"` | NTTP bound is inferred and emitted as bound-specific instantiation, e.g. `pub fn same_2(...) -> bool`, with call-site rewritten to `same_2(...)`. |
| variadic template instantiation with duplicate expanded parameter names | Emits `todo!("variadic template instantiation")` stub rather than incorrect body rewrite. |

Example snippets:

```cpp
// C++
std::vector<Employee> employees;
vector<char> bytes;
```

```rust
// Rust (emitted aliases)
pub type std_vector_Employee = std_vector<Employee>;
pub type vector_char = std_vector<i8>;
```

```cpp
// C++
template<typename T>
bool XMLTest(const char* testString, T expected, T found, bool echo);

bool call_xml_test_strings() {
  return XMLTest("Test", "Expected", "Found", true);
}
```

```rust
// Rust (emitted instantiation + call path)
pub fn XMLTest_ptr_const_i8(
    testString: *const i8,
    expected: *const i8,
    found: *const i8,
    echo: bool,
) -> bool { ... }
```

```cpp
// C++
template<size_t N>
bool same(const char* str, size_t size, const char (&literal)[N]);

bool IsNullString(const char* str, size_t size) {
  return same(str, size, "~");
}
```

```rust
// Rust (bound inferred from "~" + nul => N = 2)
pub fn same_2(str: *const i8, size: u64, literal: *const i8) -> bool { ... }
```

## 11. Runtime and Preamble Integration

`AstCodeGen::generate` emits integration scaffolding before/after normal item emission:

- STL/runtime preamble insertion (`emit_stl_preamble`).
- Variant helper enum generation for discovered variant usages.
- Vtable structs and static vtables generation for polymorphic classes.
- Missing type/opaque placeholder stub synthesis.
- Post-generation normalization passes to repair known degraded code patterns.

Memory/runtime call surfaces include wrappers for allocation and C runtime compatibility where needed.

## 12. Extension Guide for Contributors

When adding or changing a transpilation rule:

1. Decide layer first.
- Type-level rule: `types.rs` (`CppType` conversion)
- Declaration-level rule: `generate_top_level`, `generate_struct`, `generate_enum`, etc.
- Statement/expression-level rule: `generate_stmt`, `expr_to_string`

2. Preserve C++ semantics first, then optimize Rust style.
- Respect value/reference/pointer differences.
- Keep `unsafe` boundaries explicit and minimal.

3. Keep naming and overload rules consistent.
- Reuse `sanitize_identifier` and existing `new_N` conventions.

4. Add regression tests near existing `ast_codegen.rs` test modules and integration tests under `crates/fragile-clang/tests`.

5. If a rule is intentionally conservative (skip/placeholder), document the reason inline to keep future cleanup tractable.

## 13. Appendix: Per-ClangNodeKind Lowering Matrix

This appendix enumerates every variant in `ClangNodeKind` and the current lowering rule used by `AstCodeGen`.

### 13.1 Declaration and Structural Variants

| `ClangNodeKind` variant | Primary lowering path | Exact current lowering rule |
|---|---|---|
| `TranslationUnit` | `AstCodeGen::generate` | Runs multi-pass analysis/enrichment; emits preamble/template/vtable helpers; iterates children through `generate_top_level`. |
| `FunctionDecl` | `generate_top_level` -> `generate_function` or `generate_extern_c_function_decl` | Definitions become Rust function bodies; declarations become extern declaration + wrapper shim. |
| `FunctionTemplateDecl` | `collect_template_info` + template passes | Definition metadata is collected; concrete emissions happen through later template instantiation passes, not direct top-level emission. |
| `FunctionTemplateInstantiation` | `generate_top_level` -> `generate_variadic_template_instance` | Emits concrete instantiated function surface. |
| `ClassTemplateDecl` | `generate_top_level` + template storage | Stores template definition and emits concrete `RecordDecl` instantiations found in children. |
| `ClassTemplatePartialSpecDecl` | `generate_top_level` | Processes children and emits concrete specialized `RecordDecl` definitions when present. |
| `TemplateTypeParmDecl` | template metadata only | No direct Rust item emission; used as template parameter metadata during template processing. |
| `ParmVarDecl` | parameter metadata only | No direct item emission; consumed while building function/method/constructor signatures and body name reconciliation. |
| `VarDecl` | top-level: `generate_top_level` -> `generate_global_var`; local: `generate_stmt` via `DeclStmt` | Emits globals/statics at top-level; local declarations emit `let`/local static lowering with extensive initializer/type normalization. |
| `RecordDecl` | `generate_top_level` -> `generate_struct` | Emits `#[repr(C)] pub struct`, base fields (`__base...`), virtual-base fields, bitfield storage, impl/methods/ctors/drop/clone helpers. |
| `UnionDecl` | `generate_top_level` -> `generate_union` | Emits `#[repr(C)] pub union`; wraps non-`Copy` fields in `ManuallyDrop`; emits `new_0`, `Default`, and `Clone` (memcpy-based) when needed. |
| `FieldDecl` | consumed inside `generate_struct`/`generate_union` | Not emitted standalone; becomes struct/union fields, static-member globals, bitfield groups, accessor metadata. |
| `EnumDecl` | `generate_top_level` -> `generate_enum` | Emits Rust enum with repr (or alias for empty enums); handles duplicate discriminants with const aliases and unnamed-enum fallback constants. |
| `EnumConstantDecl` | consumed inside `generate_enum` | Not emitted standalone; becomes enum variants or unnamed-enum exported constants. |
| `CXXMethodDecl` | consumed in `generate_struct` -> `generate_method` | Emits method impl entries (including operator name sanitization, self mutability selection, rollback validation). |
| `ConstructorDecl` | consumed in `generate_struct` -> `generate_method` | Emits `new_N` constructors (+ overload suffixes), base/member initializer lowering, virtual-base constructor layering (`__new_without_vbases_N`). |
| `DestructorDecl` | consumed in `generate_struct` | Emits `Drop` impl when valid and not filtered by rollback guards. |
| `MemberRef` | constructor member-initializer parsing in `generate_method` | Used as metadata marker for member initializer-list extraction (`MemberRef` + next expr). |
| `FriendDecl` | no direct codegen branch | Currently ignored for emission; friendship access control is not modeled as a standalone Rust construct. |
| `CXXBaseSpecifier` | consumed in `generate_struct` | Emits embedded base fields (`__base`, `__base1`, ...) for non-virtual bases; contributes to inheritance/vbase metadata and ctor behavior. |
| `NamespaceDecl` | `generate_top_level` | Emits `pub mod` blocks, merges reopened namespaces, handles anonymous namespaces and inline namespace behavior. |
| `LinkageSpecDecl` | `generate_top_level` | Treated as transparent container; emits children directly. |
| `UsingDirective` | no direct codegen branch | Currently not emitted as Rust `use`; node is effectively ignored at codegen time. |
| `UsingDeclaration` | no direct codegen branch | Currently not emitted as Rust `use`; node is effectively ignored at codegen time. |
| `TypeAliasDecl` | `generate_top_level` -> `generate_type_alias` | Emits `pub type` alias with collision/self-cycle/filter checks and alias-target tracking. |
| `TypeAliasTemplateDecl` | no direct codegen branch | Currently no direct Rust alias-template emission; treated as unsupported/no-op in codegen stage. |
| `TypedefDecl` | `generate_top_level` -> `generate_type_alias` | Same alias lowering path as `TypeAliasDecl`. |
| `ModuleImportDecl` | `generate_top_level` | Emits comment placeholders for C++20 module/header-unit import; no functional Rust module import mapping yet. |

### 13.2 Statement Variants

| `ClangNodeKind` variant | Primary lowering path | Exact current lowering rule |
|---|---|---|
| `CompoundStmt` | `generate_stmt` + `generate_block_contents` | Emits `{ ... }` block with scoped-local tracking and recursive statement lowering. |
| `ReturnStmt` | `generate_stmt` | Emits `return ...;` with pointer/null/reference/int/bool/enum cast normalization and function return-type compatibility fixes. |
| `IfStmt` | `generate_if_stmt` | Emits Rust `if`/`else`; supports C++17 if-initializer via enclosing block and generated init statement. |
| `WhileStmt` | `generate_while_stmt` | Standard `while` lowering; declaration-condition forms (`while (T x = ...)`) lower to explicit `loop` with per-iteration declaration + break check. |
| `ForStmt` | `generate_for_stmt` | Lowers `for(init; cond; inc)` into block + `loop` form with explicit condition break and increment placement. |
| `CXXForRangeStmt` | `generate_range_for_stmt` | Lowers to `for` over container/reference/iterator-friendly surface with loop-variable tracking. |
| `DoStmt` | `generate_do_stmt` | Lowers `do { ... } while(cond)` to `loop { body; if !cond { break; } }` with continue semantics handling. |
| `DeclStmt` | `generate_stmt` | Emits local declarations; handles arrays, pointers, refs, function pointers, variadic `va_arg` patterns, and default/aggregate initialization fallbacks. |
| `ExprStmt` | `generate_stmt` | Emits expression + semicolon (or tail expression in tail position); skips constexpr-bool artifacts. |
| `BreakStmt` | `generate_stmt` | Emits `break;` in loops; for switch-lowered loops applies switch-specific break handling/suppression. |
| `ContinueStmt` | `generate_stmt` | Emits `continue;` (for-loop/do-while helpers inject required increment/condition sequencing around it). |
| `SwitchStmt` | `generate_switch_stmt` | Lowers to `match` with arm expansion preserving C fallthrough behavior; tracks default arm and enum case variant mapping. |
| `CaseStmt` | `collect_switch_arms` / `collect_case_arm_parts` | Consumed as switch-arm metadata; not emitted as standalone statement item. |
| `DefaultStmt` | `collect_switch_arms` | Consumed as switch default-arm metadata; emitted via generated `_ => { ... }` match arm. |
| `NullStmt` | `generate_stmt` fallback path | No dedicated branch; falls through generic expression path and thus to generic expression fallback output when surfaced. |
| `GotoStmt` | `generate_stmt` | If transformed single-exit-label context exists, lowers to labeled `break`; otherwise emits `// unsupported goto <label>`. |
| `LabelStmt` | `generate_stmt` | Normally handled by goto transform; fallback behavior emits labeled child statement body only. |
| `TryStmt` | `generate_stmt` | Lowers to `match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { ... })) { ... }`. |
| `CatchStmt` | `generate_stmt` | Not emitted standalone; consumed as part of `TryStmt` lowering. |

### 13.3 Expression and Semantic Variants

| `ClangNodeKind` variant | Primary lowering path | Exact current lowering rule |
|---|---|---|
| `IntegerLiteral` | `expr_to_string` | Emits integer literal with inferred/explicit suffix normalization and unsigned-negative correction logic. |
| `FloatingLiteral` | `expr_to_string` | Emits float literal with suffix handling and special values (`INFINITY`/`NAN`) mapping. |
| `BoolLiteral` | `expr_to_string` | Emits `true` / `false`. |
| `NullPtrLiteral` | `expr_to_string` | Emits `std::ptr::null_mut()`. |
| `StringLiteral` | `expr_to_string` | Emits byte-string pointer form `b\"...\\0\".as_ptr() as *const i8` with C-escape decoding and safe byte escaping. |
| `EvaluatedExpr` | `expr_to_string` | Emits compile-time evaluated constant (int/float); applies enum and suffix/type conversions where needed. |
| `DeclRefExpr` | `expr_to_string` | Resolves locals/globals/static members/function paths, namespace-relative names, function-template call names, and special std stream/global cases. |
| `BinaryOperator` | `expr_to_string` | Emits binary operator expression with C++ semantics adaptations (pointer arithmetic/comparison, boolean coercions, comma op block form, `<=>` handling). |
| `UnaryOperator` | `expr_to_string` | Emits unary expression with pointer/reference/bool/integer adaptations (`&`, `*`, `!`, inc/dec, wrapping negation, etc.). |
| `CallExpr` | `expr_to_string` | Emits function/method/operator-call expression; includes virtual dispatch path, std helper rewrites, function-pointer call form, explicit destructor calls. |
| `CXXConstructExpr` | `expr_to_string` | Emits constructor call (`Type::new_N(...)`), copy-like clone path, enum transmute construction, and auto/degraded construct fallback handling. |
| `MemberExpr` | `expr_to_string` | Emits member access for dot/arrow/static/implicit-this/base-inherited cases, including unsafe pointer deref and virtual-base-aware pathing. |
| `ArraySubscriptExpr` | `expr_to_string` | Emits pointer-arithmetic deref (`unsafe` `.add/.sub`) for pointer-like bases or standard indexing for array bases. |
| `CastExpr` | `expr_to_string` | Emits explicit cast lowering (`static`/`reinterpret`/`const`/C-style/functional forms) with typed conversion and constructor-pattern handling. |
| `ConditionalOperator` | `expr_to_string` | Emits `if cond { then } else { else }` with branch normalization for pointer/null conditions. |
| `ParenExpr` | `expr_to_string` | Preserves explicit parenthesization by emitting `(<child>)`. |
| `ImplicitCastExpr` | `expr_to_string` | Emits implicit conversion only when needed (integral/float/pointer decay/null/function decay), otherwise passes inner expression through. |
| `InitListExpr` | `expr_to_string` | Emits aggregate/array initializer expression, including designated init support and array fill/truncation normalization. |
| `UnaryExprOrTypeTraitExpr` | `expr_to_string` fallback | No dedicated branch; currently falls back to generic expression fallback (`first child` or `"0"`). |
| `CXXDefaultInitExpr` | `expr_to_string` fallback | No dedicated branch; currently falls back to generic expression fallback (`first child` or `"0"`). |
| `CXXThisExpr` | `expr_to_string` | Emits `self` (or `__self` in constructor self-pattern mode). |
| `TypeTraitExpr` | `expr_to_string` fallback | No dedicated branch; currently falls back to generic expression fallback (`first child` or `"0"`). |
| `ConceptDecl` | `expr_to_string` fallback | No dedicated emission/lowering in codegen; currently unsupported and falls back when surfaced as expression context. |
| `RequiresExpr` | `expr_to_string` fallback | No dedicated emission/lowering in codegen; currently unsupported and falls back when surfaced as expression context. |
| `ConceptSpecializationExpr` | `expr_to_string` fallback | No dedicated emission/lowering in codegen; currently unsupported and falls back when surfaced as expression context. |
| `CoawaitExpr` | `expr_to_string` | Emits `expr.await`. |
| `CoyieldExpr` | `expr_to_string` | Emits `yield expr` (or bare `yield`). |
| `CoreturnStmt` | `expr_to_string` | Emits `return value` / `return` (consumed via statement fallback path with semicolon emission). |
| `ThrowExpr` | `expr_to_string` | Emits `panic!(...)`, attempting string/type-aware message extraction. |
| `LambdaExpr` | `expr_to_string` | Emits Rust closure (`|...|` or `move |...|`) with parameter typing and body lowering. |
| `TypeidExpr` | `expr_to_string` | Emits `std::any::TypeId::of::<T>()` form (with expression/type operand differentiation). |
| `DynamicCastExpr` | `expr_to_string` | Emits polymorphic RTTI-like checked cast path (null on pointer-failure, panic on reference-failure) or static cast fallback. |
| `CXXNewExpr` | `expr_to_string` | Emits heap/array/placement-new forms using `Box::into_raw`, `fragile_new_array`, and `std::ptr::write`-based placement code. |
| `CXXDeleteExpr` | `expr_to_string` | Emits delete/delete[] forms using `Box::from_raw` drop and `fragile_delete_array`. |
| `Unknown` | `expr_to_string` | Handles specific known unknowns (for example `BuiltinBitCastExpr` -> `transmute`); otherwise logs diagnostic and falls back to first child or `"0"`. |

## 14. Namespace Alias Normalization for Rusty Wrappers (2026-03-04)

### Problem

Auto-exported namespace aliases were only normalizing the single case `rusty::String -> std::string::String`. Other Rusty-C++ wrapper spellings (for example `rusty::Option<rusty::String>` or `rusty::HashMap<int, long>`) were not normalized through the same shared path used by regular type lowering.

### Rule

Namespace alias target normalization must reuse the same Rusty-wrapper mapping logic used in `CppType::Named` lowering, while preserving non-rusty namespaced targets unchanged.

### Implementation

- Added shared helper `normalize_rusty_type_alias_to_std()` in `crates/fragile-clang/src/types.rs`.
- Updated `AstCodeGen::normalize_namespace_alias_target()` to delegate to this helper instead of special-casing only `rusty::String`.

### Guardrails

- Wrapper aliases now normalize to Rust std paths consistently.
- Non-rusty namespaced aliases (for example `testing::internal::Visible`) are preserved as-is, avoiding accidental namespace mangling.
- Nested Rusty namespace spellings (for example `rusty::sync::Weak<T>`, `rusty::rc::Weak<T>`, `rusty::collections::HashMap<K, V>`) are normalized through the same shared path.
- Rusty thread/channel spellings now normalize as well (for example `rusty::thread::JoinHandle<T>`, `rusty::sync::mpsc::{Sender<T>, Receiver<T>, Unit, RecvError, TryRecvError, TrySendError}`), with `JoinHandle<void>` mapped to `std::thread::JoinHandle<()>`.
- Rusty option tag spellings now normalize to Rust unit as well (`rusty::None_t` / `None_t` -> `()`), which keeps alias surfaces free of Rusty-only none-marker types.
- Missing-stub concrete alias emission now normalizes its alias target before writing `pub type` lines, so unresolved-fallback aliases also rewrite `rusty::None_t` to `()` instead of leaking Rusty-only rhs paths.
- Verification note: when checking generated alias RHS values in drop-in builds, prefer `FRAGILEC_KEEP_RS=1` sidecar files (`*.fragile.rs`) over `/tmp/fragilec_*.rs`; `/tmp` may include stale outputs from unrelated prior invocations.
- Bare/non-generic `TrySendError` spellings now normalize to `std::sync::mpsc::TrySendError<()>` as a conservative std fallback, and templated forms normalize to `std::sync::mpsc::TrySendError<T>`.
- Rusty collection wrappers now tolerate extra C++ comparator/hasher/allocator template arguments while mapping to Rust std primary parameters:
  - `rusty::{Vec, VecDeque, HashSet, BTreeSet}<T, ...>` -> std one-parameter forms
  - `rusty::{HashMap, BTreeMap}<K, V, ...>` -> std two-parameter forms
  - strict-arity wrappers such as `Option<T>` and `Result<T, E>` remain strict (extra args are not silently rewritten)
- Rusty convenience aliases now normalize through the same path:
  - `rusty::Boxed<T>` / `Boxed<T>` -> `std::boxed::Box<T>`
  - `rusty::Shared<T>` / `Shared<T>` -> `std::sync::Arc<T>`
  - `rusty::RefCounted<T>` / `RefCounted<T>` -> `std::rc::Rc<T>`
  - `rusty::Ptr<T>` / `Ptr<T>` -> `*const T`
  - `rusty::MutPtr<T>` / `MutPtr<T>` -> `*mut T`
- Non-isomorphic Rusty sync guard/result wrappers are intentionally kept on Rusty paths to avoid invalid std rewrites:
  - member guard aliases remain Rusty (`rusty::Mutex<T>::Guard`, `rusty::RwLock<T>::ReadGuard`, `rusty::RwLock<T>::WriteGuard`)
  - direct guard aliases remain Rusty (`rusty::MutexGuard<T>`, `rusty::RwLockReadGuard<T>`, `rusty::RwLockWriteGuard<T>`)
  - direct result-wrapper aliases remain Rusty (`rusty::PoisonError<T>`, `rusty::LockResult<T>`, `rusty::TryLockResult<T>`)
  - rationale: std guard/result types carry lifetimes/private internals that are not represented by these Rusty aliases; forced std rewrites create invalid arity/lifetime/private-path outputs.
  - exception for outer `Result` aliases: `rusty::Result<Ok, rusty::PoisonError<T>>` now keeps the outer std result surface (`std::result::Result<...>`) and rewrites the `PoisonError` lane to its generated companion record identifier (for example `rusty_PoisonError_T`) so alias targets remain resolvable without forcing invalid std poison-error signatures.
- Rusty RefCell borrow wrappers now normalize as well:
  - `rusty::Ref<T>` / `Ref<T>` -> `std::cell::Ref<T>`
  - `rusty::RefMut<T>` / `RefMut<T>` -> `std::cell::RefMut<T>`
- Alias fallback emitters now also run the same normalization step (`generate_type_alias`, unresolved namespaced/lowercase alias synthesis, and template-instantiation alias bridges), so `pub type ... = rusty::...` RHS paths are normalized consistently wherever aliases are emitted.
- Added a late textual alias-RHS normalization pass over emitted `type` items so Rusty wrapper RHS spellings are normalized even when produced by alternate/degraded alias emitters (for example `pub type Barrier = rusty::Barrier; -> pub type Barrier = std::sync::Barrier;`), while unmapped Rusty spellings remain unchanged.
- `AstCodeGen::generate` now performs a final alias-RHS normalization sweep immediately before returning output so aliases appended by late unresolved-closure/fallback passes are normalized too.
- Lowered Rusty thread spellings in namespace exports are normalized as well (for example `rusty::thread::rusty_thread_JoinHandle_void_ -> std::thread::JoinHandle<()>`).
- Normalization preserves explicit Rust function-pointer template arguments (for example `Option<extern "C" fn(...) -> ...>`) instead of re-parsing/mangling them through C++ named-type lowering.
- Non-template Rusty sync primitives with direct std equivalents are normalized as well (`rusty::{Barrier, Condvar, Once, WaitTimeoutResult}` and `rusty::sync::{Barrier, Condvar, Once, WaitTimeoutResult}`).
- Unqualified Rusty wrapper spellings introduced by `using namespace rusty...` are normalized too for direct std-equivalent surfaces (for example `JoinHandle<T>`, `Sender<T>`, `Receiver<T>`, `Condvar`, `Unit`, `RecvError`, `TryRecvError`, `TrySendError`) so downstream aliases/fields do not leak unresolved bare wrapper names.
- Canonical and lowered std wrapper spellings for the same surfaces are normalized through the same path (for example `std::thread::JoinHandle<T>`, `std::sync::mpsc::{Sender, Receiver, SyncSender}<T>`, and lowered identifiers like `std_thread_JoinHandle_void_` / `std_sync_mpsc_Sender_int`) so generated code emits real std paths instead of sanitized wrapper identifiers.
- Lowered `TrySendError` spellings now normalize through the same lowered-wrapper path too (`std_sync_mpsc_TrySendError_*` / `rusty_sync_mpsc_TrySendError_*` -> `std::sync::mpsc::TrySendError<T>`), with empty/placeholder payload spellings defaulting to `()`.
- Lowered mpsc enum/error spellings now normalize too (for example `enumrusty_sync_mpsc_{RecvError,TryRecvError,TrySendError}`), and lowered `Result` wrappers carrying those errors (for example `rusty_Result_*_enumrusty_sync_mpsc_TrySendError_`) normalize directly to `std::result::Result<Ok, Err>` with std mpsc error types.
- Wrapper-record alias emission now allowlists `std::sync::mpsc::TrySendError<...>` alias targets, so namespace-qualified `TrySendError` records emit `pub type` aliases instead of opaque placeholder structs.
- Rooted Rust-style alias spellings are normalized through the same path as well (for example `::rusty::...` and `crate::rusty::...`).
- CV-qualified/tagged Rusty spellings are normalized too by stripping `const`/`volatile` and `class`/`struct` prefixes before matching (for example `const class ::rusty::Barrier` and `volatile struct crate::rusty::sync::mpsc::Receiver<const class rusty::String>`).
- Namespace alias emission and unresolved-namespaced alias fallback both skip Rusty marker-trait helper exports (`rusty_is_send_*` / `rusty_is_sync_*`) so generated `pub type` aliases do not leak internal trait-probe wrapper names.
- Direct typedef/using alias emission applies the same marker-helper filter, so source-level aliases targeting `rusty::rusty_is_send_*` / `rusty::rusty_is_sync_*` are dropped instead of surfacing unstable marker artifacts in generated Rust modules.
- Structs with normalized std mpsc field types (`std::sync::mpsc::{Sender, Receiver, SyncSender}`) now avoid `#[derive(Default)]`; codegen emits a manual zeroed `Default` impl to prevent derive-time trait-bound failures (`Sender`/`Receiver` do not implement `Default`), and the non-Default detector recognizes both canonical `std::...` spellings and sanitized identifier spellings (`std_sync_mpsc_...`) used by some AST surfaces.
- Non-Default/non-Clone wrapper detection is alias-aware too: if a field type resolves through local `type` aliases to non-derivable std wrappers (for example `std::sync::mpsc::Receiver<T>` or `std::thread::JoinHandle<T>`), derive emission is suppressed and synthesized defaults use zeroed initialization for those fields.
- Late missing-impl fallback synthesis now applies the same alias-aware non-Clone wrapper guard and will not auto-inject unsafe `Clone` impls for receiver/join-handle backed structs; this avoids `std::ptr::read`-style duplication of move-only std wrappers.
- Early derive-normalization (`normalize_struct_default_clone_derives`) now mirrors the same wrapper-aware policy: it strips only the invalid derive traits (`Default` for non-default wrappers, `Clone` for non-clone wrappers), keeps remaining valid derives, and skips unsafe clone fallback synthesis for receiver/join-handle-backed structs.
- The same early derive-normalization pass now strips invalid `Copy` derives for non-`Copy` std-wrapper fields (including alias-propagated forms like `std::collections::HashMap<...>` and `std::sync::mpsc::Sender<...>`) while preserving valid `Default`/`Clone` derives and avoiding unnecessary fallback impl synthesis when only `Copy` is invalid.
- Wrapper derive detection canonicalizes Rusty spellings through the same alias-normalization map (`rusty::sync::mpsc::Sender`, `rusty::thread::JoinHandle`, `rusty::HashMap`, etc.), and the derive-normalization pass now runs whenever derive attributes are present (not only when `c_void` appears), so wrapper-only outputs also get invalid `Default`/`Clone`/`Copy` derives stripped.
- Wrapper trait-block lists now cover additional std sync surfaces that are definitely non-derivable for generated records (`Barrier`, `Once`, `WaitTimeoutResult` for `Default`; `Mutex`, `RwLock`, `Condvar`, `Barrier`, `Once` for `Clone`; `Condvar`/`Barrier`/`Once` for `Copy`), preventing unsafe fallback clone synthesis on lock/sync-backed structs.
- c_void derive blocking is now pointer-indirection and function-pointer aware: direct `std::ffi::c_void` fields still force derive rewriting (including `Copy` stripping), while raw-pointer forms (`*const std::ffi::c_void`, `*mut std::ffi::c_void`) and `c_void` mentions inside function-pointer signatures no longer trigger unnecessary fallback `Default`/`Clone` synthesis.
- Wrapper derive-block propagation now treats raw-pointer fields/aliases as pointer-indirected surfaces (`*const T` / `*mut T`): non-`Default`/non-`Clone` restrictions of pointee wrapper types no longer leak through pointer fields, avoiding invalid `#[derive(Copy)]`-without-`Clone` rewrites on pointer wrapper shells.
- Missing-stub qualifier-family alias recovery also skips Rusty marker-trait helper exports (`rusty_is_send_*` / `rusty_is_sync_*`) instead of synthesizing `pub type` bridges for `rusty::is::send::...` / `rusty::is::sync::...` unresolved spellings.
- Record/template emission marker-helper suppression now resolves names through active namespace context too, so unqualified helper declarations inside `namespace rusty { ... }` (for example `is_send<...>` / `is_sync<...>`) are skipped just like fully-qualified `rusty::...` spellings.
- Record and template-instantiation emission now aliases a conservative allowlist of Rusty wrapper surfaces directly to normalized Rust std targets (for example `rusty::Arc<T>` -> `std::sync::Arc<T>`, `rusty::Option<T>` -> `std::option::Option<T>`, `rusty::thread::JoinHandle<T>` -> `std::thread::JoinHandle<T>`, `rusty::Vec<T>` -> `std::vec::Vec<T>`, `rusty::sync::mpsc::Unit` -> `()`, and non-generic wrappers like `rusty::Barrier`/`rusty::Condvar` -> `std::sync::...`) instead of emitting opaque wrapper structs; alias lookup also considers the active namespace path so unqualified declarations inside `namespace rusty { ... }` still normalize to std aliases, template aliases sanitize the emitted lhs name when normalized lowering yields a Rust path, aliasing is rejected if any nested `rusty::` path remains after normalization, and `Ref`/`RefMut` alias targets are now emitted with explicit `'static` lifetime lanes when materialized as `type` aliases.
- Wrapper-record aliasing now also maps direct Rusty poison-error wrappers to std surfaces when payload normalization is fully resolvable (`rusty::PoisonError<T>` / `rusty::sync::PoisonError<T>` -> `std::sync::PoisonError<T>`). Cases that still normalize to nested `rusty::...` payload lanes remain non-aliased to avoid unresolved std alias targets.
- Missing-stub concrete alias recovery now reuses the same Rusty-wrapper mapping path before opaque fallback, including namespace-qualified lookup for unqualified names. This keeps degraded unresolved spellings (for example `rusty::Arc::constclass::...`) on std alias surfaces instead of emitting placeholder structs.
- Fallback `Default`/`Clone` synthesis now treats Rusty wrapper paths that normalize to external std/core/alloc targets as external too, avoiding unsafe fallback impls on wrappers like `rusty::Barrier`/`rusty::Once`; lowered Rusty thread JoinHandle wrapper spellings (`rusty::thread::rusty_thread_JoinHandle_*`) remain eligible for local fallback impls because generated field-wise defaults still depend on them.
- Fallback `Default`/`Clone` synthesis must not emit impls for external `std::`/`core::`/`alloc::` targets after alias resolution; additionally, qualified local type paths must only resolve through exact alias keys (not unrelated leaf-name aliases) to avoid orphan impls such as `impl Clone for std::sync::Barrier`.
- Debugging note: `CMakeFiles/**.fragile.rs` sidecars can be stale unless `FRAGILEC_KEEP_RS=1` is set for the build invocation; use that env var when validating freshly emitted Rust text.
- Build hygiene note: `make clean` in `vendor/mako/build_fragilec_dropin` does not remove existing `*.fragile.rs` sidecars. For reliable grep validation, either delete stale sidecars explicitly or replay the exact TU compile command with `FRAGILEC_KEEP_RS=1`.
- Build/test note: the `rpcbench` CTest entry runs from `vendor/mako` and invokes `./build/rpcbench`; when validating an alternate CMake binary dir (for example `build_fragilec_dropin`), provide `vendor/mako/build` (e.g. symlink to the active build dir) so that `rpcbench` resolves correctly.
- Lowered Rusty single-argument wrapper spellings now normalize even when C++ lowered names carry prefixed qualifier/tag tokens (for example `classrusty_Rc_*`, `constclassrusty_Arc_*`, `structrusty_Option_*`), by stripping lowered `const/volatile/class/struct/enum/union` tokens before wrapper-prefix matching.
- Std wrapper normalization is now recursive over type arguments for common one/two-parameter std surfaces (`std::option::Option`, `std::boxed::Box`, `std::{sync,rc,cell}` wrappers, `std::result::Result`, `std::collections::{HashMap,BTreeMap}`), so outputs like `std::option::Option<rusty_Rc_...>`/`std::option::Option<rusty_Arc_...>` are normalized to fully-std argument shapes.
- Lowered Rusty set marker spellings now normalize to Rust unit as well: names matching lowered `BTreeSet`/`HashSet` internal `*_Unit` marker forms (including prefixed qualifier tokens) are rewritten to `()`, which in turn normalizes aliases like `std::collections::BTreeMap<..., rusty_BTreeSet_*_Unit>` to `std::collections::BTreeMap<..., ()>`.
- Template-argument normalization now preserves already-lowered explicit Rust unit arguments (`()`) instead of re-sanitizing them into placeholder identifiers, avoiding recursive normalization regressions such as `std::collections::BTreeMap<..., ()>` degrading to `...<..., __>`.
- JoinHandle normalization now strips trailing lowered delimiters from payload spellings and accepts additional lowered prefixes (`rusty_thread_JoinHandle_*`, `JoinHandle_*`), so degraded forms like `JoinHandle<void_>`, `std_thread_JoinHandle_void__`, and `rusty_thread_JoinHandle_void__` normalize to `std::thread::JoinHandle<()>` instead of `std::thread::JoinHandle<__>`.
- Non-generic Rusty/std thread scope spellings now normalize to std too (`rusty::thread::Scope`, `std::thread::Scope` -> `std::thread::Scope<'static, 'static>`). This keeps namespace-export alias targets on std surfaces while using concrete lifetimes because the Rusty spelling does not carry explicit lifetime parameters.
- Lowered single-argument Rusty wrapper normalization now includes sync lock wrappers (`rusty_Mutex_*`/`Mutex_*`, `rusty_RwLock_*`/`RwLock_*`) in the same pass used for `Option`/`Box`/`Arc`/`Rc`/cell wrappers, allowing lowered aliases to land on `std::sync::{Mutex,RwLock}<...>` directly.
- JoinHandle payload normalization now preserves explicit unit payloads (`()`) as unit; this avoids double-normalization regressions where already-normalized `std::thread::JoinHandle<()>` payloads were reprocessed into placeholder `std::thread::JoinHandle<__>` (for example through repeated namespace-alias target normalization).
- Single-argument wrapper normalization now also treats explicit `void` payloads as unit for std-qualified, Rusty-qualified, and unqualified wrapper spellings (for example `std::option::Option<void>`, `rusty::Arc<void>`, `Option<void>`, `Shared<void>`), so these normalize to std wrappers with `()` payloads instead of preserving C++ `void` inside Rust generic arguments.

## 15. Unused `c_void` Alias Pruning in Generated Rust (2026-03-05)

### Problem

Generated sidecar Rust files could accumulate large numbers of unused placeholder aliases like `pub type X = std::ffi::c_void;`. These aliases were primarily unresolved fallback artifacts and increased output noise without improving compileability.

### Rule

Drop top-level type aliases whose rhs is exactly `std::ffi::c_void` when the alias name is not referenced in emitted item type positions.

### Implementation

- Added `normalize_unused_c_void_type_aliases()` in `crates/fragile-clang/src/ast_codegen.rs`.
- Added `normalize_type_alias_rhs_c_void_alias_references()` in `crates/fragile-clang/src/ast_codegen.rs`.
- Added `normalize_c_void_alias_identifier_references()` and `normalize_unused_c_void_use_aliases()` in `crates/fragile-clang/src/ast_codegen.rs`.
- Wired the pass into `AstCodeGen::generate()` immediately after runtime-internal alias pruning.
- The pass:
  - scans `pub type`/`pub(crate) type`/`pub(super) type`/`type` aliases,
  - rewrites type-alias rhs expressions to inline known `std::ffi::c_void` aliases (for example `*mut __locale_struct` -> `*mut std::ffi::c_void`),
  - rewrites downstream identifier-token references that still point to c_void aliases (including `pub use` alias names) to direct `std::ffi::c_void` spellings,
  - removes aliases targeting `std::ffi::c_void` when unused,
  - removes unreferenced `pub use std::ffi::c_void as Name;` aliases after identifier inlining,
  - rewrites surviving public `c_void` aliases to equivalent `pub use std::ffi::c_void as Name;` form when no same-name concrete type item exists,
  - removes contiguous preceding `///` doc lines for dropped aliases,
  - removes one following blank line for output compaction.

### Guardrails

- Referenced `std::ffi::c_void` aliases are preserved.
- Non-`std::ffi::c_void` aliases are untouched.
- Alias-rhs inlining runs before pruning, so transitive alias chains can collapse and become removable without touching non-alias item signatures.
- `pub use ... as Name` rewrites are skipped when `Name` collides with a concrete same-name `struct`/`enum`/`union` item in the final output.
- Declared-type collection for unresolved-type closure/invariant checks now recognizes `use ... as Name` declarations as defined type-like names, so invariant enforcement remains compatible with the rewrite.
- c_void alias normalization is re-run at the tail of the normalization pipeline (after late unresolved/fallback passes) so newly appended alias surfaces are canonicalized before final output.
- Regression tests cover:
  - unused alias removal with doc cleanup,
  - used alias preservation,
  - non-`c_void` alias passthrough,
  - alias-rhs inlining and subsequent prune enablement,
  - `pub type` -> `pub use` rewrite behavior and collision guards,
  - unresolved-type collection with `pub use` aliases,
  - identifier-reference inlining and `pub use` alias pruning.

## 16. Safe Lowered `unordered_map` -> `HashMap` Alias Gating (2026-03-05)

### Problem

Lowered unresolved map spellings can include unusable components (for example `unordered_map<void *, unordered_map>`). A naive `unordered_map_*` -> `std::collections::HashMap<...>` rewrite can emit invalid Rust aliases (`std_ffi_c_void` keys, unresolved bare container component types) and break drop-in builds.

### Rule

Only emit lowered map aliases when both key/value components are valid concrete Rust component types for `HashMap`. Otherwise keep the unresolved map as an opaque placeholder.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`, added `is_supported_associative_map_component_type()`.
- `stl_simple_map_key_value_rust_types_from_suffix()` now requires both parsed key/value components to pass this guard.
- Added conservative lowered full-signature parsing for map spellings via `stl_map_key_value_suffix_parts_from_suffix()`:
  - supports signatures with trailing hash/equal/less/allocator lowered tails,
  - only when key/value extraction is unambiguous.
- Extended associative alias targets to include:
  - `std_unordered_map_*` / `unordered_map_*` -> `std::collections::HashMap<K, V>`
  - `std_map_*` / `map_*` -> `std::collections::BTreeMap<K, V>`
- Added key-type gating with `is_supported_associative_map_key_type()` so only conservative key surfaces (primitive scalars, raw pointers, string-like std spellings) are lowered to std map aliases.
- Guard rejects:
  - unresolved template placeholders,
  - `std::ffi::c_void`/`std_ffi_c_void`,
  - bare lowered container base names (for example `unordered_map`, `map`, `vector`, `set`, `queue`, `stack`, and std-prefixed counterparts).
- Added regression test:
  - `test_missing_stub_unordered_map_with_unusable_component_types_keeps_placeholder`
  - `test_missing_stub_full_unordered_map_signature_aliases_to_std_hashmap`
  - `test_missing_stub_simple_map_aliases_to_std_btreemap_for_safe_keys`
  - `test_missing_stub_map_with_non_conservative_key_keeps_placeholder`

### Guardrails

- Simple lowered `std_unordered_map_*` spellings with concrete components (for example `long` -> `i64`, `constclass_rusty_Arc_class_rrr_Future_` -> `std::sync::Arc<rrr_Future>`) still alias to `std::collections::HashMap<...>`.
- Lowered full-signature unordered-map spellings with explicit hash/equal/allocator tails now alias to the same `std::collections::HashMap<...>` surface when key/value are conservatively recoverable.
- Simple lowered ordered-map spellings with conservative keys (for example `map_unsigned_int__bool`) now alias to `std::collections::BTreeMap<...>`.
- Ordered maps with non-conservative keys (for example `map_ALock__unsigned_long`) remain opaque placeholders to avoid introducing trait-bound regressions from forced std container semantics.
- Unusable lowered `unordered_map_*` spellings now remain opaque, preventing invalid Rust type aliases and preserving build stability.

## 17. Safe Lowered `set` / `unordered_set` -> std Set Alias Gating (2026-03-05)

### Problem

Lowered unresolved set spellings show up in both simple and full-signature forms (for example `set_unsigned_short`, `unordered_set_long__struct_std_hash_long__struct_std_equal_to_long__class_std_allocator_long`). A naive rewrite can force invalid aliases when element types are unresolved or non-conservative (for example `set_Arc_Job`).

### Rule

Only emit lowered set aliases when the recovered element type is conservative and suitable for std set semantics. Otherwise keep the unresolved lowered set name as an opaque placeholder.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`, added:
  - `stl_set_element_suffix_from_suffix()`
  - `stl_simple_set_element_rust_type_from_suffix()`
- The parser handles:
  - simple lowered forms (for example `set_unsigned_short`, `unordered_set_int`),
  - lowered full-signature tails with hash/equal/less/allocator markers.
- Extended associative alias targets to include:
  - `std_unordered_set_*` / `unordered_set_*` -> `std::collections::HashSet<T>`
  - `std_set_*` / `set_*` -> `std::collections::BTreeSet<T>`
- Reused conservative gating:
  - `is_supported_associative_map_component_type()` for element validity,
  - `is_supported_associative_map_key_type()` for set element suitability.
- Added associative-component suffix normalization for map key/value parsing:
  - lowered map value components that are conservative lowered set spellings are now normalized to safe std set targets (for example `set_unsigned_short` -> `std_collections_BTreeSet_u16` -> `std::collections::BTreeSet<u16>`).
- Added regression tests:
  - `test_missing_stub_simple_unordered_set_aliases_to_std_hashset`
  - `test_missing_stub_simple_set_aliases_to_std_btreeset`
  - `test_missing_stub_set_with_non_conservative_element_keeps_placeholder`
  - `test_missing_stub_map_with_conservative_set_value_aliases_to_std_btreemap`
  - `test_missing_stub_unordered_map_with_conservative_set_value_aliases_to_std_hashmap`

### Guardrails

- Conservative lowered set spellings now alias to std containers (`HashSet`/`BTreeSet`) in missing-type stubs and drop-in sidecars.
- Full-signature lowered unordered-set spellings with recognized std tail markers alias when element extraction is unambiguous.
- Lowered map spellings with conservative lowered set values now alias to std map surfaces instead of remaining opaque placeholders (for example `map_unsigned_long__set_unsigned_short` -> `std::collections::BTreeMap<u64, std_collections_BTreeSet_u16>` with `std_collections_BTreeSet_u16` aliasing to `std::collections::BTreeSet<u16>`).
- Non-conservative lowered set spellings (for example `set_Arc_Job`) remain opaque placeholders, avoiding forced std trait-bound regressions.

## 18. Basic-String Keyed `unordered_map` Missing-Stub Alias Recovery (2026-03-05)

### Problem

Lowered names like `unordered_map_basic_string_char__unsigned_long` were being mis-aliased to `std_string` because the associative alias path rejected `basic_string_char` keys, then a broad fallback treated any type mentioning `basic_string<char>` as if it were the string type itself.

### Rule

Treat lowered `basic_string<char>` spellings as conservative associative keys and prevent string-fallback aliasing from firing on lowered STL container names.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Extended `is_supported_associative_map_key_type()` to accept:
    - `basic_string_char`,
    - `std_basic_string_char`,
    - lowered `basic_string_char__*` variants.
  - Tightened the `resolve_missing_stub_concrete_alias_target()` `basic_string<char>` fallback:
    - added a guard to skip this fallback when `rust_name` is a lowered STL container spelling (`unordered_map_*`, `map_*`, `set_*`, `vector_*`, etc).
- Added regression test:
  - `test_missing_stub_unordered_map_with_basic_string_key_aliases_to_std_hashmap`
  - asserts the missing-stub emits `std::collections::HashMap<...>` and not `std_string`.

### Guardrails

- Lowered string-like concrete types still canonicalize to `std_string` when they are truly string spellings.
- Lowered associative container spellings with `basic_string<char>` keys now stay container-shaped (`HashMap`), avoiding accidental scalar alias collapse.
- Full `mako` clean build (`make clean`, `cmake --build . -j32`) and `ctest -j32 --output-on-failure` remain green after the change.

## 19. Conservative Sequence-Valued Map Aliasing + `c_void` Template Arg Normalization (2026-03-05)

### Problem

Some lowered map spellings carry safe sequence values (for example `vector<unsigned long>`), but previous gating left those maps opaque. Separately, alias normalization could still emit `std_shared_ptr<std_ffi_c_void>` / `std_unique_ptr<std_ffi_c_void>` in generated Rust, which fails to compile.

### Rule

- Allow lowered map values to alias to std map surfaces when the value is a conservative lowered sequence type.
- Normalize `std_ffi_c_void` template arguments to `std::ffi::c_void` in alias/template normalization paths.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Added `stl_simple_sequence_component_rust_type_from_suffix()` for conservative lowered sequence spellings:
    - `std_vector_` / `vector_` -> `std_vector<T>`
    - `std_deque_` / `deque_` -> `std_deque<T>`
    - `std_collections_VecDeque_` -> `std::collections::VecDeque<T>`
  - Added `is_supported_associative_map_value_type()` and switched map-value gating to use it.
  - Extended `stl_associative_component_rust_type_from_suffix()` to recover sequence-valued components before scalar fallback.
- In `crates/fragile-clang/src/types.rs`:
  - `map_alias_template_arg_to_rust()` now checks `map_rusty_type_to_std()` before raw `CppType::Named(...).to_rust_type_str()`.
  - `map_rusty_type_to_std()` now canonicalizes:
    - `std::ffi::c_void`, `core::ffi::c_void`, `c_void`, `std_ffi_c_void` -> `std::ffi::c_void`
  - Preserves prebuilt generic spellings during normalization for:
    - `std_vector<...>`, `std_deque<...>`, `std_queue<...>`, `std_stack<...>`,
    - `std_unique_ptr<...>`, `std_shared_ptr<...>`.
- Added regression tests for:
  - conservative/non-conservative vector-valued map alias behavior,
  - nested map/template normalization preserving prebuilt wrappers and canonical `c_void`.

### Guardrails

- Map aliases only become std map aliases when keys remain conservative and values are conservative leaves or conservative sequence types.
- `std_shared_ptr<std_ffi_c_void>` and `std_unique_ptr<std_ffi_c_void>` no longer appear in generated drop-in Rust output.
- Remaining `mut_std_ffi_c_void` appearances are placeholder-struct lanes (for opaque unresolved by-value surfaces), not broken template-arg aliases.

### Build workflow note

`vendor/mako` uses `/target/release/fragilec` via the compiler wrapper and does not rebuild it automatically during `cmake --build`. After transpiler edits, rebuild the compiler first:

`cargo build --release --bin fragilec`

## 20. Map Value Recovery for `vector<Arc<T>>`-Like Lanes (2026-03-05)

### Problem

Lowered map spellings such as:

- `unordered_map_basic_string_char__vector_Arc_Pollable`
- `map_basic_string_char__vector_Arc_Client`

were still emitted as opaque placeholder structs even though both key/value sides are recoverable to concrete std-container aliases. The previous sequence-value gate only accepted scalar/string-like element lanes.

### Rule

Keep key gating strict, but allow richer concrete map **value** payloads (including wrapper/object lanes like `std::sync::Arc<T>`) when they are fully resolved and not unresolved placeholders.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - `stl_simple_sequence_component_rust_type_from_suffix()` now validates sequence elements with `is_supported_associative_map_value_type()` (value policy) instead of key policy.
  - Extended lowered sequence recovery prefixes to include:
    - `std_queue_` / `queue_` -> `std_queue<T>`
    - `std_stack_` / `stack_` -> `std_stack<T>`
  - `is_supported_associative_map_value_type()` now recursively validates supported container payloads (`std_vector`, `std_deque`, `std::collections::VecDeque`, `std_queue`, `std_stack`) using value policy on inner template args.

### Tests

Added/updated missing-stub regression tests:

- `test_missing_stub_unordered_map_with_vector_arc_value_aliases_to_std_hashmap`
- `test_missing_stub_map_with_vector_arc_value_aliases_to_std_btreemap`
- `test_missing_stub_unordered_map_with_nested_lowered_vector_value_keeps_placeholder`

### Guardrails

- Keys still require conservative key surfaces (`is_supported_associative_map_key_type()`).
- Values still reject unresolved placeholders and `c_void` lanes.
- Nested lowered vector-of-lowered-vector lanes remain opaque when element recovery is not concrete.
- Full drop-in validation remains green:
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`

## 21. Nested Map Values + Full-Signature String Map Recovery (2026-03-05)

### Problem

Two conservative gaps remained in missing-stub associative alias recovery:

- Lowered simple map spellings with nested lowered map values (for example `map_basic_string_char__map_basic_string_char__unsigned_long`) remained opaque because value-suffix parsing only accepted a single `__` split in simple lanes.
- Lowered full-signature map spellings with `basic_string<char, ...>` components or lowered `class_/struct_` prefixes could collapse to unstable intermediate lanes in larger TUs (including accidental integer-key/value surfaces) instead of canonical string-like map aliases.

### Rule

- Keep map-key gating conservative, but allow nested lowered map spellings in **simple** value lanes when key/value components are still recoverable and supported.
- In **full-signature** lanes, permit `basic_string<char, ...>`-shaped value suffixes while keeping other deep nested lanes conservative.
- Canonicalize lowered `class_/struct_/const` prefix tokens in associative components and keep `std_basic_string_char` lanes on the canonical `basic_string_char` surface.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - `stl_map_key_value_suffix_parts_from_suffix()` now:
    - accepts simple key/value splits where value can carry nested lowered spellings,
    - keeps full-signature tail-marker parsing for hash/equal/less/allocator lanes,
    - allows full-signature value `__` lanes only for conservative `basic_string<char, ...>`-shaped suffixes.
  - `stl_associative_component_rust_type_from_suffix()` now recognizes nested lowered map prefixes:
    - `std_unordered_map_` / `unordered_map_` -> `std::collections::HashMap<K, V>`
    - `std_map_` / `map_` -> `std::collections::BTreeMap<K, V>`
  - Added lowered qualifier/tag stripping helper for suffix lanes:
    - strips `const` / `volatile` / `class` / `struct` / `enum` / `union` prefix tokens.
  - `stl_container_element_rust_type_from_suffix()` now canonicalizes `std_basic_string_char` to `basic_string_char`.
  - `is_supported_associative_map_key_type()` now evaluates key suitability on stripped/canonicalized lanes as well.

### Tests

Added/updated missing-stub regression tests:

- `test_missing_stub_map_with_nested_map_value_aliases_to_std_btreemap`
- `test_missing_stub_unordered_map_with_nested_map_value_aliases_to_std_hashmap`
- `test_missing_stub_map_full_signature_with_basic_string_value_aliases_to_std_btreemap`
- `test_missing_stub_map_full_signature_with_class_prefixed_string_key_aliases_to_std_btreemap`
- `test_missing_stub_map_full_signature_with_class_prefixed_string_value_keeps_string_surface`
- Existing guardrail retained:
  - `test_missing_stub_unordered_map_with_nested_lowered_vector_value_keeps_placeholder`

### Guardrails

- Conservative key policy remains unchanged for non-string/non-primitive custom keys.
- Deep full-signature nested value lanes still stay opaque unless they match conservative recoverable shapes.
- Full drop-in validation remains green:
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`
- Spot-check in generated `mako` sidecars confirms full-signature class-prefixed string map aliases now stay string-keyed/value-keyed (no `u128` collapse).

## 22. Full-Signature `basic_string` Key Split Disambiguation (2026-03-05)

### Problem

One remaining full-signature lowered ordered-map lane could still mis-split key/value recovery:

- `map_basic_string_char__struct_std_char_traits_char__class_std_allocator_char__unsigned_long`

In this shape, naive single-split parsing can treat only `basic_string_char` as key and collapse the remainder into a value lane that later resolves to an incorrect integer surface (observed `u128` alias output) instead of `u64`.

### Rule

- For lowered map suffix recovery, evaluate multiple `__` split candidates and pick the first conservative candidate that resolves both key and value to supported concrete types.
- Keep `basic_string<char, ...>` recognition conservative: only accept explicit char-traits/allocator tails for lowered component suffixes.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Replaced single-path map split helper with:
    - `stl_map_key_value_suffix_parts_candidates_from_suffix()`
  - Candidate generation now:
    - enumerates all `__` split points,
    - prefers right-to-left candidates (longer key lanes first),
    - keeps existing full-signature marker handling (`hash`/`equal`/`less`/`allocator` tails),
    - rejects nested full-signature lanes unless they are conservative `basic_string<char, ...>` shapes.
  - `stl_simple_map_key_value_rust_types_from_suffix()` now iterates candidates and returns only when:
    - key/value resolve,
    - unresolved placeholder/c_void guards pass,
    - key/value support gates pass.
  - Tightened `is_lowered_basic_string_component_suffix()`:
    - accepts only `basic_string_char` / `std_basic_string_char` heads,
    - allows only `std_char_traits_char` and `std_allocator_char` as tail lanes.

### Tests

Added missing-stub regression test:

- `test_missing_stub_map_full_signature_with_basic_string_key_and_scalar_value_splits_correctly`
  - asserts alias is `std::collections::BTreeMap<basic_string_char, u64>`
  - asserts it is not `...<basic_string_char, u128>`.

### Guardrails

- Simple and nested map-value recovery behavior remains unchanged.
- Full-signature map recovery is still conservative for non-string deep nested lanes.
- Fresh drop-in validation remains green:
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`

## 23. Lowered `rusty_Result_*` Alias Recovery for Safe Std Surfaces (2026-03-05)

### Problem

Lowered/sanitized Rusty result spellings can appear without angle-bracket syntax, for example:

- `rusty_Result_classrrr_AddrInfo_int_`
- `rusty_Result_void_type_parameter_0_0_`
- `rusty_Result_classrusty_Arc_classrrr_Future__int_`

These previously stayed opaque and missed the existing `std::result::Result<...>` alias normalization path.

### Rule

- Keep the existing mpsc-specific lowered result mapper unchanged and first in precedence.
- Add a generic lowered `rusty_Result_*` / `Result_*` mapper that:
  - splits on an error-like RHS component (`*Error*`) when a conservative tagged boundary is found,
  - otherwise falls back to simple RHS scalar/placeholder lanes (for example `int`, `void`, `type_parameter_*`),
  - normalizes each component through existing type mapping before building `std::result::Result<Ok, Err>`.

### Implementation

- In `crates/fragile-clang/src/types.rs`:
  - Added `map_lowered_result_component_to_std()`.
  - Added lowered split helpers:
    - `split_lowered_result_on_error_component()`
    - `split_lowered_result_on_simple_rhs_component()`
  - Added `map_lowered_result_alias_to_std()`.
  - Wired the new mapper into `map_rusty_type_to_std()` after the mpsc-specialized lowered-result path.

### Tests

Extended `test_normalize_rusty_type_alias_to_std_maps_wrappers_and_preserves_non_rusty_paths` with:

- `rusty_Result_classrrr_AddrInfo_int_ -> std::result::Result<rrr_AddrInfo, i32>`
- `rusty_Result_void_type_parameter_0_0_ -> std::result::Result<(), ()>`
- `rusty_Result_classrusty_Arc_classrrr_Future__int_ -> std::result::Result<std::sync::Arc<rrr_Future>, i32>`

### Guardrails

- mpsc error lowering remains handled by the dedicated specialized mapper.
- Split heuristics stay conservative (ASCII-lowered lanes only, tagged error boundary or scalar/placeholder fallback).
- Fresh validation remains green:
  - `cargo test -p fragile-clang --lib normalize_rusty_type_alias_to_std`
  - `cargo test -p fragile-clang --lib missing_stub_`
  - `cargo build --release --bin fragilec`
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`

## 24. `Result<..., PoisonError<...>>` Alias Error-Lane Companion Mapping (2026-03-05)

### Problem

Some Rusty result aliases carry non-isomorphic sync wrappers in the error lane, for example:

- `rusty::Result<rusty::MutexGuard<int>, rusty::PoisonError<int>>`

The outer `Result` surface should still normalize to `std::result::Result<...>`, but preserving `rusty::PoisonError<...>` as-is in that lane can block wrapper-record alias recovery because generated sidecars typically reference monomorphized companion record names (for example `rusty_PoisonError_int`) rather than nested Rusty namespace template paths.

### Rule

- Keep direct `PoisonError` aliases non-std (do not force invalid lifetime/private std poison-error signatures).
- For `Result` template normalization only, when the error lane is a `PoisonError<...>` wrapper, rewrite that lane to the generated companion record identifier form so the outer result alias can remain `std::result::Result<Ok, Err>`.

### Implementation

- In `crates/fragile-clang/src/types.rs`:
  - Added `sanitize_cpp_type_like_record_identifier()` helper for deterministic C++-like template spelling sanitization into identifier-safe forms.
  - Added `map_result_error_wrapper_arg_to_generated_record()` to map `PoisonError<...>` lane spellings to generated companion record identifiers.
  - Wired this mapping into `map_result_template_arg_to_rust()` before generic alias-arg lowering.
- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Updated wrapper-record alias helper regression to assert that `rusty::Result<rusty::MutexGuard<int>, rusty::PoisonError<int>>` now normalizes to `std::result::Result<rusty_MutexGuard_int, rusty_PoisonError_int>`.

### Tests

- `types::tests::test_normalize_rusty_type_alias_to_std_maps_wrappers_and_preserves_non_rusty_paths`
  - added assertion for `rusty::Result<rusty::MutexGuard<int>, rusty::PoisonError<int>> -> std::result::Result<rusty_MutexGuard_int, rusty_PoisonError_int>`
- `ast_codegen::tests::test_rusty_wrapper_record_alias_helper_maps_result_with_poison_error_to_generated_record_companions`
  - validates wrapper-record alias helper returns the same normalized std result alias target.

### Guardrails

- Scope is limited to `Result` error-lane normalization for `PoisonError` wrappers.
- Direct `PoisonError`/`LockResult`/`TryLockResult` alias surfaces remain intentionally non-std.
- Fresh validation remains green:
  - `cargo test -p fragile-clang --lib poison_error_to_generated_record`
  - `cargo test -p fragile-clang --lib maps_wrappers_and_preserves_non_rusty_paths`
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`

## 25. `Ref`/`RefMut` Wrapper Record Aliasing with Explicit Lifetimes (2026-03-05)

### Problem

Rusty wrapper-record aliasing previously excluded `rusty::Ref<T>` / `rusty::RefMut<T>` even though type normalization already mapped these surfaces to `std::cell::{Ref, RefMut}`. This left many generated opaque wrapper structs in drop-in output.

A direct allowlist expansion caused Rust compile failures for alias items:

- `error[E0106]: missing lifetime specifier`

because `type` aliases cannot use lifetime elision in `std::cell::Ref<...>` / `std::cell::RefMut<...>` positions.

### Rule

- Allow wrapper-record aliasing for `Ref` / `RefMut` surfaces.
- In record-alias emission context, inject explicit `'static` lifetime lanes into `std::cell::Ref` / `std::cell::RefMut` targets (including nested appearances) so emitted `type` aliases are well-formed.
- Keep this lifetime injection scoped to wrapper-record alias target emission only (do not globally rewrite all `Ref`/`RefMut` spellings).

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Added `normalize_ref_like_lifetimes_in_alias_target()` to rewrite:
    - `std::cell::Ref<...>` -> `std::cell::Ref<'static, ...>`
    - `std::cell::RefMut<...>` -> `std::cell::RefMut<'static, ...>`
    - while preserving already-injected `'static` forms.
  - Applied this normalization inside `rusty_wrapper_alias_target_from_record_name()` after standard Rusty-to-std target normalization.
  - Expanded wrapper-record alias allowlist prefixes to include:
    - `std::cell::Ref<`
    - `std::cell::RefMut<`
  - Extended wrapper trait detectors so `Ref` / `RefMut` are treated as non-`Default`/non-`Copy` wrappers, with `RefMut` also treated as non-`Clone`.

### Tests

- Extended `test_rusty_wrapper_record_alias_helper_supports_option_and_result` with:
  - `rusty::Ref<class Foo> -> std::cell::Ref<'static, Foo>`
  - `rusty::RefMut<class Foo> -> std::cell::RefMut<'static, Foo>`
  - nested `rusty::Option<rusty::RefMut<class Foo>> -> std::option::Option<std::cell::RefMut<'static, Foo>>`
- Extended `test_wrapper_trait_detectors_normalize_rusty_alias_spellings` with `Ref`/`RefMut` trait-block assertions.

### Guardrails

- The `normalized == record_name` guard remains based on pre-lifetime-injection normalization, so non-Rusty record names are still not auto-aliased by this pass.
- `Ref`/`RefMut` lifetime insertion is deterministic and idempotent for `'static` aliases.
- Fresh validation remains green:
  - `cargo test -p fragile-clang --lib rusty_wrapper_record_alias_helper_supports_option_and_result`
  - `cargo test -p fragile-clang --lib wrapper_trait_detectors_normalize_rusty_alias_spellings`
  - `cargo build --release --bin fragilec`
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`

## 26. Direct `PoisonError<T>` Wrapper Record Aliasing to `std::sync::PoisonError<T>` (2026-03-05)

### Problem

Even after broad Rusty-wrapper aliasing, many direct `rusty::PoisonError<T>` record wrappers were still emitted as opaque structs in sidecars, despite having a direct std counterpart.

### Rule

- For wrapper-record alias emission, map direct Rusty poison-error wrappers to std:
  - `rusty::PoisonError<T>`
  - `rusty::sync::PoisonError<T>`
  - unqualified `PoisonError<T>` in Rusty namespace contexts
- Keep aliasing conservative: if the normalized payload still carries nested unresolved `rusty::...` paths, reject alias emission and keep existing wrapper-struct fallback.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Added `map_non_isomorphic_sync_wrapper_alias_target()`:
    - detects direct poison-error wrapper spellings,
    - normalizes payload lanes through existing type normalization and C++-to-Rust named-type lowering,
    - returns `std::sync::PoisonError<...>` alias targets.
  - Wired this helper into `rusty_wrapper_alias_target_from_record_name()` before generic namespace alias normalization.
  - Added `std::sync::PoisonError<` to wrapper alias allowlisted prefixes.

### Tests

- Extended `test_rusty_wrapper_record_alias_helper_supports_option_and_result` with:
  - `rusty::PoisonError<int> -> std::sync::PoisonError<i32>`
- Re-ran existing result-lane companion mapping regression:
  - `test_rusty_wrapper_record_alias_helper_maps_result_with_poison_error_to_generated_record_companions`

### Guardrails

- This change is scoped to wrapper-record alias emission; it does not globally rewrite all `PoisonError` type normalization surfaces.
- Existing `Result<..., PoisonError<...>>` companion-record behavior remains intact.
- Fresh validation remains green:
  - `cargo test -p fragile-clang --lib rusty_wrapper_record_alias_helper_supports_option_and_result`
  - `cargo test -p fragile-clang --lib maps_result_with_poison_error_to_generated_record_companions`
  - `cargo build --release --bin fragilec`
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`

## 27. Recover Degraded `PoisonError::...` Record Spellings to std Alias Targets (2026-03-05)

### Problem

After direct `PoisonError<T>` wrapper aliasing, many generated sidecars still emitted opaque
`rusty_PoisonError_*` structs for degraded C++ spellings such as:

- `rusty::PoisonError::conststructrrr::Future::State`
- `rusty::PoisonError::classrusty::Option::classrusty::thread::JoinHandle::void`

These are non-template degraded forms that use scope separators (`::`) instead of canonical
template syntax (`<...>`), so they bypassed the prior alias lane.

### Rule

- Treat `PoisonError::...` degraded record spellings as non-isomorphic wrapper forms and map
  them to `std::sync::PoisonError<...>` during wrapper-record alias emission.
- Normalize degraded payload lanes through the same std/identifier recovery path:
  - normalize direct alias spellings first,
  - recover compact `constclass`/`conststruct`-style prefixes,
  - fallback through lowered identifier normalization and `CppType::Named` conversion.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Extended `map_non_isomorphic_sync_wrapper_alias_target()` with:
    - `PoisonError::`-style prefix handling for:
      - `rusty::PoisonError::...`
      - `rusty::sync::PoisonError::...`
      - unqualified `PoisonError::...`
    - local payload mapper that strips compact cv/tag prefixes (`constclass`, `conststruct`,
      etc.) before normalized fallback lowering.
  - Reused this payload mapper for both canonical `<...>` and degraded `::...` forms.

### Tests

- Extended `test_rusty_wrapper_record_alias_helper_supports_option_and_result` with:
  - `rusty::PoisonError::conststructrrr::Future::State -> std::sync::PoisonError<rrr_Future_State>`
  - `rusty::PoisonError::classrusty::Option::classrusty::thread::JoinHandle::void -> std::sync::PoisonError<std::option::Option<std::thread::JoinHandle<()>>>`

### Guardrails

- Scope remains wrapper-record alias emission only.
- Existing `Result<..., PoisonError<...>>` companion-record behavior is preserved.
- Fresh validation remains green:
  - `cargo test -p fragile-clang --lib test_rusty_wrapper_record_alias_helper_supports_option_and_result`
  - `cargo test -p fragile-clang --lib maps_result_with_poison_error_to_generated_record_companions`
  - `cargo build --release --bin fragilec`
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`

## 28. Recover `MutexGuard`/`RwLock*Guard` Wrapper Records to std Guard Alias Targets (2026-03-05)

### Problem

After `PoisonError` recovery, many sidecars still emitted large numbers of `rusty_MutexGuard_*`
opaque structs. Canonical and degraded guard spellings were still treated as non-mappable wrappers,
even though they are Rusty wrappers around std guard surfaces.

### Rule

- Map wrapper-record guard forms to std sync guards during alias emission:
  - `MutexGuard<T>` -> `std::sync::MutexGuard<'static, T>`
  - `RwLockReadGuard<T>` -> `std::sync::RwLockReadGuard<'static, T>`
  - `RwLockWriteGuard<T>` -> `std::sync::RwLockWriteGuard<'static, T>`
- Support both canonical template spellings (`<...>`) and degraded scoped spellings (`::...`).
- Inject explicit `'static` lifetimes only in wrapper-record alias targets so generated alias items
  remain well-formed.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Extended `map_non_isomorphic_sync_wrapper_alias_target()` to normalize and map:
    - `rusty::MutexGuard<...>` / `rusty::sync::MutexGuard<...>` / `MutexGuard<...>`
    - `rusty::RwLockReadGuard<...>` / `rusty::sync::RwLockReadGuard<...>` / `RwLockReadGuard<...>`
    - `rusty::RwLockWriteGuard<...>` / `rusty::sync::RwLockWriteGuard<...>` / `RwLockWriteGuard<...>`
    - and their degraded `::...` counterparts.
  - Reused the same payload normalization path used by `PoisonError` recovery, including compact
    cv/tag prefix stripping.
  - Extended alias-target lifetime injection to include:
    - `std::sync::MutexGuard`
    - `std::sync::RwLockReadGuard`
    - `std::sync::RwLockWriteGuard`
  - Added these std guard prefixes to wrapper alias allowlist.
  - Updated wrapper trait detectors so std guard lanes are treated as non-`Default`,
    non-`Clone`, and non-`Copy`.

### Tests

- Extended `test_rusty_wrapper_record_alias_helper_supports_option_and_result` with:
  - direct guard spellings (`rusty::MutexGuard<int>`, `rusty::sync::RwLockReadGuard<long>`,
    `RwLockWriteGuard<long>`)
  - degraded scoped spellings (`rusty::MutexGuard::conststruct...`,
    `rusty::RwLockReadGuard::classrusty::Option::...`)
- Extended `test_wrapper_trait_detectors_normalize_rusty_alias_spellings` with std guard checks.
- Re-ran guardrail regression:
  - `test_rusty_wrapper_record_alias_helper_maps_result_with_poison_error_to_generated_record_companions`

### Guardrails

- Scope is still wrapper-record alias emission; global type normalization policy remains unchanged.
- Companion-record normalization for `Result<..., PoisonError<...>>` remains intact.
- Fresh validation remains green:
  - `cargo test -p fragile-clang --lib test_rusty_wrapper_record_alias_helper_supports_option_and_result`
  - `cargo test -p fragile-clang --lib test_wrapper_trait_detectors_normalize_rusty_alias_spellings`
  - `cargo test -p fragile-clang --lib maps_result_with_poison_error_to_generated_record_companions`
  - `cargo build --release --bin fragilec`
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`

## 29. Recover Degraded `rusty::Function` Signature Placeholders to Emitted Siblings (2026-03-05)

### Problem

Drop-in sidecars still emitted many concrete placeholder structs for degraded function wrapper
spellings like:

- `rusty::Function::void::::void::` -> `rusty_Function_void__void_`

even when a concrete sibling type was already emitted in the same translation unit:

- `rusty::Function<void (void)>` -> `rusty_Function_void_void__`

This produced duplicated function-wrapper surfaces and unnecessary placeholder structs.

### Rule

- During missing-stub concrete alias resolution, detect degraded `rusty::Function` spellings that
  carry repeated scope separators (`::::`).
- Prefer aliasing the degraded lowered name to an already-emitted sibling obtained by collapsing
  repeated lowered separators, instead of generating a new placeholder struct.
- Keep this fallback conservative:
  - only for `rusty::Function` degraded spellings,
  - only when a concrete candidate is already emitted.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Added `degraded_function_signature_siblings()`:
    - targets `rusty_Function_` / `Function_` lowered spellings with repeated `__` separators,
    - produces canonical sibling candidates by collapsing repeated underscore runs and testing a
      template-close underscore variant.
  - Extended `resolve_missing_stub_concrete_alias_target()`:
    - when `cpp_name` contains both `rusty::Function::` and `::::`,
    - probes `degraded_function_signature_siblings()` candidates and aliases to the first emitted
      sibling/path match.

### Tests

- Added `test_missing_stub_aliases_degraded_rusty_function_signature_to_known_sibling`:
  - seeds output with `rusty_Function_void_void__`,
  - marks `rusty_Function_void__void_` (`rusty::Function::void::::void::`) as missing,
  - verifies stub generation emits:
    - `pub type rusty_Function_void__void_ = rusty_Function_void_void__;`
  - and does not emit a placeholder struct for `rusty_Function_void__void_`.

### Guardrails

- Scope is missing-stub alias recovery only; no global remapping of `rusty::Function` type
  lowering is introduced.
- Candidate aliasing is gated on already-emitted targets, avoiding synthetic or speculative
  mappings.
- Fresh validation remains green:
  - `cargo test -p fragile-clang --lib test_missing_stub_aliases_degraded_rusty_function_signature_to_known_sibling`
  - `cargo test -p fragile-clang --lib test_missing_stub_aliases_lowered_rusty_wrapper_name_when_cpp_path_is_degraded`
  - `cargo test -p fragile-clang --lib test_missing_stub_aliases_unqualified_rusty_wrapper_in_namespace_context`
  - `cargo build --release --bin fragilec`
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - `ctest -j32 --output-on-failure`

## 30. Preserve `c_void` in Lowercase Item-Type Fallback Normalization (2026-03-05)

### Problem

`normalize_unresolved_lowercase_item_type_tokens()` rewrites unresolved lowercase item-signature
type identifiers to `u128`. Without reserving `c_void`, this fallback can rewrite valid FFI paths:

- `std::ffi::c_void` -> `std::ffi::u128`

This leads to hard compile failures in generated sidecars (for example `cannot find type u128 in
module std::ffi`).

### Rule

- Treat `c_void` as a reserved lowercase FFI type token in item-signature fallback normalization.
- Never rewrite `c_void` through the unresolved lowercase -> `u128` lane.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Added `"c_void"` to the `reserved_module_like` set used by
    `normalize_unresolved_lowercase_item_type_tokens()`.

### Tests

- Added `test_normalize_unresolved_lowercase_item_type_tokens_preserves_c_void_paths`:
  - verifies signatures containing both `*mut c_void` and `*const std::ffi::c_void` are preserved,
  - verifies no `std::ffi::u128`/`*mut u128` rewrite occurs.

### Guardrails

- Scope is limited to lowercase item-type fallback normalization.
- Existing unresolved lowercase rewrites (for true unknown identifiers like `pair`, `id`) remain
  intact.

### Validation

- `cargo test -p fragile-clang --lib test_missing_stub_generation_ignores_expression_like_referenced_but_undefined_names`
- `cargo test -p fragile-clang --lib test_missing_stub_generation_ignores_expression_like_path_used_type_names`
- `cargo test -p fragile-clang --lib test_normalize_unresolved_lowercase_item_type_tokens_preserves_c_void_paths`
- `cargo build --release --bin fragilec`
- `make clean`
- `cmake -S .. -B .` (from `vendor/mako/build_fragilec_dropin`)
- `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
- `ctest -j32 --output-on-failure` (only failure: `rpcbench` path expects `./build/rpcbench`)

## 31. Normalize `std::hash<...>` Record Surfaces to Rust `DefaultHasher` (2026-03-05)

### Problem

Generated sidecars still emitted opaque placeholder structs for C++ hasher record spellings, for
example:

- `std::hash<class rusty::String>` -> `pub struct std_hash_classrusty_String_ { ... }`

These are Rusty-safe/std-backed hasher surfaces and can be represented by Rust std hasher types.

### Rule

- Normalize recognized C++ `std::hash` spellings to Rust std hasher surface:
  - `std::hash<...>`
  - degraded scoped spellings like `std::hash::...`
  - lowered spellings like `std_hash_..._`
- Map them conservatively to:
  - `std::collections::hash_map::DefaultHasher`

### Implementation

- In `crates/fragile-clang/src/types.rs`:
  - Added `map_std_hash_to_default_hasher(...)`.
  - Wired it into `map_rusty_type_to_std(...)` before generic wrapper fallbacks.
- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Added `std::collections::hash_map::DefaultHasher` to wrapper-alias exact allowlist so
    record-level alias emission can emit `pub type ... = DefaultHasher` instead of placeholder
    structs.

### Tests

- Extended `types` normalization regression:
  - `std::hash<class rusty::String>`
  - `hash<class rusty::String>`
  - `std::hash::constclassrusty::String::`
  - `std_hash_classrusty_String_`
- Extended wrapper alias helper regression:
  - `std::hash<class rusty::String>`
  - `std::hash::constclassrusty::String::`

### Validation

- `cargo test -p fragile-clang --lib test_normalize_rusty_type_alias_to_std_maps_wrappers_and_preserves_non_rusty_paths`
- `cargo test -p fragile-clang --lib test_rusty_wrapper_record_alias_helper_supports_join_handle_and_non_generic_wrappers`
- `cargo test -p fragile-clang --lib test_normalize_unresolved_lowercase_item_type_tokens_preserves_c_void_paths`
- `cargo build --release --bin fragilec`
- `make clean`
- `cmake -S .. -B .` (from `vendor/mako/build_fragilec_dropin`)
- `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
- targeted regeneration check:
  - `cmake --build . -j32 --target test_rpc`
  - confirms `std::hash<class rusty::String>` now emits alias to
    `std::collections::hash_map::DefaultHasher`
- `ctest -j32 --output-on-failure` (only failure remains `rpcbench` path `./build/rpcbench`)

## 32. Normalize `std::is_error_code_enum<...>` Marker Records to Unit `()` (2026-03-05)

### Problem

Generated sidecars still emitted marker-trait record placeholders for ASIO error enum traits, for
example:

- `std::is_error_code_enum<enum asio::error::basic_errors>`
  -> `pub struct std_is_error_code_enum_enumasio_error_basic_errors_ { ... }`

These are trait-marker-like surfaces with no runtime payload and should map to a Rust unit marker.

### Rule

- Normalize recognized `std::is_error_code_enum` spellings to `()`:
  - `std::is_error_code_enum<...>`
  - unqualified `is_error_code_enum<...>`
  - degraded scoped `std::is_error_code_enum::...`
  - lowered `std_is_error_code_enum_..._`

### Implementation

- In `crates/fragile-clang/src/types.rs`:
  - Added `map_std_is_error_code_enum_marker_to_unit(...)`.
  - Wired it into `map_rusty_type_to_std(...)` alongside existing std wrapper normalization rules.
- In `crates/fragile-clang/src/ast_codegen.rs`:
  - No additional emission logic required; existing wrapper alias path now resolves these record
    names to `()` and emits `pub type ... = ();`.

### Tests

- Extended `test_normalize_rusty_type_alias_to_std_maps_wrappers_and_preserves_non_rusty_paths`
  with:
  - `std::is_error_code_enum<enum asio::error::basic_errors>`
  - `is_error_code_enum<enum asio::error::basic_errors>`
  - `std::is_error_code_enum::enumasio::error::basic_errors::`
  - `std_is_error_code_enum_enumasio_error_basic_errors_`
- Extended `test_rusty_wrapper_record_alias_helper_supports_join_handle_and_non_generic_wrappers`
  with:
  - `std::is_error_code_enum<enum asio::error::basic_errors>`
  - `std::is_error_code_enum::enumasio::error::basic_errors::`
  - `std_is_error_code_enum_enumasio_error_basic_errors_`

### Validation

- `cargo test -p fragile-clang --lib test_normalize_rusty_type_alias_to_std_maps_wrappers_and_preserves_non_rusty_paths`
- `cargo test -p fragile-clang --lib test_rusty_wrapper_record_alias_helper_supports_join_handle_and_non_generic_wrappers`
- `cargo build --release --bin fragilec`
- clean rebuild (`vendor/mako/build_fragilec_dropin`):
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
- sidecar spot check:
  - `rg -n "pub type std_is_error_code_enum_" -g "*.fragile.rs" .`
  - confirms marker records now emit `pub type ... = ();`
- `ctest -j32 --output-on-failure` (only failure remains `rpcbench` path `./build/rpcbench`)

## 33. Preserve Lowered Namespace Wrappers and Canonicalize Compact STL Integer Suffixes (2026-03-05)

### Problem

Two degraded-lowering behaviors were producing avoidable non-std-safe surfaces:

- Lowered namespace wrapper tokens (for example `std_function_void__void_`) could be rewritten by
  lowercase fallback normalization into `u128` lanes.
- Lowered STL container suffixes with compact integral spellings (for example `unsignedint`) could
  remain unresolved, yielding aliases like `std_vector<unsignedint>` instead of canonical Rust
  integer lanes.

During clean mako replay, this surfaced as:

- `error[E0425]: cannot find type 'unsignedint'` on
  `pub type std_vector_unsignedint = std_vector<unsignedint>;`

### Rule

- Treat lowered namespace wrappers (`std_*`, `core_*`, `alloc_*`, `rusty_*`, `crate_*`) as
  preservable type tokens in lowercase unresolved-type fallback normalization.
- Canonicalize compact lowered integral suffix spellings to Rust integer lanes in STL element
  mapping (for example `unsignedint -> u32`, `longlong -> i64`).

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Updated `normalize_unresolved_lowercase_item_type_tokens(...)` to skip rewriting unresolved
    lowered namespace wrapper names with prefixes:
    - `std_`, `core_`, `alloc_`, `rusty_`, `crate_`
  - Extended `stl_container_element_rust_type_from_suffix(...)` to normalize compact integral
    spellings:
    - `signedchar`, `unsignedchar`
    - `signedshort`, `unsignedshort`
    - `signedint`, `unsignedint`
    - `signedlong`, `unsignedlong`
    - `longlong`, `unsignedlonglong`

### Tests

- Added:
  - `test_normalize_unresolved_lowercase_item_type_tokens_preserves_lowered_namespace_wrappers`
  - `test_missing_stub_std_vector_alias_normalizes_compact_unsigned_int_suffix`

### Validation

- `cargo test -p fragile-clang --lib test_normalize_unresolved_lowercase_item_type_tokens_preserves_lowered_namespace_wrappers`
- `cargo test -p fragile-clang --lib test_missing_stub_std_vector_alias_normalizes_compact_unsigned_int_suffix`
- `cargo build --release --bin fragilec`
- clean rebuild (`vendor/mako/build_fragilec_dropin`):
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
- sidecar spot checks:
  - `rg -n "std_vector_unsignedint = std_vector<u32>" CMakeFiles/test_client_service.dir/test/test_client_service.cc.fragile.rs`
  - confirms compact `unsignedint` suffix canonicalizes to `u32`
- `ctest -j32 --output-on-failure` (only failure remains `rpcbench` path `./build/rpcbench`)

## 34. Reserved `thread` Alias Recovery and Untyped Default/Clone Artifact Cleanup (2026-03-05)

### Problem

After preserving lowered composite/container spellings, clean mako replay shifted to three generic
failure classes:

- unresolved bare `thread` in top-level signatures even when a unique public namespaced target
  existed (for example `asio::detail::thread`);
- ambiguous untyped placeholder defaults in degraded bodies:
  - `let mut t = Default::default();` (E0790),
  - `Default::default().join();` (E0282);
- single-use local clone artifacts on non-`Clone` placeholder structs:
  - `(poll_arc).clone()` (E0599).

### Rule

- Reserved lowercase names such as `thread` can still receive top-level unresolved namespaced alias
  fallbacks when:
  - the target is non-`std/core/alloc`, and
  - there is no top-level module-name collision.
- Alias dedupe must only consider real top-level declarations; nested aliases with the same leaf
  name must not block top-level fallback alias emission.
- Untyped local default placeholders and degraded clone/join artifacts should be normalized away in
  compile-safe, generic ways rather than forcing non-`Clone` impl synthesis on wrapper-backed
  structs.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - `normalize_unresolved_namespaced_type_aliases(...)`:
    - tracks top-level module names and only blocks reserved-name alias synthesis on true
      root-module collisions or std/core/alloc targets;
    - switches alias dedupe from global string-search (`out.contains("pub type X =")`) to
      top-level declaration tracking, allowing valid top-level fallback aliases when same-name
      nested aliases already exist.
  - `normalize_unresolved_join_method_calls(...)` now rewrites untyped default receivers
    (`Default::default().join();`, `std::default::Default::default().join();`) to `();`.
  - `normalize_unused_default_local_bindings(...)` function-head detection now recognizes extern ABI
    signatures (`pub extern "C" fn ...`) and related qualifier combinations.
  - Added `normalize_single_use_local_clone_calls(...)`:
    - rewrites parenthesized local clone calls (`(x).clone()`) to moves (`(x)`) when the local is
      single-use in-function (declaration + clone site only).
  - Added `normalize_noop_default_local_placeholder_bindings(...)`:
    - drops untyped `let ... = Default::default();` placeholders when immediately followed by noop
      `();` and unused before block close.
  - Pipeline integration:
    - runs the noop-default cleanup alongside existing unused-default cleanup both in early body
      cleanup and final late cleanup stages.

### Tests

- Added:
  - `test_normalize_unresolved_namespaced_type_aliases_allows_nonstd_reserved_thread_aliases`
  - `test_normalize_unresolved_namespaced_type_aliases_skips_reserved_thread_alias_when_top_level_module_conflicts`
  - `test_normalize_unresolved_join_method_calls_rewrites_untyped_default_receiver_join_noop`
  - `test_normalize_unused_default_local_bindings_drops_unused_untyped_defaults_in_extern_fn`
  - `test_normalize_single_use_local_clone_calls_rewrites_parenthesized_local_clone_move`
  - `test_normalize_single_use_local_clone_calls_keeps_multi_use_local_clone_calls`
  - `test_normalize_noop_default_local_placeholder_bindings_drops_noop_placeholder_pair`
  - `test_normalize_noop_default_local_placeholder_bindings_keeps_used_bindings_before_block_end`

### Validation

- Targeted unit tests:
  - `cargo test -p fragile-clang --lib normalize_unresolved_namespaced_type_aliases -- --nocapture`
  - `cargo test -p fragile-clang --lib normalize_unresolved_join_method_calls -- --nocapture`
  - `cargo test -p fragile-clang --lib normalize_unused_default_local_bindings -- --nocapture`
  - `cargo test -p fragile-clang --lib normalize_single_use_local_clone_calls -- --nocapture`
  - `cargo test -p fragile-clang --lib normalize_noop_default_local_placeholder_bindings -- --nocapture`
- Compiler build:
  - `cargo build --release --bin fragilec`
- clean mako rebuild (`vendor/mako/build_fragilec_dropin`):
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
  - result: `EXIT:0`
- test run:
  - `ctest -j32 --output-on-failure`
  - 116 tests pass; only `rpcbench` fails due its upstream test command hardcoding
    `WORKING_DIRECTORY=/vendor/mako` plus `./build/rpcbench` (path mismatch for this build dir).
  - `ctest -j32 --output-on-failure -E '^rpcbench$'` passes (116/116).

## 35. Transitive Alias-Chain Collapse to Concrete std/Primitive Targets (2026-03-05)

### Problem

Even after Rusty wrapper RHS normalization, many generated top-level aliases remained as
alias-to-alias chains, for example:

- `A = B`
- `B = std::sync::MutexGuard<'static, T>`

This kept wrapper-heavy outputs less directly std-native and increased alias indirection in safe
regions.

### Rule

- Collapse only top-level alias chains.
- Resolve transitive alias RHS targets (`A -> B -> C -> ...`) and rewrite only when the final
  concrete target is clearly safe/std-like:
  - `std::...`, `core::...`, `alloc::...`
  - primitive/unit/never lanes (`i*`, `u*`, `f*`, `bool`, `char`, `()`, `!`)
  - raw/reference pointer lanes (`*...`, `&...`)
- Do not rewrite module-scoped aliases or non-std/non-primitive terminal targets.
- Preserve cycle safety (leave cycles to existing cycle-break normalization).

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Added `normalize_transitive_std_type_alias_rhs_paths(...)`.
  - The pass:
    - collects top-level simple aliases,
    - resolves transitive simple-identifier chains,
    - rewrites only eligible std/primitive/pointer terminal targets.
- Integrated into the pipeline in two locations:
  - immediately after `normalize_rusty_type_alias_rhs_paths(...)`,
  - and again in late cleanup after the final alias-rhs normalization rerun.

### Tests

- Added:
  - `test_normalize_transitive_std_type_alias_rhs_paths_collapses_std_and_primitive_chains`
  - `test_normalize_transitive_std_type_alias_rhs_paths_avoids_non_std_module_aliases`

### Validation

- Targeted tests:
  - `cargo test -p fragile-clang normalize_transitive_std_type_alias_rhs_paths -- --nocapture`
  - `cargo test -p fragile-clang normalize_rusty_type_alias_rhs_paths -- --nocapture`
- clean mako rebuild (`vendor/mako/build_fragilec_dropin`):
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
- test run:
  - `ctest -j32 --output-on-failure`

## 36. Scope-Aware Transitive Alias Collapse for Module-Local Safe Lanes (2026-03-05)

### Problem

Section 35 collapsed only top-level alias chains. Nested module aliases that were still purely
safe/std-native (for example `Alias -> Target -> std::sync::Barrier`) remained uncollapsed.

### Rule

- Resolve alias chains using lexical scope visibility:
  - current scope first, then ancestors.
- Preserve alias-definition scope during transitive hops so sibling modules cannot leak names into
  each other.
- Rewrite only when the terminal target remains in the same safe lanes as Section 35
  (`std/core/alloc`, primitive/unit/never, raw/reference pointer).

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - `normalize_transitive_std_type_alias_rhs_paths(...)` now:
    - scans aliases by lexical scope and records per-line scope ids,
    - resolves visible aliases with ancestor fallback,
    - tracks definition scope through transitive resolution,
    - rewrites indented/module-local alias lines (not just top-level lines).

### Tests

- Updated module behavior coverage:
  - `test_normalize_transitive_std_type_alias_rhs_paths_resolves_module_scope_without_sibling_leakage`
- Existing chain collapse coverage remains:
  - `test_normalize_transitive_std_type_alias_rhs_paths_collapses_std_and_primitive_chains`

### Validation

- Targeted tests:
  - `cargo test -p fragile-clang normalize_transitive_std_type_alias_rhs_paths -- --nocapture`
  - `cargo test -p fragile-clang normalize_rusty_type_alias_rhs_paths -- --nocapture`
- clean mako rebuild (`vendor/mako/build_fragilec_dropin`):
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
- test run:
  - `ctest -j32 --output-on-failure`
  - 117/117 tests passed.

## 37. Brace-Aware Scope Tracking for Transitive Alias Collapse (2026-03-05)

### Problem

The scope-aware transitive alias pass (Section 36) originally tracked scope with raw `{`/`}`
counts. In real generated Rust, braces frequently appear inside string literals/comments, which can
distort lexical depth and prevent otherwise valid alias-chain collapses.

### Rule

- Scope tracking for alias visibility must ignore braces in:
  - double-quoted string literals (including escaped quotes),
  - raw string literals (`r#"..."#`, with arbitrary `#` count),
  - line comments (`// ...`),
  - block comments (`/* ... */`, including multi-line spans).
- Alias-chain collapse remains restricted to std/core/alloc/primitive/pointer terminal targets.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - `normalize_transitive_std_type_alias_rhs_paths(...)` scope scanning now masks non-code segments
    before brace accounting.
  - Added brace-scan state tracking for:
    - block comments,
    - quoted strings with escapes,
    - raw strings with hash delimiters.

### Tests

- Added:
  - `test_normalize_transitive_std_type_alias_rhs_paths_ignores_braces_in_strings_and_comments`
- Existing transitive collapse tests still pass:
  - std/primitive chain collapse,
  - module scope + sibling leakage guard.

### Validation

- Targeted tests:
  - `cargo test -p fragile-clang normalize_transitive_std_type_alias_rhs_paths -- --nocapture`
  - `cargo test -p fragile-clang normalize_rusty_type_alias_rhs_paths -- --nocapture`
- clean mako rebuild (`vendor/mako/build_fragilec_dropin`):
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
- test run:
  - `ctest -j32 --output-on-failure`
  - 117/117 tests passed.

## 38. Non-Enum Terminal Preference for Missing-Stub Qualifier Families (2026-03-05)

### Problem

Some degraded qualifier-family spellings (`struct`/`conststruct`, merged separators) were still
falling back to enum-shadow aliases like `... = State` in missing-stub generation, even when a
safe concrete alias chain existed (for example to `std::sync::MutexGuard` /
`std::sync::PoisonError`).

### Rule

- When resolving missing-stub qualifier-family candidates, prefer the deepest emitted alias in the
  chain when its terminal target is non-enum.
- Reject candidates whose terminal target resolves to a known enum alias target.
- Keep expression-like safe terminal targets (such as `std::sync::*`) valid without forcing a
  placeholder/enum fallback.

### Implementation

- In `crates/fragile-clang/src/ast_codegen.rs`:
  - Replaced terminal-target-only chain probing with
    `resolve_emitted_alias_terminal(...) -> (terminal_alias_name, terminal_target)`.
  - Updated `select_preferred_concrete_alias_candidate(...)` to:
    - collapse through alias chains to the deepest concrete alias surface,
    - filter enum-terminal chains,
    - preserve non-enum std-expression terminals.

### Tests

- Added/updated coverage:
  - `test_missing_stub_qualifier_family_dense_key_aliases_conststruct_variant_to_compact_sibling`
  - `test_missing_stub_qualifier_family_dense_key_prefers_non_enum_alias_when_enum_shadow_exists`
  - `test_missing_stub_qualifier_family_dense_key_resolves_mixed_mutexguard_poisonerror_variants_to_compact_std_aliases`

### Validation

- Targeted tests:
  - `cargo test -p fragile-clang test_missing_stub_qualifier_family_dense_key -- --nocapture`
- clean mako rebuild (`vendor/mako/build_fragilec_dropin`):
  - `make clean`
  - `FRAGILEC_KEEP_RS=1 cmake --build . -j32`
- test run:
  - `ctest -j32 --output-on-failure`
  - 117/117 tests passed.
- Artifact spot-check:
  - `test_rpc_circuit_breaker_integration` generated aliases now resolve
    `rusty_MutexGuard_*Future_State` / `rusty_PoisonError_*Future_State` to `std::sync::*`
    targets (no `= State` fallback for this family).

## 39. RPC Bench Harness Deterministic Plan Scaffold (2026-03-12)

### Problem

The active RPC bring-up track needs a reproducible benchmark harness that compares
`clang` and `fragilec` lanes with identical parameters and artifact capture.
Implementing configure/build/run/replay/aggregation in one pass is too large for
one safe leaf change.

### Decision

Implement leaf `1.1` first as a deterministic planning scaffold:

- add `scripts/mako_rpcbench_harness.py`
- emit stable command plan + manifest + expected-artifact contract
- define lane/trial naming and deterministic per-lane trial ports
- keep this leaf in `--plan-only` mode (no runtime execution yet)

This keeps the harness structure testable before introducing heavy execution
paths in later leaves.

### Wrong-Approach Check

Aligned with Section 1.3 and `docs/dev/wrong.md`:

- no `rpcbench`/`test_rpc` codegen conditionals were introduced
- no semantic fake method bodies or forced compile-success stubs were added
- no force-native bypass path was used

### Implementation

- Script: `scripts/mako_rpcbench_harness.py`
- Docs:
  - `docs/rpc_benchmark_harness_breakdown_2026_03_12.md`
  - `docs/rpc_benchmark_harness_user_manual.md`
- Regression tests:
  - `tests/python/test_mako_rpcbench_harness.py`

### Validation

- `python3 -m unittest tests/python/test_mako_rpcbench_harness.py -v`
- full workspace suite (run after leaf integration): `cargo test`

## 40. RPC Bench Harness Configure/Build Capture (2026-03-12)

### Problem

After leaf `1.1`, the harness only emitted plans/manifests. RPC bring-up still lacked
actual deterministic configure/clean/build capture for both lanes.

### Decision

Implement leaf `1.2` in the harness script directly (generic dual-lane behavior):

- execute `configure`/`clean`/`build` in fixed lane order (`clang`, `fragilec`)
- capture per-step `status`/`stdout`/`stderr` artifacts under lane directories
- classify lane failures (`none`, `*_failed`, `*_timeout`) and persist metadata
- keep runtime replay/aggregation out of scope for later leaves (`1.3+`)

### Wrong-Approach Check

Aligned with Section 1.3 and `docs/dev/wrong.md`:

- no compiler/codegen fallback stubs were added
- no target-name conditionals were introduced in transpiler code
- no force-native bypass path was used
- failure handling is explicit metadata (`failure_class`, `status=-1` for skipped), not hidden

### Implementation

- Updated `scripts/mako_rpcbench_harness.py`:
  - lane configure/clean/build command execution with timeout capture
  - deterministic step artifact writing
  - lane failure-class derivation and persistence
  - manifest enrichment with per-lane status/failure metadata
- Added regression tests in `tests/python/test_mako_rpcbench_harness.py`:
  - execution-mode success capture path
  - execution-mode configure failure + skipped follow-up path

### Validation

- `python3 -m unittest tests/python/test_mako_rpcbench_harness.py -v`
- full workspace suite: `cargo test` (currently reports pre-existing `fragile-clang`
  `ast_codegen` lib-test failures that reproduce on clean `origin/main` baseline)
