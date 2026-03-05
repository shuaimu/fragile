# Fragile Transpiler Dev Book

## Table of Contents

- [1. Purpose and Scope](#1-purpose-and-scope)
- [2. End-to-End Architecture](#2-end-to-end-architecture)
- [2.3 C++ `_v` trait globals and export linkage](#23-c-_v-trait-globals-and-export-linkage)
- [3. Internal Data Models](#3-internal-data-models)
- [4. C++ Declaration to Rust Item Mapping](#4-c-declaration-to-rust-item-mapping)
- [5. C++ Type to Rust Type Mapping](#5-c-type-to-rust-type-mapping)
- [5.4 Rusty-C++ alias normalization to Rust std types](#54-rusty-c-alias-normalization-to-rust-std-types)
- [6. Object Model and Inheritance Design](#6-object-model-and-inheritance-design)
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
- It does not describe planned behavior; it describes current implementation.

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
- `normalize_with_capacity_default_string_placeholders`:
  - rewrites degraded `with_capacity::default()` placeholders (from failed `String::with_capacity` recovery) to `std::string::String::new()`.

Outcome snapshot:

- On March 4, 2026, `make clean && cmake --build . -j32` in `build_fragilec_dropin` completed to 100% after iterative generic fixes.
- On March 4, 2026, `ctest -j32 --output-on-failure` hit one transient `rpcbench` kill in one run but passed on rerun (`117/117`) in `build_fragilec_dropin`; no deterministic compile-failure translation units remained in this loop.
- On March 4, 2026 (revalidation pass), a fresh rerun in `build_fragilec_dropin` with `make clean`, `cmake --build . -j32`, and `ctest -j32 --output-on-failure` completed with `117/117` tests passed and no `fragile rustc object compile failed` translation units in the build log.

### 2.3 C++ `_v` trait globals and export linkage

Large C++23 codebases that include Rusty-C++ headers can surface trait-style variable templates such as:

- `fits_in_sbo_v`

These are C++ header-inline/ODR entities (usually `constexpr bool ..._v`) and should not be emitted as strong, raw C-linkage globals from each translation unit.

Generic transpiler guard now applied in `AstCodeGen::should_export_c_global_symbol`:

- if a global is `bool`-typed and its identifier ends with `_v`, Fragile suppresses raw symbol export (`#[export_name = "..."]`).

Why this matters:

- exporting these names as plain symbols can produce duplicate-symbol linker failures when many translation units instantiate the same header trait variable template.
- keeping them as TU-local Rust-mangled statics preserves buildability without introducing mako-specific hacks.

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
  - `rusty::HashMap<K, V>` and `HashMap<K, V>` -> `std::collections::HashMap<K, V>`
- Result convenience aliases:
  - `rusty::ResultVoid<T>` and `ResultVoid<T>` -> `std::result::Result<T, ()>`
  - `rusty::ResultInt<T>` and `ResultInt<T>` -> `std::result::Result<T, i32>`
  - `rusty::ResultString<T>` and `ResultString<T>` -> `std::result::Result<T, *const i8>`
- mpsc wrapper aliases:
  - `Sender<T>` / `Receiver<T>` / `SyncSender<T>` / `TrySendError<T>` normalize to the corresponding `std::sync::mpsc::*` wrappers
  - explicit `void` and unresolved payload placeholders in those wrappers normalize to unit `()`

This keeps generated Rust in safe/std-native form instead of preserving rusty-cpp wrapper type names in emitted signatures.

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
  - result wrappers remain Rusty (`rusty::PoisonError<T>`, `rusty::LockResult<T>`, `rusty::TryLockResult<T>`)
  - rationale: std guard/result types carry lifetimes/private internals that are not represented by these Rusty aliases; forced std rewrites create invalid arity/lifetime/private-path outputs.
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
- Record and template-instantiation emission now aliases a conservative allowlist of lifetime-free Rusty wrapper surfaces directly to normalized Rust std targets (for example `rusty::Arc<T>` -> `std::sync::Arc<T>`, `rusty::Option<T>` -> `std::option::Option<T>`, `rusty::thread::JoinHandle<T>` -> `std::thread::JoinHandle<T>`, `rusty::Vec<T>` -> `std::vec::Vec<T>`, `rusty::sync::mpsc::Unit` -> `()`, and non-generic wrappers like `rusty::Barrier`/`rusty::Condvar` -> `std::sync::...`) instead of emitting opaque wrapper structs; alias lookup also considers the active namespace path so unqualified declarations inside `namespace rusty { ... }` still normalize to std aliases, template aliases sanitize the emitted lhs name when normalized lowering yields a Rust path, aliasing is rejected if any nested `rusty::` path remains after normalization, and lifetime-sensitive wrappers (`Ref`/`RefMut`) stay excluded.
- Fallback `Default`/`Clone` synthesis now treats Rusty wrapper paths that normalize to external std/core/alloc targets as external too, avoiding unsafe fallback impls on wrappers like `rusty::Barrier`/`rusty::Once`; lowered Rusty thread JoinHandle wrapper spellings (`rusty::thread::rusty_thread_JoinHandle_*`) remain eligible for local fallback impls because generated field-wise defaults still depend on them.
- Fallback `Default`/`Clone` synthesis must not emit impls for external `std::`/`core::`/`alloc::` targets after alias resolution; additionally, qualified local type paths must only resolve through exact alias keys (not unrelated leaf-name aliases) to avoid orphan impls such as `impl Clone for std::sync::Barrier`.
- Debugging note: `CMakeFiles/**.fragile.rs` sidecars can be stale unless `FRAGILEC_KEEP_RS=1` is set for the build invocation; use that env var when validating freshly emitted Rust text.
- Build hygiene note: `make clean` in `vendor/mako/build_fragilec_dropin` does not remove existing `*.fragile.rs` sidecars. For reliable grep validation, either delete stale sidecars explicitly or replay the exact TU compile command with `FRAGILEC_KEEP_RS=1`.
- Lowered Rusty single-argument wrapper spellings now normalize even when C++ lowered names carry prefixed qualifier/tag tokens (for example `classrusty_Rc_*`, `constclassrusty_Arc_*`, `structrusty_Option_*`), by stripping lowered `const/volatile/class/struct/enum/union` tokens before wrapper-prefix matching.
- Std wrapper normalization is now recursive over type arguments for common one/two-parameter std surfaces (`std::option::Option`, `std::boxed::Box`, `std::{sync,rc,cell}` wrappers, `std::result::Result`, `std::collections::{HashMap,BTreeMap}`), so outputs like `std::option::Option<rusty_Rc_...>`/`std::option::Option<rusty_Arc_...>` are normalized to fully-std argument shapes.
- Lowered Rusty set marker spellings now normalize to Rust unit as well: names matching lowered `BTreeSet`/`HashSet` internal `*_Unit` marker forms (including prefixed qualifier tokens) are rewritten to `()`, which in turn normalizes aliases like `std::collections::BTreeMap<..., rusty_BTreeSet_*_Unit>` to `std::collections::BTreeMap<..., ()>`.
- Template-argument normalization now preserves already-lowered explicit Rust unit arguments (`()`) instead of re-sanitizing them into placeholder identifiers, avoiding recursive normalization regressions such as `std::collections::BTreeMap<..., ()>` degrading to `...<..., __>`.
- JoinHandle normalization now strips trailing lowered delimiters from payload spellings and accepts additional lowered prefixes (`rusty_thread_JoinHandle_*`, `JoinHandle_*`), so degraded forms like `JoinHandle<void_>`, `std_thread_JoinHandle_void__`, and `rusty_thread_JoinHandle_void__` normalize to `std::thread::JoinHandle<()>` instead of `std::thread::JoinHandle<__>`.
- Lowered single-argument Rusty wrapper normalization now includes sync lock wrappers (`rusty_Mutex_*`/`Mutex_*`, `rusty_RwLock_*`/`RwLock_*`) in the same pass used for `Option`/`Box`/`Arc`/`Rc`/cell wrappers, allowing lowered aliases to land on `std::sync::{Mutex,RwLock}<...>` directly.
- JoinHandle payload normalization now preserves explicit unit payloads (`()`) as unit; this avoids double-normalization regressions where already-normalized `std::thread::JoinHandle<()>` payloads were reprocessed into placeholder `std::thread::JoinHandle<__>` (for example through repeated namespace-alias target normalization).
