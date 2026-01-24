//! POSIX mutex (pthread_mutex) wrappers for transpiled C++ code.
//!
//! This module provides pthread_mutex-compatible functions that transpiled C++ code
//! can call. When C++ code uses `std::mutex`, libc++ internally calls pthread_mutex
//! functions which are implemented here using Rust's std::sync primitives.
//!
//! IMPORTANT: All functions are prefixed with `fragile_` to avoid conflicts
//! with system pthread_mutex functions. The transpiler must generate calls to these
//! prefixed versions.

use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

/// Opaque mutex type.
/// This wraps a simple atomic spinlock and provides a C-compatible interface.
/// Note: This is a basic implementation. For production use, consider using
/// OS-level primitives or a proper parking lot.
#[repr(C)]
pub struct fragile_pthread_mutex_t {
    /// Pointer to heap-allocated MutexInner (null if uninitialized)
    pub mutex_ptr: *mut c_void,
    /// Initialization state marker
    pub initialized: u32,
}

impl Default for fragile_pthread_mutex_t {
    fn default() -> Self {
        Self::new()
    }
}

impl fragile_pthread_mutex_t {
    /// Create a new uninitialized fragile_pthread_mutex_t.
    pub fn new() -> Self {
        Self {
            mutex_ptr: std::ptr::null_mut(),
            initialized: 0,
        }
    }
}

// fragile_pthread_mutex_t must be Send + Sync for thread safety
unsafe impl Send for fragile_pthread_mutex_t {}
unsafe impl Sync for fragile_pthread_mutex_t {}

/// Mutex attributes (minimal implementation for API compatibility).
#[repr(C)]
pub struct fragile_pthread_mutexattr_t {
    /// Mutex type (normal, recursive, etc.)
    kind: c_int,
}

/// PTHREAD_MUTEX_NORMAL - normal mutex (default)
pub const FRAGILE_PTHREAD_MUTEX_NORMAL: c_int = 0;
/// PTHREAD_MUTEX_RECURSIVE - recursive mutex (allows same thread to lock multiple times)
pub const FRAGILE_PTHREAD_MUTEX_RECURSIVE: c_int = 1;
/// PTHREAD_MUTEX_ERRORCHECK - error-checking mutex
pub const FRAGILE_PTHREAD_MUTEX_ERRORCHECK: c_int = 2;

/// Internal mutex structure using atomic spinlock.
struct MutexInner {
    /// Locked state
    locked: AtomicBool,
    /// Mutex type
    _kind: c_int,
}

/// Initialize a mutex.
///
/// # Arguments
/// * `mutex` - Pointer to the mutex to initialize
/// * `attr` - Mutex attributes (can be null for defaults)
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_mutex_init(
    mutex: *mut fragile_pthread_mutex_t,
    _attr: *const fragile_pthread_mutexattr_t,
) -> c_int {
    if mutex.is_null() {
        return 22; // EINVAL
    }

    // Create a new mutex inner
    let inner = Box::new(MutexInner {
        locked: AtomicBool::new(false),
        _kind: FRAGILE_PTHREAD_MUTEX_NORMAL,
    });

    let mutex_ptr = Box::into_raw(inner) as *mut c_void;

    unsafe {
        (*mutex).mutex_ptr = mutex_ptr;
        (*mutex).initialized = 0xDEAD_BEEF; // Magic marker for initialized
    }

    0 // Success
}

/// Destroy a mutex.
///
/// # Arguments
/// * `mutex` - Pointer to the mutex to destroy
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_mutex_destroy(mutex: *mut fragile_pthread_mutex_t) -> c_int {
    if mutex.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*mutex).initialized != 0xDEAD_BEEF || (*mutex).mutex_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        // Recover and drop the inner
        let _inner: Box<MutexInner> = Box::from_raw((*mutex).mutex_ptr as *mut MutexInner);

        (*mutex).mutex_ptr = std::ptr::null_mut();
        (*mutex).initialized = 0;
    }

    0 // Success
}

/// Lock a mutex.
///
/// # Arguments
/// * `mutex` - Pointer to the mutex to lock
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_mutex_lock(mutex: *mut fragile_pthread_mutex_t) -> c_int {
    if mutex.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*mutex).initialized != 0xDEAD_BEEF || (*mutex).mutex_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*mutex).mutex_ptr as *const MutexInner);

        // Spinlock: keep trying until we acquire the lock
        while inner
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin hint for the CPU
            std::hint::spin_loop();
        }
    }

    0 // Success
}

/// Try to lock a mutex without blocking.
///
/// # Arguments
/// * `mutex` - Pointer to the mutex to try locking
///
/// # Returns
/// 0 on success, EBUSY if already locked, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_mutex_trylock(mutex: *mut fragile_pthread_mutex_t) -> c_int {
    if mutex.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*mutex).initialized != 0xDEAD_BEEF || (*mutex).mutex_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*mutex).mutex_ptr as *const MutexInner);

        // Try to acquire the lock without blocking
        match inner
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => 0,  // Success
            Err(_) => 16, // EBUSY - already locked
        }
    }
}

/// Unlock a mutex.
///
/// # Arguments
/// * `mutex` - Pointer to the mutex to unlock
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_mutex_unlock(mutex: *mut fragile_pthread_mutex_t) -> c_int {
    if mutex.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*mutex).initialized != 0xDEAD_BEEF || (*mutex).mutex_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*mutex).mutex_ptr as *const MutexInner);

        // Release the lock
        inner.locked.store(false, Ordering::Release);
    }

    0 // Success
}

/// Initialize mutex attributes with default values.
#[no_mangle]
pub extern "C" fn fragile_pthread_mutexattr_init(attr: *mut fragile_pthread_mutexattr_t) -> c_int {
    if attr.is_null() {
        return 22; // EINVAL
    }
    unsafe {
        (*attr).kind = FRAGILE_PTHREAD_MUTEX_NORMAL;
    }
    0
}

/// Destroy mutex attributes.
#[no_mangle]
pub extern "C" fn fragile_pthread_mutexattr_destroy(_attr: *mut fragile_pthread_mutexattr_t) -> c_int {
    0 // Nothing to clean up
}

/// Set mutex type in attributes.
#[no_mangle]
pub extern "C" fn fragile_pthread_mutexattr_settype(attr: *mut fragile_pthread_mutexattr_t, kind: c_int) -> c_int {
    if attr.is_null() {
        return 22; // EINVAL
    }
    if kind < 0 || kind > 2 {
        return 22; // EINVAL - invalid type
    }
    unsafe {
        (*attr).kind = kind;
    }
    0
}

/// Get mutex type from attributes.
#[no_mangle]
pub extern "C" fn fragile_pthread_mutexattr_gettype(attr: *const fragile_pthread_mutexattr_t, kind: *mut c_int) -> c_int {
    if attr.is_null() || kind.is_null() {
        return 22; // EINVAL
    }
    unsafe {
        *kind = (*attr).kind;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutex_init_destroy() {
        let mut mutex = fragile_pthread_mutex_t::new();

        assert_eq!(fragile_pthread_mutex_init(&mut mutex, std::ptr::null()), 0);
        assert!(mutex.initialized == 0xDEAD_BEEF);
        assert!(!mutex.mutex_ptr.is_null());

        assert_eq!(fragile_pthread_mutex_destroy(&mut mutex), 0);
        assert!(mutex.mutex_ptr.is_null());
        assert_eq!(mutex.initialized, 0);
    }

    #[test]
    fn test_mutex_lock_unlock() {
        let mut mutex = fragile_pthread_mutex_t::new();

        assert_eq!(fragile_pthread_mutex_init(&mut mutex, std::ptr::null()), 0);

        // Lock should succeed
        assert_eq!(fragile_pthread_mutex_lock(&mut mutex), 0);

        // Unlock should succeed
        assert_eq!(fragile_pthread_mutex_unlock(&mut mutex), 0);

        // Clean up
        assert_eq!(fragile_pthread_mutex_destroy(&mut mutex), 0);
    }

    #[test]
    fn test_mutex_trylock() {
        let mut mutex = fragile_pthread_mutex_t::new();

        assert_eq!(fragile_pthread_mutex_init(&mut mutex, std::ptr::null()), 0);

        // First trylock should succeed
        assert_eq!(fragile_pthread_mutex_trylock(&mut mutex), 0);

        // Second trylock should fail with EBUSY
        assert_eq!(fragile_pthread_mutex_trylock(&mut mutex), 16);

        // Unlock
        assert_eq!(fragile_pthread_mutex_unlock(&mut mutex), 0);

        // Now trylock should succeed again
        assert_eq!(fragile_pthread_mutex_trylock(&mut mutex), 0);
        assert_eq!(fragile_pthread_mutex_unlock(&mut mutex), 0);

        // Clean up
        assert_eq!(fragile_pthread_mutex_destroy(&mut mutex), 0);
    }

    #[test]
    fn test_mutexattr() {
        let mut attr = fragile_pthread_mutexattr_t { kind: -1 };

        assert_eq!(fragile_pthread_mutexattr_init(&mut attr), 0);
        assert_eq!(attr.kind, FRAGILE_PTHREAD_MUTEX_NORMAL);

        assert_eq!(fragile_pthread_mutexattr_settype(&mut attr, FRAGILE_PTHREAD_MUTEX_RECURSIVE), 0);

        let mut kind = 0;
        assert_eq!(fragile_pthread_mutexattr_gettype(&attr, &mut kind), 0);
        assert_eq!(kind, FRAGILE_PTHREAD_MUTEX_RECURSIVE);

        assert_eq!(fragile_pthread_mutexattr_destroy(&mut attr), 0);
    }

    #[test]
    fn test_mutex_multithread() {
        use std::sync::Arc;
        use std::thread;

        // Use Arc to share mutex across threads
        let mutex_box = Box::new(fragile_pthread_mutex_t::new());
        let mutex_ptr = Box::into_raw(mutex_box);

        assert_eq!(fragile_pthread_mutex_init(mutex_ptr, std::ptr::null()), 0);

        let counter = Arc::new(std::sync::atomic::AtomicI32::new(0));
        let mut handles = vec![];

        for _ in 0..4 {
            let counter_clone = counter.clone();
            let mutex_raw = mutex_ptr as usize; // Convert to usize for thread safety

            let handle = thread::spawn(move || {
                let mutex = mutex_raw as *mut fragile_pthread_mutex_t;
                for _ in 0..100 {
                    fragile_pthread_mutex_lock(mutex);
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                    fragile_pthread_mutex_unlock(mutex);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All increments should have happened
        assert_eq!(counter.load(Ordering::SeqCst), 400);

        // Clean up
        fragile_pthread_mutex_destroy(mutex_ptr);
        unsafe {
            let _ = Box::from_raw(mutex_ptr);
        }
    }
}
