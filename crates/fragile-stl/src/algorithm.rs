// STL algorithm stubs

/// std::sort(first, last) - sorts range [first, last) in ascending order
pub fn std_sort_int(first: *mut i32, last: *mut i32) {
    if first.is_null() || last.is_null() { return; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return; }
    let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };
    slice.sort();
}

/// std::find(first, last, value) - returns iterator to first match or last
pub fn std_find_int(first: *const i32, last: *const i32, value: i32) -> *const i32 {
    if first.is_null() || last.is_null() { return last; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return last; }
    let slice = unsafe { std::slice::from_raw_parts(first, len) };
    match slice.iter().position(|&x| x == value) {
        Some(idx) => unsafe { first.add(idx) },
        None => last,
    }
}

/// std::count(first, last, value) - counts occurrences of value in range
pub fn std_count_int(first: *const i32, last: *const i32, value: i32) -> usize {
    if first.is_null() || last.is_null() { return 0; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return 0; }
    let slice = unsafe { std::slice::from_raw_parts(first, len) };
    slice.iter().filter(|&&x| x == value).count()
}

/// std::copy(first, last, dest) - copies range to dest, returns end of dest
pub fn std_copy_int(first: *const i32, last: *const i32, dest: *mut i32) -> *mut i32 {
    if first.is_null() || last.is_null() || dest.is_null() { return dest; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return dest; }
    unsafe { std::ptr::copy_nonoverlapping(first, dest, len); }
    unsafe { dest.add(len) }
}

/// std::fill(first, last, value) - fills range with value
pub fn std_fill_int(first: *mut i32, last: *mut i32, value: i32) {
    if first.is_null() || last.is_null() { return; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return; }
    let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };
    for elem in slice.iter_mut() { *elem = value; }
}

/// std::reverse(first, last) - reverses range in place
pub fn std_reverse_int(first: *mut i32, last: *mut i32) {
    if first.is_null() || last.is_null() { return; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return; }
    let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };
    slice.reverse();
}

pub type tuple_type_parameter_0_0___ = std::ffi::c_void;
pub type _Int__Tp = std::ffi::c_void;
pub type _Tp = std::ffi::c_void;
pub type _Up = std::ffi::c_void;
pub type _Args = std::ffi::c_void;
pub type _Elements___ = std::ffi::c_void;

// Template type alias placeholder
pub type value_type = std::ffi::c_void;

// System header union type stubs
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct union__unnamed_union_at__usr_include_x86_64_linux_gnu_bits_types___mbstate_t_h_16_3_ { pub __wch: u32 }

// libc++ internal function stubs
#[inline]
pub fn __hash(_ptr: *const i8) -> usize {
    // FNV-1a hash for null-terminated string
    let mut hash: usize = 14695981039346656037;
    if _ptr.is_null() { return hash; }
    let mut p = _ptr;
    unsafe {
        while *p != 0 {
            hash ^= *p as usize;
            hash = hash.wrapping_mul(1099511628211);
            p = p.add(1);
        }
    }
    hash
}

#[inline]
pub fn __string_to_type_name(_ptr: *const i8) -> *const i8 { _ptr }

// std::piecewise_construct_t and constant for pair construction
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct piecewise_construct_t {}
pub static piecewise_construct: piecewise_construct_t = piecewise_construct_t {};

// std::forward_as_tuple stubs for pair construction in map emplace
// These return empty tuple or tuple containing the value
#[inline]
pub fn forward_as_tuple<T>(x: T) -> tuple_element_1<T> { tuple_element_1 { _0: x } }

#[repr(C)]
#[derive(Default, Clone)]
pub struct tuple_element_1<T> { pub _0: T }
