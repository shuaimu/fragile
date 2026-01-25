//! C++ exception handling support.
//!
//! This module implements exception handling using a thread-local exception
//! state. It's designed to be compatible with C++ exception ABI on supported
//! platforms.

use core::ffi::c_void;

#[cfg(feature = "std")]
use std::cell::RefCell;

#[cfg(not(feature = "std"))]
use core::cell::RefCell;

/// A C++ exception object.
#[repr(C)]
pub struct CppException {
    /// Type info pointer (for RTTI)
    pub type_info: *const c_void,
    /// Exception data
    pub data: *mut c_void,
    /// Destructor function (if any)
    pub destructor: Option<unsafe extern "C" fn(*mut c_void)>,
}

impl CppException {
    /// Create a null/empty exception.
    pub const fn null() -> Self {
        Self {
            type_info: core::ptr::null(),
            data: core::ptr::null_mut(),
            destructor: None,
        }
    }

    /// Check if this exception is null/empty.
    pub fn is_null(&self) -> bool {
        self.data.is_null()
    }
}

/// Thread-local exception handling state.
struct ExceptionState {
    /// Current exception (if any)
    current_exception: Option<CppException>,
    /// Stack of try block handlers
    try_stack_depth: usize,
    /// Whether we're currently unwinding
    unwinding: bool,
}

impl ExceptionState {
    const fn new() -> Self {
        Self {
            current_exception: None,
            try_stack_depth: 0,
            unwinding: false,
        }
    }
}

#[cfg(feature = "std")]
thread_local! {
    static EXCEPTION_STATE: RefCell<ExceptionState> = const { RefCell::new(ExceptionState::new()) };
}

#[cfg(not(feature = "std"))]
static mut EXCEPTION_STATE: ExceptionState = ExceptionState::new();

/// Initialize exception handling (called by fragile_rt_init).
pub fn init_exception_handling() {
    // Nothing special needed for now
}

/// Clean up exception handling (called by fragile_rt_shutdown).
pub fn cleanup_exception_handling() {
    #[cfg(feature = "std")]
    {
        EXCEPTION_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(ex) = state.current_exception.take() {
                // Call destructor if present
                if let Some(dtor) = ex.destructor {
                    unsafe { dtor(ex.data) };
                }
            }
        });
    }

    #[cfg(not(feature = "std"))]
    unsafe {
        if let Some(ex) = EXCEPTION_STATE.current_exception.take() {
            if let Some(dtor) = ex.destructor {
                dtor(ex.data);
            }
        }
    }
}

/// Begin a try block.
///
/// Called at the start of a try block to set up exception handling.
#[no_mangle]
pub extern "C" fn fragile_rt_try_begin() {
    #[cfg(feature = "std")]
    {
        EXCEPTION_STATE.with(|state| {
            state.borrow_mut().try_stack_depth += 1;
        });
    }

    #[cfg(not(feature = "std"))]
    unsafe {
        EXCEPTION_STATE.try_stack_depth += 1;
    }
}

/// End a try block.
///
/// Called at the end of a try block to clean up exception handling.
#[no_mangle]
pub extern "C" fn fragile_rt_try_end() {
    #[cfg(feature = "std")]
    {
        EXCEPTION_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.try_stack_depth > 0 {
                state.try_stack_depth -= 1;
            }
        });
    }

    #[cfg(not(feature = "std"))]
    unsafe {
        if EXCEPTION_STATE.try_stack_depth > 0 {
            EXCEPTION_STATE.try_stack_depth -= 1;
        }
    }
}

/// Throw a C++ exception.
///
/// This stores the exception and begins unwinding.
#[no_mangle]
pub extern "C" fn fragile_rt_throw(exception: CppException) -> ! {
    #[cfg(feature = "std")]
    {
        EXCEPTION_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.current_exception = Some(exception);
            state.unwinding = true;
        });

        // In a real implementation, this would use platform-specific
        // unwinding (e.g., _Unwind_RaiseException on Unix)
        panic!("C++ exception thrown");
    }

    #[cfg(not(feature = "std"))]
    unsafe {
        EXCEPTION_STATE.current_exception = Some(exception);
        EXCEPTION_STATE.unwinding = true;

        // No unwinding in no_std - just abort
        loop {}
    }
}

/// Check if an exception is pending.
///
/// Returns true if an exception has been thrown and not yet caught.
#[no_mangle]
pub extern "C" fn fragile_rt_check_exception() -> bool {
    #[cfg(feature = "std")]
    {
        EXCEPTION_STATE.with(|state| state.borrow().current_exception.is_some())
    }

    #[cfg(not(feature = "std"))]
    unsafe {
        EXCEPTION_STATE.current_exception.is_some()
    }
}

/// Catch the current exception.
///
/// Returns the current exception and clears the exception state.
#[no_mangle]
pub extern "C" fn fragile_rt_catch() -> CppException {
    #[cfg(feature = "std")]
    {
        EXCEPTION_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.unwinding = false;
            state
                .current_exception
                .take()
                .unwrap_or(CppException::null())
        })
    }

    #[cfg(not(feature = "std"))]
    unsafe {
        EXCEPTION_STATE.unwinding = false;
        EXCEPTION_STATE
            .current_exception
            .take()
            .unwrap_or(CppException::null())
    }
}

/// Rethrow the current exception.
///
/// Used for `throw;` with no argument.
#[no_mangle]
pub extern "C" fn fragile_rt_rethrow() -> ! {
    #[cfg(feature = "std")]
    {
        let has_exception =
            EXCEPTION_STATE.with(|state| state.borrow().current_exception.is_some());

        if has_exception {
            panic!("C++ exception rethrown");
        } else {
            panic!("No exception to rethrow");
        }
    }

    #[cfg(not(feature = "std"))]
    unsafe {
        loop {}
    }
}

/// Get the current exception without catching it.
///
/// Used for examining the exception in a catch block.
#[no_mangle]
pub extern "C" fn fragile_rt_current_exception() -> *const CppException {
    #[cfg(feature = "std")]
    {
        EXCEPTION_STATE.with(|state| {
            let state = state.borrow();
            state
                .current_exception
                .as_ref()
                .map(|ex| ex as *const CppException)
                .unwrap_or(core::ptr::null())
        })
    }

    #[cfg(not(feature = "std"))]
    unsafe {
        EXCEPTION_STATE
            .current_exception
            .as_ref()
            .map(|ex| ex as *const CppException)
            .unwrap_or(core::ptr::null())
    }
}

/// Check if the current exception matches a type.
///
/// Used for catch clause type matching.
#[no_mangle]
pub extern "C" fn fragile_rt_exception_matches(type_info: *const c_void) -> bool {
    if type_info.is_null() {
        // catch(...) matches everything
        return fragile_rt_check_exception();
    }

    #[cfg(feature = "std")]
    {
        EXCEPTION_STATE.with(|state| {
            let state = state.borrow();
            if let Some(ref ex) = state.current_exception {
                // Simple pointer comparison for now
                // Real implementation would do RTTI inheritance checking
                ex.type_info == type_info
            } else {
                false
            }
        })
    }

    #[cfg(not(feature = "std"))]
    unsafe {
        if let Some(ref ex) = EXCEPTION_STATE.current_exception {
            ex.type_info == type_info
        } else {
            false
        }
    }
}
