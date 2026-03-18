// STL stub implementations for the Fragile C++-to-Rust transpiler.
//
// These are hand-written Rust implementations of C++ STL types that get
// emitted as a "preamble" in every generated .rs file. The libc++ implementations
// are too template-heavy to transpile directly, so we provide simplified stubs
// that cover enough surface area for user code to compile and run.
//
// Each module file is included via include!() so all types share a flat namespace,
// matching how they appear when inlined into generated code.

#![allow(
    dead_code,
    unused_variables,
    unused_mut,
    unused_imports,
    unused_assignments
)]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(unused_parens, unused_unsafe)]
#![allow(clippy::all)]
#![allow(static_mut_refs)]

use std::io::Write;

pub mod layout_contract;

include!("file_header.rs");
include!("array_helpers.rs");
include!("comparison.rs");
include!("vector.rs");
include!("container_adapters.rs");
include!("string.rs");
include!("smart_ptr.rs");
include!("algorithm.rs");
include!("tree.rs");
include!("ordered_map.rs");
include!("unordered_map.rs");
include!("hash.rs");
include!("numeric.rs");
include!("locale.rs");
include!("io.rs");
include!("exception.rs");
include!("math.rs");
include!("clib.rs");
include!("runtime.rs");
