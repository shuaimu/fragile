// Format context types
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct basic_format_parse_context_char;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct basic_format_parse_context_wchar_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct basic_format_parse_context_typename_type_parameter_0_0_char_type;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct basic_format_context_back_insert_iterator___format___output_buffer_char__char;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct basic_format_context_back_insert_iterator___format___output_buffer_wchar_t__wchar_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct basic_format_args_format_context;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct basic_format_args_wformat_context;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __compile_time_basic_format_context_type_parameter_0_0;
pub type basic_string_view_typename_type_parameter_0_0_char_type__char_traits_typename_type_parameter_0_0_char_type = std::ffi::c_void;

// Allocator traits types
pub type allocator_traits_typename_allocator_traits_type_parameter_0_1_template_rebind_alloc_typename_allocator_traits_type_parameter_0_1_pointer = std::ffi::c_void;
pub type allocator_traits_typename_allocator_traits_type_parameter_0_3_template_rebind_alloc___hash_node_type_parameter_0_0__typename_allocator_traits_type_parameter_0_3_void_pointer = std::ffi::c_void;
pub type __allocation_result_typename_allocator_traits_type_parameter_0_2_pointer__typename_allocator_traits_type_parameter_0_2_size_type = std::ffi::c_void;

// Additional template types
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __alignment_checker_type__Alignment;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_waitable_traits___decay_type_parameter_0_0___void;
pub type __const_iterator = std::ffi::c_void;
pub type _BMSkipTable_typename_iterator_traits_type_parameter_0_0_value_type__typename_iterator_traits_type_parameter_0_0_difference_type__type_parameter_0_1__type_parameter_0_2__is_integral_v_value_type___sizeof_value_type___eq__1___is_same_v__Hash__hash_value_type___is_same_v__BinaryPredicate__equal_to_ = std::ffi::c_void;

// Format and unicode type stubs
pub type std___indic_conjunct_break___property = u32;
pub type std___unicode___consume_result__unnamed_enum_at__home_shuai_workspace_fragile_vendor_llvm_project_libcxx_include___format_unicode_h_48_3_ = u32;
pub type std___format_spec___sign = u32;
pub type std_basic_format_parse_context__Indexing = u32;

// Pointer and iterator types
pub type __add_pointer_const_type_parameter_0_0_ = *const std::ffi::c_void;
pub type __add_pointer_type_parameter_0_0_ = *mut std::ffi::c_void;
pub type __bit_iterator_type_parameter_0_0__true__0 = std::ffi::c_void;
pub type __bit_iterator_type_parameter_0_0__false__0 = std::ffi::c_void;
pub type array__Tp___Size = std::ffi::c_void;
pub type tuple_type_parameter_0_0_____ = std::ffi::c_void;
pub type basic_string_view_type_parameter_0_0__char_traits_type_parameter_0_0 = std::ffi::c_void;
pub type basic_format_arg_type_parameter_0_0 = std::ffi::c_void;
pub type allocator_type_parameter_0_0 = std::ffi::c_void;
pub type allocator_traits_type_parameter_0_0 = std::ffi::c_void;
pub type __basic_format_arg_value_type_parameter_0_0 = std::ffi::c_void;
pub type __output_buffer_type_parameter_0_0 = std::ffi::c_void;
pub type _SentinelValueFill_type_parameter_0_1 = std::ffi::c_void;
pub type __compressed_pair_padding_type_parameter_0_2____is_reference_or_unpadded_object__Alloc = std::ffi::c_void;
pub type basic_string_char__std_char_traits_char__type_parameter_0_3 = std::ffi::c_void;
pub type __tuple_impl___make_integer_seq_std___integer_sequence__unsigned_long__sizeof_____Args___type_parameter_0_0___ = std::ffi::c_void;
pub type __make_unsigned_typename_conditional___is_primary_template_iterator_traits_remove_cvref_t__Ip_value__incrementable_traits___remove_cvref_type_parameter_0_0___iterator_traits___remove_cvref_type_parameter_0_0__type_difference_type_ = std::ffi::c_void;

// Struct stubs for types used with constructor/method calls
#[repr(C)]
#[derive(Default, Clone)]
pub struct basic_string_view_char { pub __data_: *const i8, pub __size_: u64 }
impl basic_string_view_char {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_1(__str: *const i8) -> Self { Self { __data_: __str, __size_: 0 } }
    pub fn new_2(__str: *const i8, __len: u64) -> Self { Self { __data_: __str, __size_: __len } }
    pub fn new_3(_tag: u64, __str: *const i8, __len: u64) -> Self { Self { __data_: __str, __size_: __len } }
}
#[repr(C)]
#[derive(Default, Clone)]
pub struct basic_string_view_wchar_t { pub __data_: *const i32, pub __size_: u64 }
impl basic_string_view_wchar_t {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_3(_tag: u64, __str: *const i32, __len: u64) -> Self { Self { __data_: __str, __size_: __len } }
}
#[repr(C)]
#[derive(Default, Clone)]
pub struct basic_string_view_char8_t { pub __data_: *const u8, pub __size_: u64 }
impl basic_string_view_char8_t {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_3(_tag: u64, __str: *const u8, __len: u64) -> Self { Self { __data_: __str, __size_: __len } }
    pub fn data(&self) -> *const u8 { self.__data_ }
    pub fn length(&self) -> u64 { self.__size_ }
    pub fn size(&self) -> u64 { self.__size_ }
    pub fn empty(&self) -> bool { self.__size_ == 0 }
}
#[repr(C)]
#[derive(Default, Clone)]
pub struct basic_string_view_char16_t { pub __data_: *const u16, pub __size_: u64 }
impl basic_string_view_char16_t {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_3(_tag: u64, __str: *const u16, __len: u64) -> Self { Self { __data_: __str, __size_: __len } }
    pub fn data(&self) -> *const u16 { self.__data_ }
    pub fn length(&self) -> u64 { self.__size_ }
    pub fn size(&self) -> u64 { self.__size_ }
    pub fn empty(&self) -> bool { self.__size_ == 0 }
}
#[repr(C)]
#[derive(Default, Clone)]
pub struct basic_string_view_char32_t { pub __data_: *const u32, pub __size_: u64 }
impl basic_string_view_char32_t {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_3(_tag: u64, __str: *const u32, __len: u64) -> Self { Self { __data_: __str, __size_: __len } }
    pub fn data(&self) -> *const u32 { self.__data_ }
    pub fn length(&self) -> u64 { self.__size_ }
    pub fn size(&self) -> u64 { self.__size_ }
    pub fn empty(&self) -> bool { self.__size_ == 0 }
}

// Template instantiation stubs with constructors
#[repr(C)]
#[derive(Default, Clone)]
pub struct tuple_ { }
impl tuple_ {
    pub fn new_0() -> Self { Self { } }
    pub fn new_1(_unused: i32) -> Self { Self { } }
}
#[repr(C)]
#[derive(Default, Clone)]
pub struct __cxx_atomic_impl_bool { pub __a_value: bool }
impl __cxx_atomic_impl_bool {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_1(_val: bool) -> Self { Self { __a_value: _val } }
}
pub type __cxx_atomic_base_impl_bool = __cxx_atomic_impl_bool;

// Atomic operation stubs for libc++ atomics
#[inline]
pub fn __cxx_atomic_load___cxx_atomic_base_impl_bool<M>(_ptr: *const __cxx_atomic_base_impl_bool, _order: M) -> bool {
    let _ = _order;
    unsafe { (*_ptr).__a_value }
}
#[inline]
pub fn __cxx_atomic_store___cxx_atomic_base_impl_bool<M>(_ptr: *mut __cxx_atomic_base_impl_bool, _val: bool, _order: M) {
    let _ = _order;
    unsafe { (*_ptr).__a_value = _val; }
}
#[inline]
pub fn __cxx_atomic_exchange___cxx_atomic_base_impl_bool<M>(_ptr: *mut __cxx_atomic_base_impl_bool, _val: bool, _order: M) -> bool {
    let _ = _order;
    unsafe { let old = (*_ptr).__a_value; (*_ptr).__a_value = _val; old }
}

// atomic_flag test method extension trait
pub trait AtomicFlagTest { fn test<M>(&self, _m: M) -> bool; }

// Shared pointer reference counting stub traits
pub trait SharedPtrOnZeroShared { fn __on_zero_shared(&mut self); }
pub trait SharedPtrReleaseWeak { fn __release_weak(&mut self); }

// Format error throw stub
#[inline(always)]
pub fn __throw_invalid_type_format_error<T>(_id: T) { panic!("invalid type format"); }
// char_traits module stub
pub mod char_traits {
    pub fn length<T: Copy + Default + PartialEq>(_s: *const T) -> u64 {
        unsafe {
            let mut len = 0u64;
            let zero: T = Default::default();
            while *_s.add(len as usize) != zero { len += 1; }
            len
        }
    }
    pub fn copy<T: Copy>(_dest: *mut T, _src: *const T, _n: u64) -> *mut T { unsafe { std::ptr::copy_nonoverlapping(_src, _dest, _n as usize); _dest } }
    pub fn compare<T: Copy + Ord>(_s1: *const T, _s2: *const T, _n: u64) -> i32 {
        unsafe {
            for i in 0.._n as usize {
                let a = *_s1.add(i);
                let b = *_s2.add(i);
                match a.cmp(&b) { std::cmp::Ordering::Less => return -1, std::cmp::Ordering::Greater => return 1, _ => {} }
            }
            0
        }
    }
    pub fn eq<T: PartialEq>(_a: &T, _b: &T) -> bool { *_a == *_b }
    pub fn lt<T: PartialOrd>(_a: &T, _b: &T) -> bool { *_a < *_b }
    pub fn eq_int_type<T: PartialEq>(_a: T, _b: T) -> bool { _a == _b }
    pub fn to_char_type(_c: i32) -> i8 { _c as i8 }
    pub fn to_int_type(_c: i8) -> i32 { _c as i32 }
    pub fn eof() -> i32 { -1 }
    pub fn not_eof(_c: i32) -> i32 { if _c == -1 { 0 } else { _c } }
    
    // move functions for different char types
    pub fn move_ptr_mut_i8_ptr_const_i8(_dest: *mut i8, _src: *const i8, _n: u64) -> *mut i8 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }
    pub fn move_ptr_mut_i32_ptr_const_i32(_dest: *mut i32, _src: *const i32, _n: u64) -> *mut i32 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }
    pub fn move_ptr_mut_u8_ptr_const_u8(_dest: *mut u8, _src: *const u8, _n: u64) -> *mut u8 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }
    pub fn move_ptr_mut_u16_ptr_const_u16(_dest: *mut u16, _src: *const u16, _n: u64) -> *mut u16 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }
    pub fn move_ptr_mut_u32_ptr_const_u32(_dest: *mut u32, _src: *const u32, _n: u64) -> *mut u32 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }
    
    // assign functions for different char types (fill)
    pub fn assign_ptr_mut_i8(_s: *mut i8, _n: u64, _a: i8) -> *mut i8 { unsafe { for i in 0.._n as usize { *_s.add(i) = _a; } _s } }
    pub fn assign_ptr_mut_i32(_s: *mut i32, _n: u64, _a: i32) -> *mut i32 { unsafe { for i in 0.._n as usize { *_s.add(i) = _a; } _s } }
    pub fn assign_ptr_mut_u8(_s: *mut u8, _n: u64, _a: u8) -> *mut u8 { unsafe { for i in 0.._n as usize { *_s.add(i) = _a; } _s } }
    pub fn assign_u16(_dest: &mut u16, _src: &u16) { *_dest = *_src; }
    pub fn assign_u32(_dest: &mut u32, _src: &u32) { *_dest = *_src; }
    
    // compare functions for different char types
    pub fn compare_ptr_const_i32(_s1: *const i32, _s2: *const i32, _n: u64) -> i32 { unsafe { for i in 0.._n as usize { let a = *_s1.add(i); let b = *_s2.add(i); if a != b { return if a < b { -1 } else { 1 }; } } 0 } }
    pub fn compare_ptr_const_u8(_s1: *const u8, _s2: *const u8, _n: u64) -> i32 { unsafe { for i in 0.._n as usize { let a = *_s1.add(i); let b = *_s2.add(i); if a != b { return if a < b { -1 } else { 1 }; } } 0 } }
}

// construct_at stubs for placement new (C++20 std::construct_at)
#[inline]
pub fn construct_at_i8_ref_i8(_p: *const i8, _val: i8) -> *mut i8 { unsafe { let p = _p as *mut i8; *p = _val; p } }
#[inline]
pub fn construct_at_i32_ref_i32(_p: *const i32, _val: i32) -> *mut i32 { unsafe { let p = _p as *mut i32; *p = _val; p } }
#[inline]
pub fn construct_at_u8_ref_u8(_p: *const u8, _val: u8) -> *mut u8 { unsafe { let p = _p as *mut u8; *p = _val; p } }
#[inline]
pub fn construct_at_u16_ref_u16(_p: *const u16, _val: u16) -> *mut u16 { unsafe { let p = _p as *mut u16; *p = _val; p } }
#[inline]
pub fn construct_at_u32_ref_u32(_p: *const u32, _val: u32) -> *mut u32 { unsafe { let p = _p as *mut u32; *p = _val; p } }

// STL algorithm stubs
#[inline]
pub fn upper_bound_unsigned_int_unsigned_int(_first: *const u32, _last: *const u32, _val: u32) -> *const u32 {
    // Binary search for upper bound - returns pointer like C++ iterator
    unsafe {
        let len = (_last as usize - _first as usize) / std::mem::size_of::<u32>();
        let mut lo = 0usize;
        let mut hi = len;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if *_first.add(mid) <= _val { lo = mid + 1; } else { hi = mid; }
        }
        _first.add(lo)
    }
}

// Array begin/end stubs (template instantiations)
#[inline] pub fn begin__u32__110_(__arr: *mut [u32; 110]) -> *const u32 { unsafe { (*__arr).as_ptr() } }
#[inline] pub fn end__u32__110_(__arr: *mut [u32; 110]) -> *const u32 { unsafe { (*__arr).as_ptr().add(110) } }
#[inline] pub fn begin__u32__403_(__arr: *mut [u32; 403]) -> *const u32 { unsafe { (*__arr).as_ptr() } }
#[inline] pub fn end__u32__403_(__arr: *mut [u32; 403]) -> *const u32 { unsafe { (*__arr).as_ptr().add(403) } }
#[inline] pub fn begin__u32__1501_(__arr: *mut [u32; 1501]) -> *const u32 { unsafe { (*__arr).as_ptr() } }
#[inline] pub fn end__u32__1501_(__arr: *mut [u32; 1501]) -> *const u32 { unsafe { (*__arr).as_ptr().add(1501) } }

// UTF-8 encoding helper stubs
#[inline]
pub fn __is_continuation_char(_c: u8) -> bool { (_c & 0xC0) == 0x80 }

// C++20 bit manipulation stubs (std::countl_one, etc.)
#[inline]
pub fn countl_one_u8(x: u8) -> u32 { (!x).leading_zeros() as u32 - 24 }
#[inline]
pub fn countl_zero_u8(x: u8) -> u32 { x.leading_zeros() as u32 - 24 }
#[inline]
pub fn __countl_zero_u64(x: u64) -> u32 { x.leading_zeros() }

// Red-black tree internal functions for map/set iterators.
// These are boundary-navigation helpers used by iterator increment/decrement
// paths and intentionally model in-order predecessor/successor traversal.
#[inline]
unsafe fn __rb_tree_min(mut node: *mut __tree_node_base) -> *mut __tree_node_base {
    while !node.is_null() && !(*node).__left_.is_null() {
        node = (*node).__left_;
    }
    node
}

#[inline]
unsafe fn __rb_tree_max(mut node: *mut __tree_node_base) -> *mut __tree_node_base {
    while !node.is_null() && !(*node).__right_.is_null() {
        node = (*node).__right_;
    }
    node
}

#[inline]
unsafe fn __rb_tree_is_header(node: *mut __tree_node_base) -> bool {
    if node.is_null() || (*node).__is_black_ {
        return false;
    }
    let parent = (*node).__parent_ as *mut __tree_node_base;
    if parent.is_null() {
        return false;
    }
    ((*parent).__parent_ as *mut __tree_node_base) == node
}

pub fn _Rb_tree_increment(node: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
    if node.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let mut current = node as *mut __tree_node_base;
        if !(*current).__right_.is_null() {
            return __rb_tree_min((*current).__right_) as *mut std::ffi::c_void;
        }

        let mut parent = (*current).__parent_ as *mut __tree_node_base;
        while !parent.is_null() && current == (*parent).__right_ {
            current = parent;
            parent = (*parent).__parent_ as *mut __tree_node_base;
        }

        if parent.is_null() {
            return std::ptr::null_mut();
        }
        if (*current).__right_ != parent {
            current = parent;
        }
        current as *mut std::ffi::c_void
    }
}

pub fn _Rb_tree_decrement(node: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
    if node.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let mut current = node as *mut __tree_node_base;
        if __rb_tree_is_header(current) {
            return (*current).__right_ as *mut std::ffi::c_void;
        }

        if !(*current).__left_.is_null() {
            return __rb_tree_max((*current).__left_) as *mut std::ffi::c_void;
        }

        let mut parent = (*current).__parent_ as *mut __tree_node_base;
        while !parent.is_null() && current == (*parent).__left_ {
            current = parent;
            parent = (*parent).__parent_ as *mut __tree_node_base;
        }

        if parent.is_null() {
            return std::ptr::null_mut();
        }
        parent as *mut std::ffi::c_void
    }
}

// iostream type aliases
pub type basic_filebuf_char = std::ffi::c_void;
pub type basic_filebuf_wchar_t = std::ffi::c_void;
pub type basic_ifstream_char = std::ffi::c_void;
pub type basic_ifstream_wchar_t = std::ffi::c_void;
pub type basic_ofstream_char = std::ffi::c_void;
pub type basic_ofstream_wchar_t = std::ffi::c_void;
pub type basic_fstream_char = std::ffi::c_void;
pub type basic_fstream_wchar_t = std::ffi::c_void;
pub type basic_ios_char = std::ffi::c_void;
pub type basic_ios_wchar_t = std::ffi::c_void;
pub type basic_istream_char = std::ffi::c_void;
pub type basic_istream_wchar_t = std::ffi::c_void;
pub type basic_ostream_char = std::ffi::c_void;
pub type basic_ostream_wchar_t = std::ffi::c_void;
pub type basic_iostream_char = std::ffi::c_void;
pub type basic_iostream_wchar_t = std::ffi::c_void;
pub type basic_streambuf_char = std::ffi::c_void;
pub type basic_streambuf_wchar_t = std::ffi::c_void;
pub type basic_stringbuf_char = std::ffi::c_void;
pub type basic_stringbuf_wchar_t = std::ffi::c_void;
pub type basic_istringstream_char = std::ffi::c_void;
pub type basic_istringstream_wchar_t = std::ffi::c_void;
pub type basic_ostringstream_char = std::ffi::c_void;
pub type basic_ostringstream_wchar_t = std::ffi::c_void;
pub type basic_stringstream_char = std::ffi::c_void;
pub type basic_stringstream_wchar_t = std::ffi::c_void;

// Template parameter placeholder types
pub type __impl_type_parameter_0_0___ = std::ffi::c_void;
pub type __remove_reference_t__Tp_ = std::ffi::c_void;
pub type __remove_cvref_type_parameter_0_1_ = std::ffi::c_void;
pub type __swap___fn = std::ffi::c_void;
pub type __strong_order___fn = std::ffi::c_void;
pub type __weak_order___fn = std::ffi::c_void;
pub type __partial_order___fn = std::ffi::c_void;
pub type __compare_partial_order_fallback___fn = std::ffi::c_void;
pub type __compare_strong_order_fallback___fn = std::ffi::c_void;
pub type __compare_weak_order_fallback___fn = std::ffi::c_void;
pub type back_insert_iterator = std::ffi::c_void;

// Function stubs
pub fn __gv_swap<T>(_a: &mut T, _b: &mut T) { std::mem::swap(_a, _b); }
pub fn r#move<T>(x: T) -> T { x }
pub fn uselocale(_locale: *mut std::ffi::c_void) -> *mut std::ffi::c_void { std::ptr::null_mut() }
pub fn max_f64(a: f64, b: f64) -> f64 { if a > b { a } else { b } }
pub fn equal<T: PartialEq>(_first1: *const T, _last1: *const T, _first2: *const T) -> bool { true }
pub fn __libcpp_atomic_refcount_increment_i64(_ptr: *mut i64) -> i64 { unsafe { *_ptr += 1; *_ptr } }
pub fn __libcpp_atomic_refcount_decrement_i64(_ptr: *mut i64) -> i64 { unsafe { *_ptr -= 1; *_ptr } }
// GCC/libstdc++ atomic functions (single-threaded stubs)
pub fn __exchange_and_add(__mem: *mut i32, __val: i32) -> i32 { unsafe { let old = *__mem; *__mem += __val; old } }
pub fn __atomic_add(__mem: *mut i32, __val: i32) { unsafe { *__mem += __val; } }
// Atomic wait/notify stubs (no-op placeholders)
pub fn __atomic_wait_std_atomic_flag_bool<T, M>(_: T, _: bool, _: M) {}
pub fn __atomic_notify_one_std_atomic_flag<T>(_: T) {}
pub fn __atomic_notify_all_std_atomic_flag<T>(_: T) {}
// Math function stubs
pub fn __lerp_f64(a: f64, b: f64, t: f64) -> f64 { a + t * (b - a) }
pub fn __hypot_f64(x: f64, y: f64, z: f64) -> f64 { (x * x + y * y + z * z).sqrt() }
pub fn __hermite_u32(_n: u32, _x: f64) -> f64 { 0.0 }

// Shared pointer support
pub static __Control: () = ();

// Atomic and system call function stubs
#[inline] pub unsafe fn __atomic_thread_fence(_order: i32) { std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst) }
#[inline] pub unsafe fn __atomic_signal_fence(_order: i32) { std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst) }
#[inline] pub unsafe fn __atomic_test_and_set(_ptr: *mut bool, _order: i32) -> bool { false }
#[inline] pub unsafe fn __atomic_clear(_ptr: *mut bool, _order: i32) { }
#[inline] pub unsafe fn __atomic_wait_address_v_bool_bool(_ptr: *const bool, _expected: bool) { }
#[inline] pub fn __atomic_notify_address_bool(_addr: *mut bool, _all: bool) { }
#[inline] pub fn __platform_wait_i32(_addr: *const i32, _val: i32) { }
#[inline] pub fn __platform_notify_i32(_addr: *const i32, _all: bool) { }
#[inline] pub unsafe fn __builtin_ia32_pause() { std::hint::spin_loop() }
#[inline] pub unsafe fn syscall(_num: i64) -> i64 { 0 }
#[inline] pub unsafe fn sem_init(_sem: *mut std::ffi::c_void, _pshared: i32, _value: u32) -> i32 { 0 }
#[inline] pub unsafe fn sem_destroy(_sem: *mut std::ffi::c_void) -> i32 { 0 }
#[inline] pub unsafe fn __gthread_cond_timedwait(_cond: *mut std::ffi::c_void, _mutex: *mut std::ffi::c_void, _abs: *const std::ffi::c_void) -> i32 { 0 }
#[inline] pub unsafe fn pthread_cond_clockwait(_cond: *mut std::ffi::c_void, _mutex: *mut std::ffi::c_void, _clock: i32, _abs: *const std::ffi::c_void) -> i32 { 0 }
#[inline] pub fn __terminate() { std::process::abort() }
#[inline] pub fn __throw_system_error(_err: i32) { panic!("system error") }
#[inline] pub fn load_i32(ptr: *const i32) -> i32 { unsafe { std::ptr::read_volatile(ptr) } }
#[inline] pub fn compare_exchange_strong_i32(_ptr: *mut i32, _expected: *mut i32, _desired: i32, _success: i32, _fail: i32) -> bool { false }
#[inline] pub fn fetch_add_i32(ptr: *mut i32, val: i32, _order: i32) -> i32 { unsafe { let old = *ptr; *ptr += val; old } }
#[inline] pub fn _S_do_try_acquire(_sem: *mut std::ffi::c_void) -> bool { false }
#[inline] pub fn __atomic_spin___std___detail___default_spin_policy(_f: impl FnMut() -> bool, _p: std::ffi::c_void) -> bool { false }
#[inline] pub fn swap_std_thread_id_std_thread_id(a: *mut u64, b: *mut u64) { unsafe { std::ptr::swap(a, b) } }
#[inline] pub fn swap_std_stop_source_std_stop_source(a: *mut std::ffi::c_void, b: *mut std::ffi::c_void) { unsafe { std::ptr::swap(a, b) } }
#[inline] pub fn swap_thread_thread(a: *mut std::ffi::c_void, b: *mut std::ffi::c_void) { unsafe { std::ptr::swap(a, b) } }
pub const NaN: f64 = f64::NAN;
#[inline] pub fn pthread_self() -> u64 { 0 }
pub const __wait_private: i32 = 0x80;
pub const __wake_private: i32 = 0x80;

// Thread-related type stubs
pub type thread_id = u64;
pub type __native_type = usize;
pub type __wait_clock_t_time_point = u64;
pub type _Stop_state_ref = std::ffi::c_void;
pub type stop_token__Stop_state_ref = std::ffi::c_void;
pub type __waiter_std_true_type = std::ffi::c_void;
pub type __waiter_std_false_type = std::ffi::c_void;
pub type __waiter_base___waiter_pool = std::ffi::c_void;
pub type __timed_waiter_std_true_type = std::ffi::c_void;
pub type __timed_waiter_std_false_type = std::ffi::c_void;
pub type __detail___bare_wait = std::ffi::c_void;

// Atomic type stubs
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_bool { pub _M_i: bool }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_char { pub _M_i: i8 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_signed_char { pub _M_i: i8 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_unsigned_char { pub _M_i: u8 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_short { pub _M_i: i16 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_unsigned_short { pub _M_i: u16 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_int { pub _M_i: i32 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_unsigned_int { pub _M_i: u32 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_long { pub _M_i: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_unsigned_long { pub _M_i: u64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_long_long { pub _M_i: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_unsigned_long_long { pub _M_i: u64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_wchar_t { pub _M_i: i32 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_char8_t { pub _M_i: u8 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_char16_t { pub _M_i: u16 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_base_char32_t { pub _M_i: u32 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_flag_base { pub _M_i: bool }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_float_float { pub _M_fp: f32 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_float_double { pub _M_fp: f64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __atomic_float_long_double { pub _M_fp: f64 }

impl __atomic_base_bool {
    #[inline] pub fn new_0() -> Self { Self { _M_i: false } }
    #[inline] pub fn new_1(val: bool) -> Self { Self { _M_i: val } }
    #[inline] pub fn op_assign(&mut self, val: bool) -> &mut Self { self._M_i = val; self }
    #[inline] pub fn load(&self, _order: i32) -> bool { self._M_i }
    #[inline] pub fn store(&mut self, val: bool, _order: i32) { self._M_i = val; }
    #[inline] pub fn exchange(&mut self, val: bool, _order: i32) -> bool { let old = self._M_i; self._M_i = val; old }
    #[inline] pub fn compare_exchange_weak(&mut self, expected: &mut bool, desired: bool, _success: i32, _fail: i32) -> bool {
        if self._M_i == *expected { self._M_i = desired; true } else { *expected = self._M_i; false }
    }
    #[inline] pub fn compare_exchange_strong(&mut self, expected: &mut bool, desired: bool, _success: i32, _fail: i32) -> bool {
        self.compare_exchange_weak(expected, desired, _success, _fail)
    }
    #[inline] pub fn is_lock_free(&self) -> bool { true }
    #[inline] pub fn wait(&self, _old: bool, _order: i32) { }
    #[inline] pub fn notify_one(&self) { }
    #[inline] pub fn notify_all(&self) { }
}

// Memory order constants
pub const memory_order_relaxed: i32 = 0;
pub const memory_order_consume: i32 = 1;
pub const memory_order_acquire: i32 = 2;
pub const memory_order_release: i32 = 3;
pub const memory_order_acq_rel: i32 = 4;
pub const memory_order_seq_cst: i32 = 5;
pub const __memory_order_mask: i32 = 0x0ffff;
pub const __memory_order_modifier_mask: i32 = 0xffff0000u32 as i32;

// More libstdc++ type stubs
pub type basic_ostream_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;
pub type memory_resource = std::ffi::c_void;
pub type num_put_type_parameter_0_0__ostreambuf_iterator_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;
pub type num_get_type_parameter_0_0__istreambuf_iterator_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;
pub type basic_syncbuf_char = std::ffi::c_void;
pub type basic_osyncstream_char = std::ffi::c_void;
pub type basic_syncbuf_wchar_t = std::ffi::c_void;
pub type basic_osyncstream_wchar_t = std::ffi::c_void;
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct __pthread_list_t { pub __prev: *mut (), pub __next: *mut () }
pub type _unnamed_struct_at__usr_include_x86_64_linux_gnu_bits_atomic_wide_counter_h_28_3_ = std::ffi::c_void;
pub type _Maybe_unary_or_binary_function_type_parameter_0_0__type_parameter_0_1__type_parameter_0_2___ = std::ffi::c_void;
pub type integral_constant_unsigned_long__sizeof_____ArgTypes_ = std::ffi::c_void;
#[derive(Default, Clone, Copy)] pub struct __hash_base_size_t__string_view;
#[derive(Default, Clone, Copy)] pub struct __hash_base_size_t__wstring_view;
#[derive(Default, Clone, Copy)] pub struct __hash_base_size_t__u8string_view;
#[derive(Default, Clone, Copy)] pub struct __hash_base_size_t__u16string_view;
#[derive(Default, Clone, Copy)] pub struct __hash_base_size_t__u32string_view;
#[derive(Default, Clone, Copy)] pub struct __hash_base_size_t__error_code;
#[derive(Default, Clone, Copy)] pub struct __hash_base_size_t__error_condition;
#[derive(Default, Clone, Copy)] pub struct _Tuple_impl_0__type_parameter_0_0___;
pub type _Callback_list = *mut ();
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __ctype_abstract_base_wchar_t { pub __vtable: *const std::ffi::c_void }
pub type __cv_selector_type_parameter_0_1___IsConst___IsVol = std::ffi::c_void;
pub type __strictest_alignment_type_parameter_0_1___ = std::ffi::c_void;
pub type _Size_decltype___detail___to_unsigned_like_std_declval_typename___iter_traits_impl_typename_remove_cvref_type_parameter_0_0_type__incrementable_traits_typename_remove_cvref_type_parameter_0_0_type_type_difference_type_______S_store_size = usize;
pub type std_strong_ordering = i8;

// Chrono type stubs
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct chrono_microseconds { pub _M_r: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct chrono_milliseconds { pub _M_r: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct chrono_seconds { pub _M_r: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct chrono_minutes { pub _M_r: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct chrono_hours { pub _M_r: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct chrono_nanoseconds { pub _M_r: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct chrono_steady_clock { pub _M_t: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct chrono_system_clock { pub _M_t: i64 }
pub type chrono_time_point___clock_t = i64;
pub type time_point = i64;

// Ratio type stubs (SI prefixes)
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1__1000000000000000000L { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1__1000000000000000L { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1__1000000000000L { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1__1000000000 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1__1000000 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1__1000 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1__100 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1__10 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_10__1 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_100__1 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1000__1 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1000000__1 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1000000000__1 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1000000000000L__1 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1000000000000000L__1 { pub num: i64, pub den: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct ratio_1000000000000000000L__1 { pub num: i64, pub den: i64 }

// Thread and stop token type stubs
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct std_thread { pub _M_id: u64 }
impl std_thread {
    pub fn new_0() -> Self { Self { _M_id: 0 } }
    pub fn join(&mut self) { /* stub: actual thread join not implemented */ }
}
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct std_nostopstate_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct std_counting_semaphore_1 { pub _M_counter: i32 }
pub const nostopstate: std_nostopstate_t = std_nostopstate_t;

// Complex template type aliases
pub type duration_typename_common_type_type_parameter_0_0_type__typename_type_parameter_0_1_type = i64;
pub type _Cb_impl = std::ffi::c_void;
pub type _Callback = std::ffi::c_void;
pub type __hash_base_size_t__thread_id = usize;
pub type atomic_make_signed_t___detail___platform_wait_t = i32;
pub type atomic_make_unsigned_t___detail___platform_wait_t = u32;
pub type enable_if___and____not__is_pointer__Dp__is_default_constructible__Dp_value__void = ();
pub type __enable_if_t___is_duration_duration_long__ratio_1__1_value__time_point_system_clock__duration_long__ratio_1__1 = i64;
pub type __enable_if_is_duration_duration_long__ratio_1__1000000000 = i64;
pub type _Digit__Base___Dig = u8;
pub type _Val_int = i32;
