//! POSIX threads (pthread) wrappers for transpiled C++ code.
//!
//! This module provides pthread-compatible functions that transpiled C++ code
//! can call. When C++ code uses `std::thread`, libc++ internally calls pthread
//! functions which are implemented here using Rust's std::thread.
//!
//! IMPORTANT: All functions are prefixed with `fragile_` to avoid conflicts
//! with system pthread functions. The transpiler must generate calls to these
//! prefixed versions.

use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};

/// Thread ID counter for generating unique thread IDs.
static THREAD_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Opaque thread handle type.
/// This wraps a Rust JoinHandle and provides a C-compatible interface.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct fragile_pthread_t {
    /// Internal thread ID (for identification)
    pub id: u64,
    /// Pointer to heap-allocated JoinHandle (null after join)
    pub handle_ptr: *mut c_void,
}

impl Default for fragile_pthread_t {
    fn default() -> Self {
        Self::new()
    }
}

impl fragile_pthread_t {
    /// Create a new uninitialized fragile_pthread_t.
    pub fn new() -> Self {
        Self {
            id: 0,
            handle_ptr: std::ptr::null_mut(),
        }
    }
}

// fragile_pthread_t must be Send + Sync for thread safety
unsafe impl Send for fragile_pthread_t {}
unsafe impl Sync for fragile_pthread_t {}

/// Thread attributes (minimal implementation for API compatibility).
#[repr(C)]
pub struct fragile_pthread_attr_t {
    /// Detach state (0 = joinable, 1 = detached)
    detach_state: c_int,
}

/// Wrapper to make function pointer and arg Send-safe.
struct ThreadStartInfo {
    start_routine: extern "C" fn(*mut c_void) -> *mut c_void,
    arg: usize, // Store as usize to avoid Send issues with raw pointers
}

unsafe impl Send for ThreadStartInfo {}

/// Create a new thread.
///
/// # Arguments
/// * `thread` - Pointer to store the new thread handle
/// * `attr` - Thread attributes (can be null for defaults)
/// * `start_routine` - Function to run in the new thread
/// * `arg` - Argument to pass to start_routine
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_create(
    thread: *mut fragile_pthread_t,
    _attr: *const fragile_pthread_attr_t,
    start_routine: Option<extern "C" fn(*mut c_void) -> *mut c_void>,
    arg: *mut c_void,
) -> c_int {
    if thread.is_null() {
        return 22; // EINVAL
    }

    let start_fn = match start_routine {
        Some(f) => f,
        None => return 22, // EINVAL
    };

    let thread_id = THREAD_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

    // Wrap function and arg to make them Send-safe
    let info = ThreadStartInfo {
        start_routine: start_fn,
        arg: arg as usize,
    };

    // Spawn the thread
    let handle: JoinHandle<usize> = thread::spawn(move || {
        let result = (info.start_routine)(info.arg as *mut c_void);
        result as usize
    });

    // Store handle on heap so we can pass it through C API
    let handle_box = Box::new(handle);
    let handle_ptr = Box::into_raw(handle_box) as *mut c_void;

    unsafe {
        (*thread).id = thread_id;
        (*thread).handle_ptr = handle_ptr;
    }

    0 // Success
}

/// Wait for a thread to terminate.
///
/// # Arguments
/// * `thread` - Thread handle to wait for
/// * `retval` - Pointer to store the thread's return value (can be null)
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_join(thread: fragile_pthread_t, retval: *mut *mut c_void) -> c_int {
    if thread.handle_ptr.is_null() {
        return 22; // EINVAL - no valid handle
    }

    // Recover the JoinHandle from the raw pointer
    let handle_box: Box<JoinHandle<usize>> =
        unsafe { Box::from_raw(thread.handle_ptr as *mut JoinHandle<usize>) };

    match handle_box.join() {
        Ok(result) => {
            if !retval.is_null() {
                unsafe {
                    *retval = result as *mut c_void;
                }
            }
            0 // Success
        }
        Err(_) => 1, // Thread panicked
    }
}

/// Get the calling thread's ID.
///
/// # Returns
/// The thread ID of the calling thread.
#[no_mangle]
pub extern "C" fn fragile_pthread_self() -> fragile_pthread_t {
    // For the main thread or threads not created via fragile_pthread_create,
    // we return a handle with just an ID (no joinable handle)
    // Use the thread ID's debug representation to get a unique number
    let thread_id = thread::current().id();
    let id_str = format!("{:?}", thread_id);
    // Extract numeric part from "ThreadId(N)"
    let id_num = id_str
        .trim_start_matches("ThreadId(")
        .trim_end_matches(')')
        .parse::<u64>()
        .unwrap_or(0);

    fragile_pthread_t {
        id: id_num,
        handle_ptr: std::ptr::null_mut(),
    }
}

/// Compare two thread IDs for equality.
///
/// # Returns
/// Non-zero if equal, 0 if not equal.
#[no_mangle]
pub extern "C" fn fragile_pthread_equal(t1: fragile_pthread_t, t2: fragile_pthread_t) -> c_int {
    if t1.id == t2.id {
        1
    } else {
        0
    }
}

/// Initialize thread attributes with default values.
#[no_mangle]
pub extern "C" fn fragile_pthread_attr_init(attr: *mut fragile_pthread_attr_t) -> c_int {
    if attr.is_null() {
        return 22; // EINVAL
    }
    unsafe {
        (*attr).detach_state = 0; // Joinable by default
    }
    0
}

/// Destroy thread attributes.
#[no_mangle]
pub extern "C" fn fragile_pthread_attr_destroy(_attr: *mut fragile_pthread_attr_t) -> c_int {
    0 // Nothing to clean up
}

/// Set detach state in thread attributes.
#[no_mangle]
pub extern "C" fn fragile_pthread_attr_setdetachstate(attr: *mut fragile_pthread_attr_t, detachstate: c_int) -> c_int {
    if attr.is_null() {
        return 22; // EINVAL
    }
    unsafe {
        (*attr).detach_state = detachstate;
    }
    0
}

/// Get detach state from thread attributes.
#[no_mangle]
pub extern "C" fn fragile_pthread_attr_getdetachstate(attr: *const fragile_pthread_attr_t, detachstate: *mut c_int) -> c_int {
    if attr.is_null() || detachstate.is_null() {
        return 22; // EINVAL
    }
    unsafe {
        *detachstate = (*attr).detach_state;
    }
    0
}

/// Detach a thread (allow it to clean up automatically when it exits).
#[no_mangle]
pub extern "C" fn fragile_pthread_detach(thread: fragile_pthread_t) -> c_int {
    if thread.handle_ptr.is_null() {
        return 22; // EINVAL
    }

    // Recover the JoinHandle and immediately drop it (detaches the thread)
    let _handle_box: Box<JoinHandle<usize>> =
        unsafe { Box::from_raw(thread.handle_ptr as *mut JoinHandle<usize>) };

    // Handle is dropped here, which detaches the thread
    0
}

/// Exit the current thread.
#[no_mangle]
pub extern "C" fn fragile_pthread_exit(_retval: *mut c_void) -> ! {
    // Note: We can't easily return a value from Rust threads this way.
    // For now, just terminate the thread by panicking (which will be caught by join).
    panic!("fragile_pthread_exit called");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pthread_self() {
        let self_thread = fragile_pthread_self();
        assert!(self_thread.id > 0);
    }

    #[test]
    fn test_pthread_attr() {
        let mut attr = fragile_pthread_attr_t { detach_state: -1 };

        assert_eq!(fragile_pthread_attr_init(&mut attr), 0);
        assert_eq!(attr.detach_state, 0);

        assert_eq!(fragile_pthread_attr_setdetachstate(&mut attr, 1), 0);

        let mut state = 0;
        assert_eq!(fragile_pthread_attr_getdetachstate(&attr, &mut state), 0);
        assert_eq!(state, 1);

        assert_eq!(fragile_pthread_attr_destroy(&mut attr), 0);
    }

    extern "C" fn return_value_thread(arg: *mut c_void) -> *mut c_void {
        // Return the argument as the return value
        arg
    }

    #[test]
    fn test_pthread_create_join() {
        let mut thread = fragile_pthread_t::new();
        let expected_value = 42 as *mut c_void;

        let result = fragile_pthread_create(
            &mut thread,
            std::ptr::null(),
            Some(return_value_thread),
            expected_value,
        );
        assert_eq!(result, 0);

        let mut retval: *mut c_void = std::ptr::null_mut();
        let result = fragile_pthread_join(thread, &mut retval);
        assert_eq!(result, 0);
        assert_eq!(retval, expected_value);
    }
}
