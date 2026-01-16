//! Runtime library for C++ feature support in Fragile compiler.
//!
//! This library provides runtime support for C++ features that cannot be
//! directly expressed in MIR, including:
//!
//! - Exception handling (try/catch/throw)
//! - RAII (automatic destructor calls)
//! - Virtual function dispatch
//! - new/delete operators
//!
//! # Architecture
//!
//! C++ features are lowered to runtime function calls during MIR generation.
//! For example:
//!
//! ```cpp
//! try {
//!     might_throw();
//! } catch (const std::exception& e) {
//!     handle(e);
//! }
//! ```
//!
//! Becomes:
//!
//! ```text
//! fragile_rt_try_begin();
//! might_throw();  // may call fragile_rt_throw
//! if (fragile_rt_check_exception()) {
//!     e = fragile_rt_catch();
//!     handle(e);
//! }
//! fragile_rt_try_end();
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod exceptions;
mod memory;
mod vtable;

pub use exceptions::*;
pub use memory::*;
pub use vtable::*;

/// Runtime version for compatibility checking.
pub const RUNTIME_VERSION: u32 = 1;

/// Initialize the Fragile runtime.
///
/// This must be called before using any runtime features.
/// It is automatically called by the generated main function.
#[no_mangle]
pub extern "C" fn fragile_rt_init() {
    // Initialize exception handling state
    exceptions::init_exception_handling();
}

/// Shutdown the Fragile runtime.
///
/// This is called automatically when the program exits.
#[no_mangle]
pub extern "C" fn fragile_rt_shutdown() {
    // Clean up any pending exceptions
    exceptions::cleanup_exception_handling();
}

/// Panic handler for no_std builds.
#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
