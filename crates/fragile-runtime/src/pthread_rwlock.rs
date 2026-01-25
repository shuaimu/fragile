//! POSIX read-write lock (pthread_rwlock) wrappers for transpiled C++ code.
//!
//! This module provides pthread_rwlock-compatible functions that transpiled C++ code
//! can call. When C++ code uses `std::shared_mutex`, libc++ internally calls
//! pthread_rwlock functions which are implemented here using atomic operations.
//!
//! IMPORTANT: All functions are prefixed with `fragile_` to avoid conflicts
//! with system pthread_rwlock functions. The transpiler must generate calls to these
//! prefixed versions.

use std::ffi::c_void;
use std::os::raw::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

/// Opaque read-write lock type.
/// This uses an atomic counter for lock state:
/// - 0: unlocked
/// - positive: number of readers holding the lock
/// - -1: writer holding the lock
#[repr(C)]
pub struct fragile_pthread_rwlock_t {
    /// Pointer to heap-allocated RwLockInner (null if uninitialized)
    pub rwlock_ptr: *mut c_void,
    /// Initialization state marker (0x52574C4B = "RWLK")
    pub initialized: u32,
}

/// Magic marker for initialized rwlock
const RWLK_INIT_MARKER: u32 = 0x52574C4B; // "RWLK" in hex

impl Default for fragile_pthread_rwlock_t {
    fn default() -> Self {
        Self::new()
    }
}

impl fragile_pthread_rwlock_t {
    /// Create a new uninitialized fragile_pthread_rwlock_t.
    pub fn new() -> Self {
        Self {
            rwlock_ptr: std::ptr::null_mut(),
            initialized: 0,
        }
    }
}

// fragile_pthread_rwlock_t must be Send + Sync for thread safety
unsafe impl Send for fragile_pthread_rwlock_t {}
unsafe impl Sync for fragile_pthread_rwlock_t {}

/// Read-write lock attributes (minimal implementation for API compatibility).
#[repr(C)]
pub struct fragile_pthread_rwlockattr_t {
    /// Reserved for future use
    _reserved: c_int,
}

/// Internal read-write lock structure using atomic counter.
/// State encoding:
/// - 0: unlocked
/// - positive N: N readers holding the lock
/// - -1: writer holding the lock
struct RwLockInner {
    state: AtomicI32,
}

/// Initialize a read-write lock.
///
/// # Arguments
/// * `rwlock` - Pointer to the read-write lock to initialize
/// * `attr` - Read-write lock attributes (can be null for defaults)
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_rwlock_init(
    rwlock: *mut fragile_pthread_rwlock_t,
    _attr: *const fragile_pthread_rwlockattr_t,
) -> c_int {
    if rwlock.is_null() {
        return 22; // EINVAL
    }

    // Create a new rwlock inner
    let inner = Box::new(RwLockInner {
        state: AtomicI32::new(0),
    });

    let rwlock_ptr = Box::into_raw(inner) as *mut c_void;

    unsafe {
        (*rwlock).rwlock_ptr = rwlock_ptr;
        (*rwlock).initialized = RWLK_INIT_MARKER;
    }

    0 // Success
}

/// Destroy a read-write lock.
///
/// # Arguments
/// * `rwlock` - Pointer to the read-write lock to destroy
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_rwlock_destroy(rwlock: *mut fragile_pthread_rwlock_t) -> c_int {
    if rwlock.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*rwlock).initialized != RWLK_INIT_MARKER || (*rwlock).rwlock_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        // Recover and drop the inner
        let _inner: Box<RwLockInner> = Box::from_raw((*rwlock).rwlock_ptr as *mut RwLockInner);

        (*rwlock).rwlock_ptr = std::ptr::null_mut();
        (*rwlock).initialized = 0;
    }

    0 // Success
}

/// Acquire a read lock.
///
/// Multiple readers can hold the lock simultaneously.
/// Blocks if a writer holds the lock.
///
/// # Arguments
/// * `rwlock` - Pointer to the read-write lock
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_rwlock_rdlock(rwlock: *mut fragile_pthread_rwlock_t) -> c_int {
    if rwlock.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*rwlock).initialized != RWLK_INIT_MARKER || (*rwlock).rwlock_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*rwlock).rwlock_ptr as *const RwLockInner);

        loop {
            let state = inner.state.load(Ordering::Acquire);

            // If writer holds the lock (state == -1), spin and retry
            if state < 0 {
                std::hint::spin_loop();
                continue;
            }

            // Try to increment reader count
            if inner
                .state
                .compare_exchange_weak(state, state + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return 0; // Success
            }
            // CAS failed, retry
            std::hint::spin_loop();
        }
    }
}

/// Try to acquire a read lock without blocking.
///
/// # Arguments
/// * `rwlock` - Pointer to the read-write lock
///
/// # Returns
/// 0 on success, EBUSY if would block, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_rwlock_tryrdlock(rwlock: *mut fragile_pthread_rwlock_t) -> c_int {
    if rwlock.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*rwlock).initialized != RWLK_INIT_MARKER || (*rwlock).rwlock_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*rwlock).rwlock_ptr as *const RwLockInner);

        let state = inner.state.load(Ordering::Acquire);

        // If writer holds the lock, fail immediately
        if state < 0 {
            return 16; // EBUSY
        }

        // Try to increment reader count
        match inner
            .state
            .compare_exchange(state, state + 1, Ordering::AcqRel, Ordering::Relaxed)
        {
            Ok(_) => 0,   // Success
            Err(_) => 16, // EBUSY - state changed
        }
    }
}

/// Acquire a write lock.
///
/// Only one writer can hold the lock at a time.
/// Blocks if any readers or another writer holds the lock.
///
/// # Arguments
/// * `rwlock` - Pointer to the read-write lock
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_rwlock_wrlock(rwlock: *mut fragile_pthread_rwlock_t) -> c_int {
    if rwlock.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*rwlock).initialized != RWLK_INIT_MARKER || (*rwlock).rwlock_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*rwlock).rwlock_ptr as *const RwLockInner);

        loop {
            // Try to change state from 0 (unlocked) to -1 (writer locked)
            if inner
                .state
                .compare_exchange_weak(0, -1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return 0; // Success
            }
            // Lock is held by readers or another writer, spin and retry
            std::hint::spin_loop();
        }
    }
}

/// Try to acquire a write lock without blocking.
///
/// # Arguments
/// * `rwlock` - Pointer to the read-write lock
///
/// # Returns
/// 0 on success, EBUSY if would block, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_rwlock_trywrlock(rwlock: *mut fragile_pthread_rwlock_t) -> c_int {
    if rwlock.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*rwlock).initialized != RWLK_INIT_MARKER || (*rwlock).rwlock_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*rwlock).rwlock_ptr as *const RwLockInner);

        // Try to change state from 0 (unlocked) to -1 (writer locked)
        match inner
            .state
            .compare_exchange(0, -1, Ordering::AcqRel, Ordering::Relaxed)
        {
            Ok(_) => 0,   // Success
            Err(_) => 16, // EBUSY - lock is held
        }
    }
}

/// Release a read or write lock.
///
/// Note: Unlike some implementations, this single function handles both
/// read and write unlock. For write locks, it sets state from -1 to 0.
/// For read locks, it decrements the reader count.
///
/// # Arguments
/// * `rwlock` - Pointer to the read-write lock
///
/// # Returns
/// 0 on success, error code on failure.
#[no_mangle]
pub extern "C" fn fragile_pthread_rwlock_unlock(rwlock: *mut fragile_pthread_rwlock_t) -> c_int {
    if rwlock.is_null() {
        return 22; // EINVAL
    }

    unsafe {
        if (*rwlock).initialized != RWLK_INIT_MARKER || (*rwlock).rwlock_ptr.is_null() {
            return 22; // EINVAL - not initialized
        }

        let inner = &*((*rwlock).rwlock_ptr as *const RwLockInner);

        loop {
            let state = inner.state.load(Ordering::Acquire);

            if state == -1 {
                // Writer lock - release by setting to 0
                if inner
                    .state
                    .compare_exchange_weak(-1, 0, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    return 0; // Success
                }
            } else if state > 0 {
                // Reader lock - decrement count
                if inner
                    .state
                    .compare_exchange_weak(state, state - 1, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    return 0; // Success
                }
            } else {
                // state == 0, not locked - error
                return 1; // Error - not locked
            }
            std::hint::spin_loop();
        }
    }
}

/// Initialize read-write lock attributes with default values.
#[no_mangle]
pub extern "C" fn fragile_pthread_rwlockattr_init(
    attr: *mut fragile_pthread_rwlockattr_t,
) -> c_int {
    if attr.is_null() {
        return 22; // EINVAL
    }
    unsafe {
        (*attr)._reserved = 0;
    }
    0
}

/// Destroy read-write lock attributes.
#[no_mangle]
pub extern "C" fn fragile_pthread_rwlockattr_destroy(
    _attr: *mut fragile_pthread_rwlockattr_t,
) -> c_int {
    0 // Nothing to clean up
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rwlock_init_destroy() {
        let mut rwlock = fragile_pthread_rwlock_t::new();

        assert_eq!(
            fragile_pthread_rwlock_init(&mut rwlock, std::ptr::null()),
            0
        );
        assert!(rwlock.initialized == RWLK_INIT_MARKER);
        assert!(!rwlock.rwlock_ptr.is_null());

        assert_eq!(fragile_pthread_rwlock_destroy(&mut rwlock), 0);
        assert!(rwlock.rwlock_ptr.is_null());
        assert_eq!(rwlock.initialized, 0);
    }

    #[test]
    fn test_rwlock_rdlock_unlock() {
        let mut rwlock = fragile_pthread_rwlock_t::new();
        assert_eq!(
            fragile_pthread_rwlock_init(&mut rwlock, std::ptr::null()),
            0
        );

        // Read lock should succeed
        assert_eq!(fragile_pthread_rwlock_rdlock(&mut rwlock), 0);

        // Unlock
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);

        assert_eq!(fragile_pthread_rwlock_destroy(&mut rwlock), 0);
    }

    #[test]
    fn test_rwlock_wrlock_unlock() {
        let mut rwlock = fragile_pthread_rwlock_t::new();
        assert_eq!(
            fragile_pthread_rwlock_init(&mut rwlock, std::ptr::null()),
            0
        );

        // Write lock should succeed
        assert_eq!(fragile_pthread_rwlock_wrlock(&mut rwlock), 0);

        // Unlock
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);

        assert_eq!(fragile_pthread_rwlock_destroy(&mut rwlock), 0);
    }

    #[test]
    fn test_rwlock_multiple_readers() {
        let mut rwlock = fragile_pthread_rwlock_t::new();
        assert_eq!(
            fragile_pthread_rwlock_init(&mut rwlock, std::ptr::null()),
            0
        );

        // Multiple read locks should succeed
        assert_eq!(fragile_pthread_rwlock_rdlock(&mut rwlock), 0);
        assert_eq!(fragile_pthread_rwlock_rdlock(&mut rwlock), 0);
        assert_eq!(fragile_pthread_rwlock_rdlock(&mut rwlock), 0);

        // Unlock all
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);

        assert_eq!(fragile_pthread_rwlock_destroy(&mut rwlock), 0);
    }

    #[test]
    fn test_rwlock_tryrdlock() {
        let mut rwlock = fragile_pthread_rwlock_t::new();
        assert_eq!(
            fragile_pthread_rwlock_init(&mut rwlock, std::ptr::null()),
            0
        );

        // Try read lock should succeed
        assert_eq!(fragile_pthread_rwlock_tryrdlock(&mut rwlock), 0);

        // Another try read lock should also succeed (readers can coexist)
        assert_eq!(fragile_pthread_rwlock_tryrdlock(&mut rwlock), 0);

        // Unlock both
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);

        assert_eq!(fragile_pthread_rwlock_destroy(&mut rwlock), 0);
    }

    #[test]
    fn test_rwlock_trywrlock() {
        let mut rwlock = fragile_pthread_rwlock_t::new();
        assert_eq!(
            fragile_pthread_rwlock_init(&mut rwlock, std::ptr::null()),
            0
        );

        // Try write lock should succeed
        assert_eq!(fragile_pthread_rwlock_trywrlock(&mut rwlock), 0);

        // Another try write lock should fail (writer is exclusive)
        assert_eq!(fragile_pthread_rwlock_trywrlock(&mut rwlock), 16); // EBUSY

        // Try read lock should also fail
        assert_eq!(fragile_pthread_rwlock_tryrdlock(&mut rwlock), 16); // EBUSY

        // Unlock
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);

        assert_eq!(fragile_pthread_rwlock_destroy(&mut rwlock), 0);
    }

    #[test]
    fn test_rwlock_writer_excludes_readers() {
        let mut rwlock = fragile_pthread_rwlock_t::new();
        assert_eq!(
            fragile_pthread_rwlock_init(&mut rwlock, std::ptr::null()),
            0
        );

        // Acquire read lock
        assert_eq!(fragile_pthread_rwlock_rdlock(&mut rwlock), 0);

        // Try write lock should fail (reader present)
        assert_eq!(fragile_pthread_rwlock_trywrlock(&mut rwlock), 16); // EBUSY

        // Unlock reader
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);

        // Now write lock should succeed
        assert_eq!(fragile_pthread_rwlock_trywrlock(&mut rwlock), 0);

        // Unlock writer
        assert_eq!(fragile_pthread_rwlock_unlock(&mut rwlock), 0);

        assert_eq!(fragile_pthread_rwlock_destroy(&mut rwlock), 0);
    }

    #[test]
    fn test_rwlockattr() {
        let mut attr = fragile_pthread_rwlockattr_t { _reserved: -1 };

        assert_eq!(fragile_pthread_rwlockattr_init(&mut attr), 0);
        assert_eq!(attr._reserved, 0);

        assert_eq!(fragile_pthread_rwlockattr_destroy(&mut attr), 0);
    }
}
