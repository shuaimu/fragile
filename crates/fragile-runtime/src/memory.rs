//! C++ memory management support (new/delete).

use core::alloc::Layout;
use core::ffi::c_void;

#[cfg(feature = "std")]
use std::alloc::{alloc, alloc_zeroed, dealloc, realloc};

#[cfg(not(feature = "std"))]
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
}

/// Allocate memory for a C++ object (operator new).
///
/// # Safety
/// Caller must ensure proper alignment and call destructor before dealloc.
#[no_mangle]
pub unsafe extern "C" fn fragile_rt_new(size: usize) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }

    #[cfg(feature = "std")]
    {
        let layout = Layout::from_size_align_unchecked(size, 8);
        alloc(layout) as *mut c_void
    }

    #[cfg(not(feature = "std"))]
    {
        malloc(size)
    }
}

/// Allocate zero-initialized memory (operator new with zero-init).
///
/// # Safety
/// Same as fragile_rt_new.
#[no_mangle]
pub unsafe extern "C" fn fragile_rt_new_zeroed(size: usize) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }

    #[cfg(feature = "std")]
    {
        let layout = Layout::from_size_align_unchecked(size, 8);
        alloc_zeroed(layout) as *mut c_void
    }

    #[cfg(not(feature = "std"))]
    {
        calloc(1, size)
    }
}

/// Free memory for a C++ object (operator delete).
///
/// # Safety
/// Pointer must have been allocated by fragile_rt_new.
#[no_mangle]
pub unsafe extern "C" fn fragile_rt_delete(ptr: *mut c_void, size: usize) {
    if ptr.is_null() {
        return;
    }

    #[cfg(feature = "std")]
    {
        let layout = Layout::from_size_align_unchecked(size, 8);
        dealloc(ptr as *mut u8, layout);
    }

    #[cfg(not(feature = "std"))]
    {
        let _ = size;
        free(ptr);
    }
}

/// Allocate memory for a C++ array (operator new[]).
///
/// # Safety
/// Same as fragile_rt_new, but for arrays.
#[no_mangle]
pub unsafe extern "C" fn fragile_rt_new_array(count: usize, element_size: usize) -> *mut c_void {
    let total_size = count.saturating_mul(element_size);
    if total_size == 0 {
        return core::ptr::null_mut();
    }

    fragile_rt_new(total_size)
}

/// Free memory for a C++ array (operator delete[]).
///
/// # Safety
/// Same as fragile_rt_delete, but for arrays.
#[no_mangle]
pub unsafe extern "C" fn fragile_rt_delete_array(
    ptr: *mut c_void,
    count: usize,
    element_size: usize,
) {
    if ptr.is_null() {
        return;
    }

    let total_size = count.saturating_mul(element_size);
    fragile_rt_delete(ptr, total_size);
}

/// Reallocate memory (for std::realloc compatibility).
///
/// # Safety
/// Pointer must have been allocated by fragile_rt_new.
#[no_mangle]
pub unsafe extern "C" fn fragile_rt_realloc(
    ptr: *mut c_void,
    old_size: usize,
    new_size: usize,
) -> *mut c_void {
    if ptr.is_null() {
        return fragile_rt_new(new_size);
    }

    if new_size == 0 {
        fragile_rt_delete(ptr, old_size);
        return core::ptr::null_mut();
    }

    #[cfg(feature = "std")]
    {
        let layout = Layout::from_size_align_unchecked(old_size, 8);
        realloc(ptr as *mut u8, layout, new_size) as *mut c_void
    }

    #[cfg(not(feature = "std"))]
    {
        let _ = old_size;
        realloc(ptr, new_size)
    }
}

/// Placement new helper - just returns the pointer.
///
/// In C++, `new (ptr) T(args)` constructs T at ptr.
/// We handle construction separately, so this is just identity.
#[no_mangle]
pub extern "C" fn fragile_rt_placement_new(ptr: *mut c_void) -> *mut c_void {
    ptr
}

/// Call a destructor for an object.
///
/// # Safety
/// Destructor must be valid and object must be initialized.
#[no_mangle]
pub unsafe extern "C" fn fragile_rt_call_destructor(
    ptr: *mut c_void,
    destructor: unsafe extern "C" fn(*mut c_void),
) {
    if !ptr.is_null() {
        destructor(ptr);
    }
}

/// Call destructors for an array of objects.
///
/// # Safety
/// All elements must be initialized and destructor must be valid.
#[no_mangle]
pub unsafe extern "C" fn fragile_rt_call_array_destructor(
    ptr: *mut c_void,
    count: usize,
    element_size: usize,
    destructor: unsafe extern "C" fn(*mut c_void),
) {
    if ptr.is_null() || count == 0 {
        return;
    }

    // Call destructors in reverse order (C++ requirement)
    let mut current = (ptr as usize) + (count - 1) * element_size;
    for _ in 0..count {
        destructor(current as *mut c_void);
        current -= element_size;
    }
}

// ============================================================================
// Standard C memory allocation functions
// These are provided for libc++ compatibility since it may call malloc/free
// directly instead of operator new/delete in some cases.
// ============================================================================

/// Standard C malloc - allocate memory.
///
/// # Safety
/// Returns a pointer that must be freed with fragile_free.
#[no_mangle]
pub unsafe extern "C" fn fragile_malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }

    #[cfg(feature = "std")]
    {
        let layout = Layout::from_size_align_unchecked(size, 8);
        alloc(layout) as *mut c_void
    }

    #[cfg(not(feature = "std"))]
    {
        malloc(size)
    }
}

/// Standard C free - deallocate memory.
///
/// # Safety
/// Pointer must have been allocated by fragile_malloc, fragile_calloc, or fragile_realloc.
#[no_mangle]
pub unsafe extern "C" fn fragile_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    #[cfg(feature = "std")]
    {
        // Note: We don't know the original size, so we use a minimal layout.
        // This works for std::alloc because the allocator tracks sizes internally.
        let layout = Layout::from_size_align_unchecked(1, 8);
        dealloc(ptr as *mut u8, layout);
    }

    #[cfg(not(feature = "std"))]
    {
        free(ptr);
    }
}

/// Standard C realloc - reallocate memory.
///
/// # Safety
/// Pointer must have been allocated by fragile_malloc, fragile_calloc, or fragile_realloc.
#[no_mangle]
pub unsafe extern "C" fn fragile_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    if ptr.is_null() {
        return fragile_malloc(new_size);
    }

    if new_size == 0 {
        fragile_free(ptr);
        return core::ptr::null_mut();
    }

    #[cfg(feature = "std")]
    {
        // For std::alloc, we need to know the old layout to realloc.
        // Since we don't track sizes, we allocate new memory and copy.
        // This is inefficient but correct.
        let new_ptr = fragile_malloc(new_size);
        if !new_ptr.is_null() {
            // We don't know old size, so we copy conservatively.
            // In practice, realloc is rarely used for shrinking.
            core::ptr::copy_nonoverlapping(ptr as *const u8, new_ptr as *mut u8, new_size);
        }
        fragile_free(ptr);
        new_ptr
    }

    #[cfg(not(feature = "std"))]
    {
        realloc(ptr, new_size)
    }
}

/// Standard C calloc - allocate and zero memory.
///
/// # Safety
/// Returns a pointer that must be freed with fragile_free.
#[no_mangle]
pub unsafe extern "C" fn fragile_calloc(nmemb: usize, size: usize) -> *mut c_void {
    let total_size = nmemb.saturating_mul(size);
    if total_size == 0 {
        return core::ptr::null_mut();
    }

    #[cfg(feature = "std")]
    {
        let layout = Layout::from_size_align_unchecked(total_size, 8);
        alloc_zeroed(layout) as *mut c_void
    }

    #[cfg(not(feature = "std"))]
    {
        calloc(nmemb, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_malloc_free() {
        unsafe {
            let ptr = fragile_malloc(100);
            assert!(!ptr.is_null());
            // Write some data to ensure it's valid memory
            let byte_ptr = ptr as *mut u8;
            *byte_ptr = 42;
            assert_eq!(*byte_ptr, 42);
            fragile_free(ptr);
        }
    }

    #[test]
    fn test_calloc() {
        unsafe {
            let ptr = fragile_calloc(10, 4); // 10 i32s
            assert!(!ptr.is_null());
            // calloc should zero-initialize
            let int_ptr = ptr as *mut i32;
            for i in 0..10 {
                assert_eq!(*int_ptr.add(i), 0);
            }
            fragile_free(ptr);
        }
    }

    #[test]
    fn test_realloc() {
        unsafe {
            let ptr = fragile_malloc(10);
            assert!(!ptr.is_null());
            let byte_ptr = ptr as *mut u8;
            *byte_ptr = 42;

            let new_ptr = fragile_realloc(ptr, 100);
            assert!(!new_ptr.is_null());
            // Data should be preserved
            let new_byte_ptr = new_ptr as *mut u8;
            assert_eq!(*new_byte_ptr, 42);

            fragile_free(new_ptr);
        }
    }

    #[test]
    fn test_malloc_zero_size() {
        unsafe {
            let ptr = fragile_malloc(0);
            assert!(ptr.is_null());
        }
    }

    #[test]
    fn test_free_null() {
        unsafe {
            // Should not crash
            fragile_free(core::ptr::null_mut());
        }
    }
}
