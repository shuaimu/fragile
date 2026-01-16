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
