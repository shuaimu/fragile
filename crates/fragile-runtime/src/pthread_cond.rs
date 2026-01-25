//! POSIX condition variable (pthread_cond) wrappers for transpiled C++ code.
//!
//! This module provides pthread_cond-compatible functions that transpiled C++ code
//! can call. When C++ code uses `std::condition_variable`, libc++ internally calls
//! pthread_cond functions which are implemented here using Rust's std::sync::Condvar.
//!
//! IMPORTANT: All functions are prefixed with `fragile_` to avoid conflicts
//! with system pthread_cond functions. The transpiler must generate calls to these
//! prefixed versions.

use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::pthread_mutex::fragile_pthread_mutex_t;

/// Opaque condition variable type.
/// This wraps a Rust Condvar and provides a C-compatible interface.
#[repr(C)]
pub struct fragile_pthread_cond_t {
    /// Pointer to heap-allocated CondvarInner (null if uninitialized)
    pub cond_ptr: *mut c_void,
    /// Initialization state marker
    pub initialized: u32,
}

impl Default for fragile_pthread_cond_t {
    fn default() -> Self {
        Self::new()
    }
}

impl fragile_pthread_cond_t {
    /// Create a new uninitialized fragile_pthread_cond_t.
    pub fn new() -> Self {
        Self {
            cond_ptr: std::ptr::null_mut(),
            initialized: 0,
        }
    }
}

// fragile_pthread_cond_t must be Send + Sync for thread safety
unsafe impl Send for fragile_pthread_cond_t {}
unsafe impl Sync for fragile_pthread_cond_t {}

/// Condition variable attributes (minimal implementation for API compatibility).
#[repr(C)]
pub struct fragile_pthread_condattr_t {
    /// Reserved for future use
    _reserved: c_int,
}

/// Internal condition variable structure.
/// Note: We use a separate Mutex internally because Rust's Condvar requires it,
/// but the actual locking is done by the pthread_mutex passed to wait functions.
struct CondvarInner {
    condvar: Condvar,
    /// Internal mutex for the condvar (used to satisfy Rust's API)
    mutex: Mutex<()>,
}

/// Initialize a condition variable.
///
/// # Arguments
/// * `cond` - Pointer to the condition variable to initialize
/// * `attr` - Condition variable attributes (can be null for defaults)
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_cond_init(
    cond: *mut fragile_pthread_cond_t,
    _attr: *const fragile_pthread_condattr_t,
) -> c_int {
    if cond.is_null() {
        return 22; // EINVAL
    }

    // Create a new condvar inner
    let inner = Box::new(CondvarInner {
        condvar: Condvar::new(),
        mutex: Mutex::new(()),
    });

    let cond_ptr = Box::into_raw(inner) as *mut c_void;

    unsafe {
        (*cond).cond_ptr = cond_ptr;
        (*cond).initialized = 0xC0DEBABE; // Magic marker for initialized
    }

    0 // Success
}

/// Destroy a condition variable.
///
/// # Arguments
/// * `cond` - Pointer to the condition variable to destroy
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_cond_destroy(cond: *mut fragile_pthread_cond_t) -> c_int {
    if cond.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*cond).initialized != 0xC0DEBABE || (*cond).cond_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        // Recover and drop the inner
        let _inner: Box<CondvarInner> = Box::from_raw((*cond).cond_ptr as *mut CondvarInner);

        (*cond).cond_ptr = std::ptr::null_mut();
        (*cond).initialized = 0;
    }

    0 // Success
}

/// Wait on a condition variable.
///
/// This atomically releases the mutex, waits on the condition variable,
/// and re-acquires the mutex before returning.
///
/// # Arguments
/// * `cond` - Pointer to the condition variable
/// * `mutex` - Pointer to the mutex to release/reacquire
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_cond_wait(
    cond: *mut fragile_pthread_cond_t,
    mutex: *mut fragile_pthread_mutex_t,
) -> c_int {
    if cond.is_null() || mutex.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*cond).initialized != 0xC0DEBABE || (*cond).cond_ptr.is_null() {
            return 22; // EINVAL - condvar not initialized
        }

        if (*mutex).initialized != 0xDEAD_BEEF || (*mutex).mutex_ptr.is_null() {
            return 22; // EINVAL - mutex not initialized
        }

        let inner = &*((*cond).cond_ptr as *const CondvarInner);

        // We need to unlock the external mutex, wait on the condvar, then relock.
        // Since we're using a spinlock-based mutex, we just unlock/relock around the wait.

        // Get access to the mutex's atomic bool
        use crate::pthread_mutex::fragile_pthread_mutex_lock;
        use crate::pthread_mutex::fragile_pthread_mutex_unlock;

        // Unlock the external mutex
        fragile_pthread_mutex_unlock(mutex);

        // Wait using our internal mutex
        {
            let guard = inner.mutex.lock().unwrap();
            let _guard = inner.condvar.wait(guard).unwrap();
        }

        // Relock the external mutex
        fragile_pthread_mutex_lock(mutex);
    }

    0 // Success
}

/// Wait on a condition variable with a timeout.
///
/// # Arguments
/// * `cond` - Pointer to the condition variable
/// * `mutex` - Pointer to the mutex to release/reacquire
/// * `abstime` - Absolute timeout (seconds since epoch and nanoseconds)
///
/// # Returns
/// 0 on success, ETIMEDOUT on timeout, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_cond_timedwait(
    cond: *mut fragile_pthread_cond_t,
    mutex: *mut fragile_pthread_mutex_t,
    abstime: *const libc_timespec,
) -> c_int {
    if cond.is_null() || mutex.is_null() || abstime.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*cond).initialized != 0xC0DEBABE || (*cond).cond_ptr.is_null() {
            return 22; // EINVAL - condvar not initialized
        }

        if (*mutex).initialized != 0xDEAD_BEEF || (*mutex).mutex_ptr.is_null() {
            return 22; // EINVAL - mutex not initialized
        }

        let inner = &*((*cond).cond_ptr as *const CondvarInner);

        // Calculate timeout duration from absolute time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);

        let target = Duration::new((*abstime).tv_sec as u64, (*abstime).tv_nsec as u32);
        let timeout = target.saturating_sub(now);

        use crate::pthread_mutex::fragile_pthread_mutex_lock;
        use crate::pthread_mutex::fragile_pthread_mutex_unlock;

        // Unlock the external mutex
        fragile_pthread_mutex_unlock(mutex);

        // Wait with timeout using our internal mutex
        let result = {
            let guard = inner.mutex.lock().unwrap();
            inner.condvar.wait_timeout(guard, timeout)
        };

        // Relock the external mutex
        fragile_pthread_mutex_lock(mutex);

        match result {
            Ok((_, timeout_result)) if timeout_result.timed_out() => 110, // ETIMEDOUT
            Ok(_) => 0,                                                   // Success
            Err(_) => 1,                                                  // Error (mutex poisoned)
        }
    }
}

/// C-compatible timespec structure.
#[repr(C)]
pub struct libc_timespec {
    pub tv_sec: i64,  // seconds
    pub tv_nsec: i64, // nanoseconds
}

/// Signal one waiting thread on a condition variable.
///
/// # Arguments
/// * `cond` - Pointer to the condition variable
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_cond_signal(cond: *mut fragile_pthread_cond_t) -> c_int {
    if cond.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*cond).initialized != 0xC0DEBABE || (*cond).cond_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*cond).cond_ptr as *const CondvarInner);
        inner.condvar.notify_one();
    }

    0 // Success
}

/// Broadcast to all waiting threads on a condition variable.
///
/// # Arguments
/// * `cond` - Pointer to the condition variable
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_cond_broadcast(cond: *mut fragile_pthread_cond_t) -> c_int {
    if cond.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*cond).initialized != 0xC0DEBABE || (*cond).cond_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*cond).cond_ptr as *const CondvarInner);
        inner.condvar.notify_all();
    }

    0 // Success
}

/// Initialize condition variable attributes with default values.
#[no_mangle]
pub extern "C" fn fragile_pthread_condattr_init(attr: *mut fragile_pthread_condattr_t) -> c_int {
    if attr.is_null() {
        return 22; // EINVAL
    }
    unsafe {
        (*attr)._reserved = 0;
    }
    0
}

/// Destroy condition variable attributes.
#[no_mangle]
pub extern "C" fn fragile_pthread_condattr_destroy(
    _attr: *mut fragile_pthread_condattr_t,
) -> c_int {
    0 // Nothing to clean up
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pthread_mutex::{
        fragile_pthread_mutex_destroy, fragile_pthread_mutex_init, fragile_pthread_mutex_lock,
        fragile_pthread_mutex_t, fragile_pthread_mutex_unlock,
    };

    #[test]
    fn test_cond_init_destroy() {
        let mut cond = fragile_pthread_cond_t::new();

        assert_eq!(fragile_pthread_cond_init(&mut cond, std::ptr::null()), 0);
        assert!(cond.initialized == 0xC0DEBABE);
        assert!(!cond.cond_ptr.is_null());

        assert_eq!(fragile_pthread_cond_destroy(&mut cond), 0);
        assert!(cond.cond_ptr.is_null());
        assert_eq!(cond.initialized, 0);
    }

    #[test]
    fn test_cond_signal_no_waiters() {
        let mut cond = fragile_pthread_cond_t::new();

        assert_eq!(fragile_pthread_cond_init(&mut cond, std::ptr::null()), 0);

        // Signal should succeed even with no waiters
        assert_eq!(fragile_pthread_cond_signal(&mut cond), 0);
        assert_eq!(fragile_pthread_cond_broadcast(&mut cond), 0);

        assert_eq!(fragile_pthread_cond_destroy(&mut cond), 0);
    }

    #[test]
    fn test_condattr() {
        let mut attr = fragile_pthread_condattr_t { _reserved: -1 };

        assert_eq!(fragile_pthread_condattr_init(&mut attr), 0);
        assert_eq!(attr._reserved, 0);

        assert_eq!(fragile_pthread_condattr_destroy(&mut attr), 0);
    }

    #[test]
    fn test_cond_signal_wait() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;

        // Allocate on heap and share via raw pointers
        let mutex_box = Box::new(fragile_pthread_mutex_t::new());
        let cond_box = Box::new(fragile_pthread_cond_t::new());
        let mutex_ptr = Box::into_raw(mutex_box);
        let cond_ptr = Box::into_raw(cond_box);

        fragile_pthread_mutex_init(mutex_ptr, std::ptr::null());
        fragile_pthread_cond_init(cond_ptr, std::ptr::null());

        let ready = Arc::new(AtomicBool::new(false));
        let ready_clone = ready.clone();

        let mutex_usize = mutex_ptr as usize;
        let cond_usize = cond_ptr as usize;

        // Spawn waiter thread
        let handle = thread::spawn(move || {
            let mutex = mutex_usize as *mut fragile_pthread_mutex_t;
            let cond = cond_usize as *mut fragile_pthread_cond_t;

            fragile_pthread_mutex_lock(mutex);
            ready_clone.store(true, Ordering::SeqCst);
            fragile_pthread_cond_wait(cond, mutex);
            fragile_pthread_mutex_unlock(mutex);
        });

        // Wait for waiter to be ready
        while !ready.load(Ordering::SeqCst) {
            thread::yield_now();
        }

        // Give waiter time to enter wait
        thread::sleep(std::time::Duration::from_millis(10));

        // Signal the waiter
        fragile_pthread_mutex_lock(mutex_ptr);
        fragile_pthread_cond_signal(cond_ptr);
        fragile_pthread_mutex_unlock(mutex_ptr);

        // Wait for thread to complete
        handle.join().unwrap();

        // Clean up
        fragile_pthread_cond_destroy(cond_ptr);
        fragile_pthread_mutex_destroy(mutex_ptr);
        unsafe {
            let _ = Box::from_raw(mutex_ptr);
            let _ = Box::from_raw(cond_ptr);
        }
    }
}
