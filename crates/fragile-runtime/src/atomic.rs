//! Atomic operations for transpiled C++ code.
//!
//! This module provides atomic operation functions that transpiled C++ code
//! can call. When C++ code uses `std::atomic`, libc++ internally calls these
//! atomic primitives which are implemented here using Rust's std::sync::atomic.
//!
//! IMPORTANT: All functions are prefixed with `fragile_` to avoid conflicts
//! with system atomic functions. The transpiler must generate calls to these
//! prefixed versions.

use std::sync::atomic::Ordering;

/// Convert C++11 memory order to Rust Ordering.
/// C++ memory_order values:
/// - 0: memory_order_relaxed
/// - 1: memory_order_consume (treated as acquire in C++)
/// - 2: memory_order_acquire
/// - 3: memory_order_release
/// - 4: memory_order_acq_rel
/// - 5: memory_order_seq_cst
fn to_ordering(memory_order: i32) -> Ordering {
    match memory_order {
        0 => Ordering::Relaxed,
        1 => Ordering::Acquire, // consume is treated as acquire
        2 => Ordering::Acquire,
        3 => Ordering::Release,
        4 => Ordering::AcqRel,
        _ => Ordering::SeqCst, // 5 and unknown values default to strongest ordering
    }
}

/// Convert C++11 memory order for failed CAS operations.
fn to_failure_ordering(memory_order: i32) -> Ordering {
    match memory_order {
        0 => Ordering::Relaxed,
        1 | 2 => Ordering::Acquire,
        3 => Ordering::Relaxed, // Release becomes Relaxed for failure
        4 => Ordering::Acquire, // AcqRel becomes Acquire for failure
        _ => Ordering::SeqCst,  // 5 and unknown values default to SeqCst
    }
}

// ============================================================================
// 8-bit atomic operations
// ============================================================================

/// Atomic load for 8-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_load_8(ptr: *const u8, order: i32) -> u8 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        atomic.load(to_ordering(order))
    }
}

/// Atomic store for 8-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_store_8(ptr: *mut u8, value: u8, order: i32) {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        atomic.store(value, to_ordering(order));
    }
}

/// Atomic exchange for 8-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_exchange_8(ptr: *mut u8, value: u8, order: i32) -> u8 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        atomic.swap(value, to_ordering(order))
    }
}

/// Atomic compare-exchange (strong) for 8-bit values.
/// Returns 1 if successful, 0 otherwise. Updates expected on failure.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_strong_8(
    ptr: *mut u8,
    expected: *mut u8,
    desired: u8,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        let exp = *expected;
        match atomic.compare_exchange(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

/// Atomic compare-exchange (weak) for 8-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_weak_8(
    ptr: *mut u8,
    expected: *mut u8,
    desired: u8,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        let exp = *expected;
        match atomic.compare_exchange_weak(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

/// Atomic fetch-add for 8-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_add_8(ptr: *mut u8, value: u8, order: i32) -> u8 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        atomic.fetch_add(value, to_ordering(order))
    }
}

/// Atomic fetch-sub for 8-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_sub_8(ptr: *mut u8, value: u8, order: i32) -> u8 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        atomic.fetch_sub(value, to_ordering(order))
    }
}

/// Atomic fetch-and for 8-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_and_8(ptr: *mut u8, value: u8, order: i32) -> u8 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        atomic.fetch_and(value, to_ordering(order))
    }
}

/// Atomic fetch-or for 8-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_or_8(ptr: *mut u8, value: u8, order: i32) -> u8 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        atomic.fetch_or(value, to_ordering(order))
    }
}

/// Atomic fetch-xor for 8-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_xor_8(ptr: *mut u8, value: u8, order: i32) -> u8 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU8);
        atomic.fetch_xor(value, to_ordering(order))
    }
}

// ============================================================================
// 16-bit atomic operations
// ============================================================================

/// Atomic load for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_load_16(ptr: *const u16, order: i32) -> u16 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        atomic.load(to_ordering(order))
    }
}

/// Atomic store for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_store_16(ptr: *mut u16, value: u16, order: i32) {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        atomic.store(value, to_ordering(order));
    }
}

/// Atomic exchange for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_exchange_16(ptr: *mut u16, value: u16, order: i32) -> u16 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        atomic.swap(value, to_ordering(order))
    }
}

/// Atomic compare-exchange (strong) for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_strong_16(
    ptr: *mut u16,
    expected: *mut u16,
    desired: u16,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        let exp = *expected;
        match atomic.compare_exchange(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

/// Atomic compare-exchange (weak) for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_weak_16(
    ptr: *mut u16,
    expected: *mut u16,
    desired: u16,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        let exp = *expected;
        match atomic.compare_exchange_weak(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

/// Atomic fetch-add for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_add_16(ptr: *mut u16, value: u16, order: i32) -> u16 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        atomic.fetch_add(value, to_ordering(order))
    }
}

/// Atomic fetch-sub for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_sub_16(ptr: *mut u16, value: u16, order: i32) -> u16 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        atomic.fetch_sub(value, to_ordering(order))
    }
}

/// Atomic fetch-and for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_and_16(ptr: *mut u16, value: u16, order: i32) -> u16 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        atomic.fetch_and(value, to_ordering(order))
    }
}

/// Atomic fetch-or for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_or_16(ptr: *mut u16, value: u16, order: i32) -> u16 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        atomic.fetch_or(value, to_ordering(order))
    }
}

/// Atomic fetch-xor for 16-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_xor_16(ptr: *mut u16, value: u16, order: i32) -> u16 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU16);
        atomic.fetch_xor(value, to_ordering(order))
    }
}

// ============================================================================
// 32-bit atomic operations
// ============================================================================

/// Atomic load for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_load_32(ptr: *const u32, order: i32) -> u32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        atomic.load(to_ordering(order))
    }
}

/// Atomic store for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_store_32(ptr: *mut u32, value: u32, order: i32) {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        atomic.store(value, to_ordering(order));
    }
}

/// Atomic exchange for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_exchange_32(ptr: *mut u32, value: u32, order: i32) -> u32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        atomic.swap(value, to_ordering(order))
    }
}

/// Atomic compare-exchange (strong) for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_strong_32(
    ptr: *mut u32,
    expected: *mut u32,
    desired: u32,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        let exp = *expected;
        match atomic.compare_exchange(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

/// Atomic compare-exchange (weak) for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_weak_32(
    ptr: *mut u32,
    expected: *mut u32,
    desired: u32,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        let exp = *expected;
        match atomic.compare_exchange_weak(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

/// Atomic fetch-add for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_add_32(ptr: *mut u32, value: u32, order: i32) -> u32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        atomic.fetch_add(value, to_ordering(order))
    }
}

/// Atomic fetch-sub for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_sub_32(ptr: *mut u32, value: u32, order: i32) -> u32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        atomic.fetch_sub(value, to_ordering(order))
    }
}

/// Atomic fetch-and for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_and_32(ptr: *mut u32, value: u32, order: i32) -> u32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        atomic.fetch_and(value, to_ordering(order))
    }
}

/// Atomic fetch-or for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_or_32(ptr: *mut u32, value: u32, order: i32) -> u32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        atomic.fetch_or(value, to_ordering(order))
    }
}

/// Atomic fetch-xor for 32-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_xor_32(ptr: *mut u32, value: u32, order: i32) -> u32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU32);
        atomic.fetch_xor(value, to_ordering(order))
    }
}

// ============================================================================
// 64-bit atomic operations
// ============================================================================

/// Atomic load for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_load_64(ptr: *const u64, order: i32) -> u64 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        atomic.load(to_ordering(order))
    }
}

/// Atomic store for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_store_64(ptr: *mut u64, value: u64, order: i32) {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        atomic.store(value, to_ordering(order));
    }
}

/// Atomic exchange for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_exchange_64(ptr: *mut u64, value: u64, order: i32) -> u64 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        atomic.swap(value, to_ordering(order))
    }
}

/// Atomic compare-exchange (strong) for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_strong_64(
    ptr: *mut u64,
    expected: *mut u64,
    desired: u64,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        let exp = *expected;
        match atomic.compare_exchange(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

/// Atomic compare-exchange (weak) for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_weak_64(
    ptr: *mut u64,
    expected: *mut u64,
    desired: u64,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        let exp = *expected;
        match atomic.compare_exchange_weak(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

/// Atomic fetch-add for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_add_64(ptr: *mut u64, value: u64, order: i32) -> u64 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        atomic.fetch_add(value, to_ordering(order))
    }
}

/// Atomic fetch-sub for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_sub_64(ptr: *mut u64, value: u64, order: i32) -> u64 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        atomic.fetch_sub(value, to_ordering(order))
    }
}

/// Atomic fetch-and for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_and_64(ptr: *mut u64, value: u64, order: i32) -> u64 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        atomic.fetch_and(value, to_ordering(order))
    }
}

/// Atomic fetch-or for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_or_64(ptr: *mut u64, value: u64, order: i32) -> u64 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        atomic.fetch_or(value, to_ordering(order))
    }
}

/// Atomic fetch-xor for 64-bit values.
#[no_mangle]
pub extern "C" fn fragile_atomic_fetch_xor_64(ptr: *mut u64, value: u64, order: i32) -> u64 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicU64);
        atomic.fetch_xor(value, to_ordering(order))
    }
}

// ============================================================================
// Pointer atomic operations (for atomic<T*>)
// ============================================================================

/// Atomic load for pointer values.
#[no_mangle]
pub extern "C" fn fragile_atomic_load_ptr(ptr: *const *mut std::ffi::c_void, order: i32) -> *mut std::ffi::c_void {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicPtr<std::ffi::c_void>);
        atomic.load(to_ordering(order))
    }
}

/// Atomic store for pointer values.
#[no_mangle]
pub extern "C" fn fragile_atomic_store_ptr(ptr: *mut *mut std::ffi::c_void, value: *mut std::ffi::c_void, order: i32) {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicPtr<std::ffi::c_void>);
        atomic.store(value, to_ordering(order));
    }
}

/// Atomic exchange for pointer values.
#[no_mangle]
pub extern "C" fn fragile_atomic_exchange_ptr(ptr: *mut *mut std::ffi::c_void, value: *mut std::ffi::c_void, order: i32) -> *mut std::ffi::c_void {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicPtr<std::ffi::c_void>);
        atomic.swap(value, to_ordering(order))
    }
}

/// Atomic compare-exchange (strong) for pointer values.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_strong_ptr(
    ptr: *mut *mut std::ffi::c_void,
    expected: *mut *mut std::ffi::c_void,
    desired: *mut std::ffi::c_void,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicPtr<std::ffi::c_void>);
        let exp = *expected;
        match atomic.compare_exchange(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

// ============================================================================
// Memory fences
// ============================================================================

/// Atomic thread fence.
#[no_mangle]
pub extern "C" fn fragile_atomic_thread_fence(order: i32) {
    std::sync::atomic::fence(to_ordering(order));
}

/// Atomic signal fence (compiler fence).
#[no_mangle]
pub extern "C" fn fragile_atomic_signal_fence(order: i32) {
    std::sync::atomic::compiler_fence(to_ordering(order));
}

// ============================================================================
// Boolean atomic operations (for atomic<bool>)
// ============================================================================

/// Atomic load for boolean values.
#[no_mangle]
pub extern "C" fn fragile_atomic_load_bool(ptr: *const bool, order: i32) -> bool {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicBool);
        atomic.load(to_ordering(order))
    }
}

/// Atomic store for boolean values.
#[no_mangle]
pub extern "C" fn fragile_atomic_store_bool(ptr: *mut bool, value: bool, order: i32) {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicBool);
        atomic.store(value, to_ordering(order));
    }
}

/// Atomic exchange for boolean values.
#[no_mangle]
pub extern "C" fn fragile_atomic_exchange_bool(ptr: *mut bool, value: bool, order: i32) -> bool {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicBool);
        atomic.swap(value, to_ordering(order))
    }
}

/// Atomic compare-exchange (strong) for boolean values.
#[no_mangle]
pub extern "C" fn fragile_atomic_compare_exchange_strong_bool(
    ptr: *mut bool,
    expected: *mut bool,
    desired: bool,
    success_order: i32,
    failure_order: i32,
) -> i32 {
    unsafe {
        let atomic = &*(ptr as *const std::sync::atomic::AtomicBool);
        let exp = *expected;
        match atomic.compare_exchange(
            exp,
            desired,
            to_ordering(success_order),
            to_failure_ordering(failure_order),
        ) {
            Ok(_) => 1,
            Err(actual) => {
                *expected = actual;
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_32_load_store() {
        let mut value: u32 = 42;

        // Store
        fragile_atomic_store_32(&mut value, 100, 5); // seq_cst

        // Load
        let loaded = fragile_atomic_load_32(&value, 5); // seq_cst
        assert_eq!(loaded, 100);
    }

    #[test]
    fn test_atomic_32_exchange() {
        let mut value: u32 = 42;

        let old = fragile_atomic_exchange_32(&mut value, 100, 5);
        assert_eq!(old, 42);

        let current = fragile_atomic_load_32(&value, 5);
        assert_eq!(current, 100);
    }

    #[test]
    fn test_atomic_32_compare_exchange() {
        let mut value: u32 = 42;
        let mut expected: u32 = 42;

        // Should succeed - expected matches
        let result = fragile_atomic_compare_exchange_strong_32(
            &mut value,
            &mut expected,
            100,
            5, // success: seq_cst
            5, // failure: seq_cst
        );
        assert_eq!(result, 1);
        assert_eq!(fragile_atomic_load_32(&value, 5), 100);

        // Should fail - expected doesn't match
        expected = 42; // wrong expected value
        let result = fragile_atomic_compare_exchange_strong_32(
            &mut value,
            &mut expected,
            200,
            5,
            5,
        );
        assert_eq!(result, 0);
        assert_eq!(expected, 100); // expected updated to actual value
    }

    #[test]
    fn test_atomic_32_fetch_add() {
        let mut value: u32 = 42;

        let old = fragile_atomic_fetch_add_32(&mut value, 10, 5);
        assert_eq!(old, 42);
        assert_eq!(fragile_atomic_load_32(&value, 5), 52);
    }

    #[test]
    fn test_atomic_32_fetch_sub() {
        let mut value: u32 = 42;

        let old = fragile_atomic_fetch_sub_32(&mut value, 10, 5);
        assert_eq!(old, 42);
        assert_eq!(fragile_atomic_load_32(&value, 5), 32);
    }

    #[test]
    fn test_atomic_64_operations() {
        let mut value: u64 = 1000000000000;

        fragile_atomic_store_64(&mut value, 2000000000000, 5);
        assert_eq!(fragile_atomic_load_64(&value, 5), 2000000000000);

        let old = fragile_atomic_fetch_add_64(&mut value, 500000000000, 5);
        assert_eq!(old, 2000000000000);
        assert_eq!(fragile_atomic_load_64(&value, 5), 2500000000000);
    }

    #[test]
    fn test_atomic_bool() {
        let mut flag: bool = false;

        fragile_atomic_store_bool(&mut flag, true, 5);
        assert!(fragile_atomic_load_bool(&flag, 5));

        let old = fragile_atomic_exchange_bool(&mut flag, false, 5);
        assert!(old);
        assert!(!fragile_atomic_load_bool(&flag, 5));
    }

    #[test]
    fn test_atomic_ptr() {
        let mut data: i32 = 42;
        let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();

        let new_ptr = &mut data as *mut i32 as *mut std::ffi::c_void;
        fragile_atomic_store_ptr(&mut ptr, new_ptr, 5);

        let loaded = fragile_atomic_load_ptr(&ptr, 5);
        assert_eq!(loaded, new_ptr);

        // Verify we can use the pointer
        unsafe {
            assert_eq!(*(loaded as *mut i32), 42);
        }
    }

    #[test]
    fn test_atomic_bitwise() {
        let mut value: u32 = 0b1010;

        // AND
        let old = fragile_atomic_fetch_and_32(&mut value, 0b1100, 5);
        assert_eq!(old, 0b1010);
        assert_eq!(fragile_atomic_load_32(&value, 5), 0b1000);

        // OR
        value = 0b1010;
        let old = fragile_atomic_fetch_or_32(&mut value, 0b0101, 5);
        assert_eq!(old, 0b1010);
        assert_eq!(fragile_atomic_load_32(&value, 5), 0b1111);

        // XOR
        value = 0b1010;
        let old = fragile_atomic_fetch_xor_32(&mut value, 0b1100, 5);
        assert_eq!(old, 0b1010);
        assert_eq!(fragile_atomic_load_32(&value, 5), 0b0110);
    }

    #[test]
    fn test_memory_fence() {
        // Just verify these don't crash
        fragile_atomic_thread_fence(5); // seq_cst
        fragile_atomic_signal_fence(5); // seq_cst
    }

    #[test]
    fn test_memory_orderings() {
        let mut value: u32 = 0;

        // Test different orderings
        fragile_atomic_store_32(&mut value, 1, 0); // relaxed
        fragile_atomic_store_32(&mut value, 2, 3); // release
        assert_eq!(fragile_atomic_load_32(&value, 2), 2); // acquire
        assert_eq!(fragile_atomic_load_32(&value, 0), 2); // relaxed
    }

    #[test]
    fn test_multithread_atomic() {
        use std::sync::Arc;
        use std::thread;

        let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_ptr = counter.as_ref() as *const _ as *mut u32;
        let counter_usize = counter_ptr as usize;

        let mut handles = vec![];

        for _ in 0..4 {
            let counter_clone = counter.clone();
            let _ = counter_clone; // Just to keep the Arc alive

            let handle = thread::spawn(move || {
                let ptr = counter_usize as *mut u32;
                for _ in 0..100 {
                    fragile_atomic_fetch_add_32(ptr, 1, 5);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(fragile_atomic_load_32(counter_usize as *const u32, 5), 400);
    }
}
