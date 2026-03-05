// Locale nested class stubs
#[repr(C)]
#[derive(Clone, Copy)]
pub struct locale_facet_vtable {
    pub __type_id: u64,
    pub __base_count: usize,
    pub __base_type_ids: &'static [u64],
    pub __destructor: unsafe fn(*mut locale_facet),
    pub do_out: unsafe fn(*const locale_facet, *mut std::ffi::c_void, *const i8, *const i8, *mut *const i8, *mut i8, *mut i8, *mut *mut i8) -> i32,
    pub do_in: unsafe fn(*const locale_facet, *mut std::ffi::c_void, *const i8, *const i8, *mut *const i8, *mut i8, *mut i8, *mut *mut i8) -> i32,
    pub do_unshift: unsafe fn(*const locale_facet, *mut std::ffi::c_void, *mut i8, *mut i8, *mut *mut i8) -> i32,
    pub do_encoding: unsafe fn(*const locale_facet) -> i32,
    pub do_always_noconv: unsafe fn(*const locale_facet) -> bool,
    pub do_length: unsafe fn(*const locale_facet, *const std::ffi::c_void, *const i8, *const i8, usize) -> isize,
    pub do_max_length: unsafe fn(*const locale_facet) -> isize,
    pub do_decimal_point: unsafe fn(*const locale_facet) -> i32,
    pub do_thousands_sep: unsafe fn(*const locale_facet) -> i32,
    pub do_grouping: unsafe fn(*const locale_facet) -> std_string,
    pub do_truename: unsafe fn(*const locale_facet) -> std_string,
    pub do_falsename: unsafe fn(*const locale_facet) -> std_string,
    pub do_toupper: unsafe fn(*const locale_facet, i32) -> i32,
    pub do_toupper_1: unsafe fn(*const locale_facet, *mut i32, *const i32) -> *const i32,
    pub do_tolower: unsafe fn(*const locale_facet, i32) -> i32,
    pub do_tolower_1: unsafe fn(*const locale_facet, *mut i32, *const i32) -> *const i32,
    pub do_widen: unsafe fn(*const locale_facet, i8) -> i32,
    pub do_widen_1: unsafe fn(*const locale_facet, *const i8, *const i8, *mut i32) -> *const i8,
    pub do_narrow: unsafe fn(*const locale_facet, i32, i8) -> i8,
    pub do_narrow_1: unsafe fn(*const locale_facet, *const i32, *const i32, i8, *mut i8) -> *const i32,
    pub do_is: unsafe fn(*const locale_facet, u32, i32) -> bool,
    pub do_is_1: unsafe fn(*const locale_facet, *const i32, *const i32, *mut u32) -> *const i32,
    pub do_scan_is: unsafe fn(*const locale_facet, u32, *const i32, *const i32) -> *const i32,
    pub do_scan_not: unsafe fn(*const locale_facet, u32, *const i32, *const i32) -> *const i32,
    pub do_compare: unsafe fn(*const locale_facet, *const i32, *const i32, *const i32, *const i32) -> i32,
    pub do_transform: unsafe fn(*const locale_facet, *const i32, *const i32) -> std::ffi::c_void,
}
// Stub functions for locale_facet_vtable Default implementation
unsafe fn __locale_facet_vtable_stub_destructor(_: *mut locale_facet) {}
unsafe fn __locale_facet_vtable_stub_do_out(_: *const locale_facet, _: *mut std::ffi::c_void, _: *const i8, _: *const i8, _: *mut *const i8, _: *mut i8, _: *mut i8, _: *mut *mut i8) -> i32 { 0 }
unsafe fn __locale_facet_vtable_stub_do_in(_: *const locale_facet, _: *mut std::ffi::c_void, _: *const i8, _: *const i8, _: *mut *const i8, _: *mut i8, _: *mut i8, _: *mut *mut i8) -> i32 { 0 }
unsafe fn __locale_facet_vtable_stub_do_unshift(_: *const locale_facet, _: *mut std::ffi::c_void, _: *mut i8, _: *mut i8, _: *mut *mut i8) -> i32 { 0 }
unsafe fn __locale_facet_vtable_stub_do_encoding(_: *const locale_facet) -> i32 { 0 }
unsafe fn __locale_facet_vtable_stub_do_always_noconv(_: *const locale_facet) -> bool { false }
unsafe fn __locale_facet_vtable_stub_do_length(_: *const locale_facet, _: *const std::ffi::c_void, _: *const i8, _: *const i8, _: usize) -> isize { 0 }
unsafe fn __locale_facet_vtable_stub_do_max_length(_: *const locale_facet) -> isize { 0 }
unsafe fn __locale_facet_vtable_stub_do_decimal_point(_: *const locale_facet) -> i32 { 0 }
unsafe fn __locale_facet_vtable_stub_do_thousands_sep(_: *const locale_facet) -> i32 { 0 }
unsafe fn __locale_facet_vtable_stub_do_grouping(_: *const locale_facet) -> std_string { std_string::new_0() }
unsafe fn __locale_facet_vtable_stub_do_truename(_: *const locale_facet) -> std_string { std_string::new_0() }
unsafe fn __locale_facet_vtable_stub_do_falsename(_: *const locale_facet) -> std_string { std_string::new_0() }
unsafe fn __locale_facet_vtable_stub_do_toupper(_: *const locale_facet, c: i32) -> i32 { c }
unsafe fn __locale_facet_vtable_stub_do_toupper_1(_: *const locale_facet, _: *mut i32, e: *const i32) -> *const i32 { e }
unsafe fn __locale_facet_vtable_stub_do_tolower(_: *const locale_facet, c: i32) -> i32 { c }
unsafe fn __locale_facet_vtable_stub_do_tolower_1(_: *const locale_facet, _: *mut i32, e: *const i32) -> *const i32 { e }
unsafe fn __locale_facet_vtable_stub_do_widen(_: *const locale_facet, c: i8) -> i32 { c as i32 }
unsafe fn __locale_facet_vtable_stub_do_widen_1(_: *const locale_facet, _: *const i8, e: *const i8, _: *mut i32) -> *const i8 { e }
unsafe fn __locale_facet_vtable_stub_do_narrow(_: *const locale_facet, _: i32, d: i8) -> i8 { d }
unsafe fn __locale_facet_vtable_stub_do_narrow_1(_: *const locale_facet, _: *const i32, e: *const i32, _: i8, _: *mut i8) -> *const i32 { e }
unsafe fn __locale_facet_vtable_stub_do_is(_: *const locale_facet, _: u32, _: i32) -> bool { false }
unsafe fn __locale_facet_vtable_stub_do_is_1(_: *const locale_facet, _: *const i32, e: *const i32, _: *mut u32) -> *const i32 { e }
unsafe fn __locale_facet_vtable_stub_do_scan_is(_: *const locale_facet, _: u32, _: *const i32, e: *const i32) -> *const i32 { e }
unsafe fn __locale_facet_vtable_stub_do_scan_not(_: *const locale_facet, _: u32, _: *const i32, e: *const i32) -> *const i32 { e }
unsafe fn __locale_facet_vtable_stub_do_compare(_: *const locale_facet, _: *const i32, _: *const i32, _: *const i32, _: *const i32) -> i32 { 0 }
unsafe fn __locale_facet_vtable_stub_do_transform(_: *const locale_facet, _: *const i32, _: *const i32) -> std::ffi::c_void { unsafe { std::mem::zeroed() } }
static __LOCALE_FACET_VTABLE_DEFAULT_BASE_IDS: [u64; 0] = [];
pub static LOCALE_FACET_VTABLE_DEFAULT: locale_facet_vtable = locale_facet_vtable {
    __type_id: 0,
    __base_count: 0,
    __base_type_ids: &__LOCALE_FACET_VTABLE_DEFAULT_BASE_IDS,
    __destructor: __locale_facet_vtable_stub_destructor,
    do_out: __locale_facet_vtable_stub_do_out,
    do_in: __locale_facet_vtable_stub_do_in,
    do_unshift: __locale_facet_vtable_stub_do_unshift,
    do_encoding: __locale_facet_vtable_stub_do_encoding,
    do_always_noconv: __locale_facet_vtable_stub_do_always_noconv,
    do_length: __locale_facet_vtable_stub_do_length,
    do_max_length: __locale_facet_vtable_stub_do_max_length,
    do_decimal_point: __locale_facet_vtable_stub_do_decimal_point,
    do_thousands_sep: __locale_facet_vtable_stub_do_thousands_sep,
    do_grouping: __locale_facet_vtable_stub_do_grouping,
    do_truename: __locale_facet_vtable_stub_do_truename,
    do_falsename: __locale_facet_vtable_stub_do_falsename,
    do_toupper: __locale_facet_vtable_stub_do_toupper,
    do_toupper_1: __locale_facet_vtable_stub_do_toupper_1,
    do_tolower: __locale_facet_vtable_stub_do_tolower,
    do_tolower_1: __locale_facet_vtable_stub_do_tolower_1,
    do_widen: __locale_facet_vtable_stub_do_widen,
    do_widen_1: __locale_facet_vtable_stub_do_widen_1,
    do_narrow: __locale_facet_vtable_stub_do_narrow,
    do_narrow_1: __locale_facet_vtable_stub_do_narrow_1,
    do_is: __locale_facet_vtable_stub_do_is,
    do_is_1: __locale_facet_vtable_stub_do_is_1,
    do_scan_is: __locale_facet_vtable_stub_do_scan_is,
    do_scan_not: __locale_facet_vtable_stub_do_scan_not,
    do_compare: __locale_facet_vtable_stub_do_compare,
    do_transform: __locale_facet_vtable_stub_do_transform,
};
impl Default for locale_facet_vtable {
    fn default() -> Self { LOCALE_FACET_VTABLE_DEFAULT }
}

// Stub vtable constants for codecvt types
pub static STD_CODECVT_CHAR_CHAR_MBSTATE_T__VTABLE: locale_facet_vtable = LOCALE_FACET_VTABLE_DEFAULT;
pub static STD_CODECVT_WCHAR_T_CHAR_MBSTATE_T__VTABLE: locale_facet_vtable = LOCALE_FACET_VTABLE_DEFAULT;
pub static STD_CODECVT_CHAR16_T_CHAR_MBSTATE_T__VTABLE: locale_facet_vtable = LOCALE_FACET_VTABLE_DEFAULT;
pub static STD_CODECVT_CHAR32_T_CHAR_MBSTATE_T__VTABLE: locale_facet_vtable = LOCALE_FACET_VTABLE_DEFAULT;
pub static STD_CODECVT_CHAR16_T_CHAR8_T_MBSTATE_T__VTABLE: locale_facet_vtable = LOCALE_FACET_VTABLE_DEFAULT;
pub static STD_CODECVT_CHAR32_T_CHAR8_T_MBSTATE_T__VTABLE: locale_facet_vtable = LOCALE_FACET_VTABLE_DEFAULT;

// Stub vtable constants for numpunct types
pub static STD_NUMPUNCT_CHAR__VTABLE: locale_facet_vtable = LOCALE_FACET_VTABLE_DEFAULT;
pub static STD_NUMPUNCT_WCHAR_T__VTABLE: locale_facet_vtable = LOCALE_FACET_VTABLE_DEFAULT;
#[repr(C)]
pub struct locale_facet {
    pub __vtable: *const locale_facet_vtable,
    pub __refs_: u32,
}
impl Default for locale_facet {
    fn default() -> Self { Self { __vtable: std::ptr::null(), __refs_: 0 } }
}
impl Clone for locale_facet {
    fn clone(&self) -> Self { Self { __vtable: self.__vtable, __refs_: self.__refs_ } }
}
#[repr(C)]
#[derive(Default, Clone)]
pub struct locale_id { pub _phantom: u8 }

// System type stubs for libc++ threading
pub type __locale_struct = std::ffi::c_void;
pub type locale_t = *mut __locale_struct;
pub type __libcpp_mutex_t = usize;
pub type __libcpp_recursive_mutex_t = usize;
pub type __libcpp_condvar_t = usize;
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct pthread_mutexattr_t { pub kind: i32 }
impl pthread_mutexattr_t { pub fn new_0() -> Self { Default::default() } }
pub type pthread_cond_t = usize;
pub type pthread_once_t = i32;
pub type pthread_key_t = u32;

// C locale functions
pub fn __cloc() -> locale_t { std::ptr::null_mut() }

// Additional pthread functions
pub unsafe fn pthread_once(
    _once_control: *mut pthread_once_t,
    _init_routine: std::option::Option<extern "C" fn()>,
) -> i32 {
    0
}
pub unsafe fn pthread_setspecific(_key: pthread_key_t, _value: *const std::ffi::c_void) -> i32 { 0 }
pub unsafe fn pthread_getspecific(_key: pthread_key_t) -> *mut std::ffi::c_void { std::ptr::null_mut() }
pub unsafe fn pthread_key_create(
    _key: *mut pthread_key_t,
    _destructor: std::option::Option<extern "C" fn(*mut std::ffi::c_void)>,
) -> i32 {
    0
}
pub unsafe fn pthread_key_delete(_key: pthread_key_t) -> i32 { 0 }

// ctype specialization stubs
pub type ctype_char_ = std::ffi::c_void;
pub type ctype_wchar_t_ = std::ffi::c_void;
pub type collate_char_ = std::ffi::c_void;
pub type collate_wchar_t_ = std::ffi::c_void;

// Template placeholder stubs for uninstantiated template types
pub type basic_string__CharT___Traits___Allocator = std::ffi::c_void;
pub type basic_string_view_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;
pub type basic_string_type_parameter_0_0__char_traits_type_parameter_0_0__allocator_type_parameter_0_0 = std::ffi::c_void;
pub type basic_string_type_parameter_0_1__char_traits_type_parameter_0_1__type_parameter_0_2 = std::ffi::c_void;
pub type initializer_list_type_parameter_0_0 = std::ffi::c_void;
pub type initializer_list_pair_const_i32__i32 = std::ffi::c_void;
pub type optional__Tp = std::ffi::c_void;
pub type string_type = std::ffi::c_void;
pub type std_locale = std::ffi::c_void;

// Iterator wrapper type stubs
pub type __wrap_iter_typename_allocator_traits_type_parameter_0_2_const_pointer = std::ffi::c_void;
pub type __wrap_iter_typename_allocator_traits_type_parameter_0_2_pointer = std::ffi::c_void;
pub type reverse_iterator_const_type_parameter_0_0 = std::ffi::c_void;
pub type reverse_iterator_type_parameter_0_0 = std::ffi::c_void;
pub type reverse_iterator___wrap_iter_typename_allocator_traits_type_parameter_0_2_const_pointer = std::ffi::c_void;
pub type reverse_iterator___wrap_iter_typename_allocator_traits_type_parameter_0_2_pointer = std::ffi::c_void;

// Additional template parameter type stubs
pub mod back_insert_iterator_type_parameter_0_0 {
    pub fn new_2<T>(_: i32, _: T) -> std::ffi::c_void { unsafe { std::mem::zeroed() } }
}
pub mod __libcpp_remove_reference_t_exception_ptr__ {
    pub fn new_2<T, U>(_: T, _: U) -> std::ffi::c_void { unsafe { std::mem::zeroed() } }
}
pub mod _HashT {
    #[derive(Default)] pub struct Hasher;
    impl Hasher { pub fn op_call(&self, _: std::ffi::c_void) -> u64 { 0 } }
    pub fn new_0() -> Hasher { Hasher }
}
pub mod std__PairT {
    pub fn new_1<T>(_: T) -> std::ffi::c_void { unsafe { std::mem::zeroed() } }
}

// Extended precision float type stub
pub type __float128 = f64; // stub: 128-bit float, approximated as f64

// Chrono and format type stubs
pub type std___extended_grapheme_custer_property_boundary___property = u32;
pub type std___format_spec___alignment = u32;
pub type _Real = f64;
pub type _Cp = std::ffi::c_void;
pub type _Fp = std::ffi::c_void;  // template type parameter placeholder
pub type _timespec = std::ffi::c_void;

// Unicode grapheme cluster break state types
pub type std___unicode___extended_grapheme_cluster_break___rule = u32;
pub type std___unicode___extended_grapheme_cluster_break___GB9c_indic_conjunct_break_state = u32;
pub type std___unicode___extended_grapheme_cluster_break___GB11_emoji_state = u32;

// Hash function type stubs
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __string_view_hash_char;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __string_view_hash_wchar_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __string_view_hash_char8_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __string_view_hash_char16_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __string_view_hash_char32_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __unary_function_error_code__size_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __unary_function_error_condition__size_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __unary_function_nullptr_t__size_t;
pub type __unique_ptr_deleter_sfinae_type_parameter_0_1 = std::ffi::c_void;

// Grapheme cluster property constants
pub const __none: u32 = 16;
pub const __Extend: u32 = 1;
pub const __Extended_Pictographic: u32 = 2;
pub const __ZWJ: u32 = 3;
pub const __Consonant: u32 = 4;
pub const __V: u32 = 5;
pub const __T: u32 = 6;
pub const __Regional_Indicator: u32 = 7;
pub const __LF: u32 = 8;
pub const __CR: u32 = 9;
pub const __L: u32 = 10;
pub const __LV: u32 = 11;
pub const __LVT: u32 = 12;
pub const __default: u32 = 0;
pub const __GB9c_indic_conjunct_break: u32 = 13;
pub const __GB12_GB13_regional_indicator: u32 = 14;
pub const __GB11_emoji: u32 = 15;

// Format result constants
pub const __consume_result_error: i32 = -1;
pub const __continue_poll: i32 = 0;
pub const __ambiguous: i32 = 1;

// iostream base type stubs
pub type std__Ios_Fmtflags = u32;
pub type std__Ios_Openmode = u32;
pub type std__Ios_Iostate = u32;
pub type std__Ios_Seekdir = i32;
pub type __gthread_mutex_t = usize;
pub type __gthread_time_t = i64;
#[repr(C)] #[derive(Clone, Copy)] pub struct error_category { pub __vtable: *const () }
impl Default for error_category { fn default() -> Self { Self { __vtable: std::ptr::null() } } }
unsafe impl Sync for error_category {}
unsafe impl Send for error_category {}
impl error_category {
    pub fn op_eq(&self, _other: &error_category) -> bool { std::ptr::eq(self, _other) }
    pub fn op____(&self, _other: &error_category) -> bool { !std::ptr::eq(self, _other) }
    pub fn name(&self) -> *const i8 { b"unknown\0".as_ptr() as *const i8 }
    pub fn equivalent(&self, _code: i32, _condition: *const std::ffi::c_void) -> bool { _code == 0 }
    pub fn equivalent_1(&self, _code: *const std::ffi::c_void, _condition: i32) -> bool { _condition == 0 }
    pub fn message(&self, _ev: i32) -> std::ffi::c_void { unsafe { std::mem::zeroed() } }
}
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __ctype_abstract_base_wchar_t_;
pub type _OI = std::ffi::c_void;
pub type _StateT = std::ffi::c_void;
pub type _T1 = std::ffi::c_void;
pub type _T2 = std::ffi::c_void;
pub type ctype_type_parameter_0_0 = std::ffi::c_void;
pub type basic_ostream__CharT___Traits = std::ffi::c_void;
pub type array_std___format___arg_t__sizeof_____Args_ = std::ffi::c_void;

// libstdc++ template placeholders
pub type basic_string__CharT___Traits___Alloc = std::ffi::c_void;
pub type basic_streambuf_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;
pub type basic_ios_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;
pub type __normal_iterator_typename___alloc_traits_type_parameter_0_2__typename_type_parameter_0_2_value_type_const_pointer__basic_string__CharT___Traits___Alloc = std::ffi::c_void;
pub type __normal_iterator_typename___alloc_traits_type_parameter_0_2__typename_type_parameter_0_2_value_type_pointer__basic_string__CharT___Traits___Alloc = std::ffi::c_void;
pub type reverse_iterator___normal_iterator_typename___alloc_traits_type_parameter_0_2__typename_type_parameter_0_2_value_type_const_pointer__basic_string__CharT___Traits___Alloc = std::ffi::c_void;
pub type reverse_iterator___normal_iterator_typename___alloc_traits_type_parameter_0_2__typename_type_parameter_0_2_value_type_pointer__basic_string__CharT___Traits___Alloc = std::ffi::c_void;

// More system type stubs
pub type __gthread_recursive_mutex_t = usize;
pub type __gthread_cond_t = usize;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct _Words { pub _M_pword: *mut (), pub _M_iword: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct _Alloc_hider { pub _M_p: *mut i8 }
pub type pthread_mutex_t = usize;

// Missing template parameter type stubs
pub type std_exception = std::ffi::c_void;
pub type std___format_spec___type = u32;
pub type std___format___arg_t = u32;
pub type std_float_round_style = i32;
pub type std_float_denorm_style = i32;
pub type std_errc = i32;
pub type std_io_errc = i32;
pub type std_type_info = std::ffi::c_void;
pub type std__OrdResult = i32;
pub type std___element_count = u64;
pub type std___variant_detail__Trait = u32;
pub type std_ios_base_seekdir = i32;
pub type std_ios_base = std::ffi::c_void;
pub type std_ios_base_event = i32;
#[repr(C)] #[derive(Clone, Copy)] pub union union__unnamed_union_at__home_shuai_workspace_fragile_vendor_llvm_project_libcxx_include___functional_hash_h_416_5_ { pub __s: union__hash_f64_inner, pub __t: f64 }
#[repr(C)] #[derive(Clone, Copy, Default)] pub struct union__hash_f64_inner { pub __a: u32, pub __b: u32 }
impl Default for union__unnamed_union_at__home_shuai_workspace_fragile_vendor_llvm_project_libcxx_include___functional_hash_h_416_5_ { fn default() -> Self { Self { __s: Default::default() } } }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct fpos_mbstate_t { pub __pos: i64, pub __state_count: i32, pub __state_value: u32 }
pub type fpos___mbstate_t = fpos_mbstate_t;
pub type fpos_t = fpos_mbstate_t;
pub type fpos64_t = fpos_mbstate_t;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct tm {
    pub tm_sec: i32,
    pub tm_min: i32,
    pub tm_hour: i32,
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
    pub tm_gmtoff: i64,
    pub tm_zone: *const i8,
}

#[repr(C)] #[derive(Default, Clone, Copy)] pub struct string_view { pub __data: *const i8, pub __size: u64 }
impl string_view {
    pub fn data(&self) -> *const i8 { self.__data }
    pub fn size(&self) -> u64 { self.__size }
    pub fn length(&self) -> u64 { self.__size }
    pub fn empty(&self) -> bool { self.__size == 0 }
}
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct wstring_view { pub __data: *const i32, pub __size: u64 }
impl wstring_view {
    pub fn data(&self) -> *const i32 { self.__data }
    pub fn size(&self) -> u64 { self.__size }
    pub fn length(&self) -> u64 { self.__size }
    pub fn empty(&self) -> bool { self.__size == 0 }
}
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct allocator_char;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct codecvt_char16_t__char__mbstate_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct codecvt_char32_t__char__mbstate_t;

// Template parameter placeholders
pub type _State = std::ffi::c_void;
pub type _Key = std::ffi::c_void;
pub type _Hash = std::ffi::c_void;
pub type _Pred = std::ffi::c_void;
pub type _Elem = std::ffi::c_void;
pub type _Codecvt = std::ffi::c_void;
pub type __iterator = std::ffi::c_void;
pub type __imp = std::ffi::c_void;
pub type __secret_tag = std::ffi::c_void;
pub type __advance = std::ffi::c_void;
pub type _HashIterator = std::ffi::c_void;
pub type auto = std::ffi::c_void;
pub type __bitset_0__0 = std::ffi::c_void;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __formatter_char_char;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __formatter_char_wchar_t;

// Placeholder and arg bindings
pub type __ph_1 = std::ffi::c_void;
pub type __ph_2 = std::ffi::c_void;
pub type __ph_3 = std::ffi::c_void;
pub type __ph_4 = std::ffi::c_void;
pub type __ph_5 = std::ffi::c_void;
pub type __ph_6 = std::ffi::c_void;
pub type __ph_7 = std::ffi::c_void;
pub type __ph_8 = std::ffi::c_void;
pub type __ph_9 = std::ffi::c_void;
pub type __ph_10 = std::ffi::c_void;
pub type __prev = std::ffi::c_void;
pub type __short = std::ffi::c_void;
pub type __sigset_t = u64;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __scalar_hash_long_double;
pub type __remove_cv_type_parameter_0_0_ = std::ffi::c_void;
pub type __remove_cv_type_parameter_0_1_ = std::ffi::c_void;
pub type std___backoff_results = std::ffi::c_void;
pub type __split_buffer_typename_allocator_traits_type_parameter_0_1_pointer__typename_allocator_traits_type_parameter_0_1_template_rebind_alloc_typename_allocator_traits_type_parameter_0_1_pointer__std___split_buffer_pointer_layout = std::ffi::c_void;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __char_traits_base_wchar_t__wint_t__static_cast_wint_t__4294967295U__;

// More template and locale type stubs
pub type __output_buffer__CharT = std::ffi::c_void;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct numpunct_wchar_t;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct numpunct_char;
pub type __next = std::ffi::c_void;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct mbstate_t { pub __count: i32, pub __value: u32 }
pub type __iter_swap___fn = std::ffi::c_void;
pub type __iter_move___fn = std::ffi::c_void;
pub type _IntT = i64;
pub type __hash_node_type_parameter_0_0__typename_allocator_traits_type_parameter_0_3_void_pointer = std::ffi::c_void;
pub type __hash_node_base_typename_pointer_traits_typename_allocator_traits_type_parameter_0_3_void_pointer_template_rebind___hash_node_type_parameter_0_0__typename_allocator_traits_type_parameter_0_3_void_pointer = std::ffi::c_void;
pub type __handle = std::ffi::c_void;
pub type __dtor_type_parameter_0_0___Traits___destructible_trait = std::ffi::c_void;
pub type __distance = std::ffi::c_void;
pub type __decay_type_parameter_0_0_ = std::ffi::c_void;
pub type __decay_typename___invoke_result_type_parameter_0_2____decay_typename___invoke_result_type_parameter_0_1__type_parameter_0_0_type__type_ = std::ffi::c_void;
pub type __decay_typename___invoke_result_type_parameter_0_1__type_parameter_0_0_type_ = std::ffi::c_void;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __cxx_atomic_impl___cxx_contention_t;
#[repr(C)] #[derive(Clone, Copy)] pub struct ctype_wchar_t { pub __vtable: *const () }
impl Default for ctype_wchar_t { fn default() -> Self { Self { __vtable: std::ptr::null() } } }
unsafe impl Sync for ctype_wchar_t {}
unsafe impl Send for ctype_wchar_t {}
#[repr(C)] #[derive(Clone, Copy)] pub struct ctype_char { pub __vtable: *const () }
impl Default for ctype_char { fn default() -> Self { Self { __vtable: std::ptr::null() } } }
unsafe impl Sync for ctype_char {}
unsafe impl Send for ctype_char {}
pub type __const_reference = std::ffi::c_void;

// ctype vtable stubs
#[repr(C)]
pub struct ctype_char__vtable {
    pub __type_id: u64,
    pub __base_count: usize,
    pub __base_type_ids: &'static [u64],
    pub do_toupper: unsafe fn(*const ctype_char_, i8) -> i8,
    pub do_toupper_1: unsafe fn(*const ctype_char_, *mut i8, *const i8) -> *const i8,
    pub do_tolower: unsafe fn(*const ctype_char_, i8) -> i8,
    pub do_tolower_1: unsafe fn(*const ctype_char_, *mut i8, *const i8) -> *const i8,
    pub __destructor: unsafe fn(*mut ctype_char_),
}
#[repr(C)]
pub struct ctype_wchar_t__vtable {
    pub __type_id: u64,
    pub __base_count: usize,
    pub __base_type_ids: &'static [u64],
    pub do_is: unsafe fn(*const ctype_wchar_t_, u32, i32) -> bool,
    pub do_is_1: unsafe fn(*const ctype_wchar_t_, *const i32, *const i32, *mut u32) -> *const i32,
    pub do_scan_is: unsafe fn(*const ctype_wchar_t_, u32, *const i32, *const i32) -> *const i32,
    pub do_scan_not: unsafe fn(*const ctype_wchar_t_, u32, *const i32, *const i32) -> *const i32,
    pub do_toupper: unsafe fn(*const ctype_wchar_t_, i32) -> i32,
    pub do_toupper_1: unsafe fn(*const ctype_wchar_t_, *mut i32, *const i32) -> *const i32,
    pub do_tolower: unsafe fn(*const ctype_wchar_t_, i32) -> i32,
    pub do_tolower_1: unsafe fn(*const ctype_wchar_t_, *mut i32, *const i32) -> *const i32,
    pub do_widen: unsafe fn(*const ctype_wchar_t_, i8) -> i32,
    pub do_widen_1: unsafe fn(*const ctype_wchar_t_, *const i8, *const i8, *mut i32) -> *const i8,
    pub do_narrow: unsafe fn(*const ctype_wchar_t_, i32, i8) -> i8,
    pub do_narrow_1: unsafe fn(*const ctype_wchar_t_, *const i32, *const i32, i8, *mut i8) -> *const i32,
    pub __destructor: unsafe fn(*mut ctype_wchar_t_),
}

// Atomic types
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct atomic_signed_char { pub __a_: i8 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct atomic_unsigned_char { pub __a_: u8 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct atomic_unsigned_short { pub __a_: u16 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct atomic_unsigned_int { pub __a_: u32 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct atomic_unsigned_long { pub __a_: u64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct atomic_long_long { pub __a_: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct atomic_unsigned_long_long { pub __a_: u64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct atomic___contention_t_or_largest { pub __a_: i64 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct atomic_make_unsigned_t___contention_t_or_largest { pub __a_: u64 }

// Char traits base types
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __char_traits_base_char8_t__unsigned_int__static_cast_unsigned_int___1__;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __char_traits_base_char16_t__uint_least16_t__static_cast_uint_least16_t_65535_;
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct __char_traits_base_char32_t__uint_least32_t__static_cast_uint_least32_t_4294967295U_;

// Locale and collate types
#[repr(C)] #[derive(Clone, Copy)] pub struct collate_char { pub __vtable: *const () }
impl Default for collate_char { fn default() -> Self { Self { __vtable: std::ptr::null() } } }
unsafe impl Sync for collate_char {}
unsafe impl Send for collate_char {}
#[repr(C)] #[derive(Clone, Copy)] pub struct collate_wchar_t { pub __vtable: *const () }
impl Default for collate_wchar_t { fn default() -> Self { Self { __vtable: std::ptr::null() } } }
unsafe impl Sync for collate_wchar_t {}
unsafe impl Send for collate_wchar_t {}

// collate vtable stubs
#[repr(C)]
pub struct collate_char__vtable {
    pub __type_id: u64,
    pub __base_count: usize,
    pub __base_type_ids: &'static [u64],
    pub do_compare: unsafe fn(*const collate_char_, *const i8, *const i8, *const i8, *const i8) -> i32,
    pub do_transform: unsafe fn(*const collate_char_, *const i8, *const i8) -> std::ffi::c_void,
    pub do_hash: unsafe fn(*const collate_char_, *const i8, *const i8) -> i64,
    pub __destructor: unsafe fn(*mut collate_char_),
}
#[repr(C)]
pub struct collate_wchar_t__vtable {
    pub __type_id: u64,
    pub __base_count: usize,
    pub __base_type_ids: &'static [u64],
    pub do_compare: unsafe fn(*const collate_wchar_t_, *const i32, *const i32, *const i32, *const i32) -> i32,
    pub do_transform: unsafe fn(*const collate_wchar_t_, *const i32, *const i32) -> std::ffi::c_void,
    pub do_hash: unsafe fn(*const collate_wchar_t_, *const i32, *const i32) -> i64,
    pub __destructor: unsafe fn(*mut collate_wchar_t_),
}

// Stub vtable constants for locale types
unsafe fn __collate_char_stub_do_compare(_: *const collate_char_, _: *const i8, _: *const i8, _: *const i8, _: *const i8) -> i32 { 0 }
unsafe fn __collate_char_stub_do_transform(_: *const collate_char_, _: *const i8, _: *const i8) -> std::ffi::c_void { std::mem::zeroed() }
unsafe fn __collate_char_stub_do_hash(_: *const collate_char_, _: *const i8, _: *const i8) -> i64 { 0 }
unsafe fn __collate_char_stub_destructor(_: *mut collate_char_) {}
static COLLATE_CHAR_STUB_BASE_IDS: [u64; 1] = [0];
pub static STD_COLLATE_BYNAME_CHAR__VTABLE: collate_char__vtable = collate_char__vtable {
    __type_id: 0, __base_count: 1, __base_type_ids: &COLLATE_CHAR_STUB_BASE_IDS,
    do_compare: __collate_char_stub_do_compare,
    do_transform: __collate_char_stub_do_transform,
    do_hash: __collate_char_stub_do_hash,
    __destructor: __collate_char_stub_destructor,
};

unsafe fn __collate_wchar_t_stub_do_compare(_: *const collate_wchar_t_, _: *const i32, _: *const i32, _: *const i32, _: *const i32) -> i32 { 0 }
unsafe fn __collate_wchar_t_stub_do_transform(_: *const collate_wchar_t_, _: *const i32, _: *const i32) -> std::ffi::c_void { std::mem::zeroed() }
unsafe fn __collate_wchar_t_stub_do_hash(_: *const collate_wchar_t_, _: *const i32, _: *const i32) -> i64 { 0 }
unsafe fn __collate_wchar_t_stub_destructor(_: *mut collate_wchar_t_) {}
static COLLATE_WCHAR_T_STUB_BASE_IDS: [u64; 1] = [0];
pub static STD_COLLATE_BYNAME_WCHAR_T__VTABLE: collate_wchar_t__vtable = collate_wchar_t__vtable {
    __type_id: 0, __base_count: 1, __base_type_ids: &COLLATE_WCHAR_T_STUB_BASE_IDS,
    do_compare: __collate_wchar_t_stub_do_compare,
    do_transform: __collate_wchar_t_stub_do_transform,
    do_hash: __collate_wchar_t_stub_do_hash,
    __destructor: __collate_wchar_t_stub_destructor,
};

unsafe fn __ctype_char_stub_do_toupper(_: *const ctype_char_, c: i8) -> i8 { c }
unsafe fn __ctype_char_stub_do_toupper_1(_: *const ctype_char_, _: *mut i8, e: *const i8) -> *const i8 { e }
unsafe fn __ctype_char_stub_do_tolower(_: *const ctype_char_, c: i8) -> i8 { c }
unsafe fn __ctype_char_stub_do_tolower_1(_: *const ctype_char_, _: *mut i8, e: *const i8) -> *const i8 { e }
unsafe fn __ctype_char_stub_destructor(_: *mut ctype_char_) {}
static CTYPE_CHAR_STUB_BASE_IDS: [u64; 1] = [0];
pub static STD_CTYPE_CHAR__VTABLE: ctype_char__vtable = ctype_char__vtable {
    __type_id: 0, __base_count: 1, __base_type_ids: &CTYPE_CHAR_STUB_BASE_IDS,
    do_toupper: __ctype_char_stub_do_toupper,
    do_toupper_1: __ctype_char_stub_do_toupper_1,
    do_tolower: __ctype_char_stub_do_tolower,
    do_tolower_1: __ctype_char_stub_do_tolower_1,
    __destructor: __ctype_char_stub_destructor,
};
pub static STD_CTYPE_BYNAME_CHAR__VTABLE: ctype_char__vtable = ctype_char__vtable {
    __type_id: 0, __base_count: 1, __base_type_ids: &CTYPE_CHAR_STUB_BASE_IDS,
    do_toupper: __ctype_char_stub_do_toupper,
    do_toupper_1: __ctype_char_stub_do_toupper_1,
    do_tolower: __ctype_char_stub_do_tolower,
    do_tolower_1: __ctype_char_stub_do_tolower_1,
    __destructor: __ctype_char_stub_destructor,
};

unsafe fn __ctype_wchar_t_stub_do_is(_: *const ctype_wchar_t_, _: u32, _: i32) -> bool { false }
unsafe fn __ctype_wchar_t_stub_do_is_1(_: *const ctype_wchar_t_, _: *const i32, e: *const i32, _: *mut u32) -> *const i32 { e }
unsafe fn __ctype_wchar_t_stub_do_scan_is(_: *const ctype_wchar_t_, _: u32, _: *const i32, e: *const i32) -> *const i32 { e }
unsafe fn __ctype_wchar_t_stub_do_scan_not(_: *const ctype_wchar_t_, _: u32, _: *const i32, e: *const i32) -> *const i32 { e }
unsafe fn __ctype_wchar_t_stub_do_toupper(_: *const ctype_wchar_t_, c: i32) -> i32 { c }
unsafe fn __ctype_wchar_t_stub_do_toupper_1(_: *const ctype_wchar_t_, _: *mut i32, e: *const i32) -> *const i32 { e }
unsafe fn __ctype_wchar_t_stub_do_tolower(_: *const ctype_wchar_t_, c: i32) -> i32 { c }
unsafe fn __ctype_wchar_t_stub_do_tolower_1(_: *const ctype_wchar_t_, _: *mut i32, e: *const i32) -> *const i32 { e }
unsafe fn __ctype_wchar_t_stub_do_widen(_: *const ctype_wchar_t_, c: i8) -> i32 { c as i32 }
unsafe fn __ctype_wchar_t_stub_do_widen_1(_: *const ctype_wchar_t_, _: *const i8, e: *const i8, _: *mut i32) -> *const i8 { e }
unsafe fn __ctype_wchar_t_stub_do_narrow(_: *const ctype_wchar_t_, _: i32, d: i8) -> i8 { d }
unsafe fn __ctype_wchar_t_stub_do_narrow_1(_: *const ctype_wchar_t_, _: *const i32, e: *const i32, _: i8, _: *mut i8) -> *const i32 { e }
unsafe fn __ctype_wchar_t_stub_destructor(_: *mut ctype_wchar_t_) {}
static CTYPE_WCHAR_T_STUB_BASE_IDS: [u64; 1] = [0];
pub static STD_CTYPE_WCHAR_T__VTABLE: ctype_wchar_t__vtable = ctype_wchar_t__vtable {
    __type_id: 0, __base_count: 1, __base_type_ids: &CTYPE_WCHAR_T_STUB_BASE_IDS,
    do_is: __ctype_wchar_t_stub_do_is,
    do_is_1: __ctype_wchar_t_stub_do_is_1,
    do_scan_is: __ctype_wchar_t_stub_do_scan_is,
    do_scan_not: __ctype_wchar_t_stub_do_scan_not,
    do_toupper: __ctype_wchar_t_stub_do_toupper,
    do_toupper_1: __ctype_wchar_t_stub_do_toupper_1,
    do_tolower: __ctype_wchar_t_stub_do_tolower,
    do_tolower_1: __ctype_wchar_t_stub_do_tolower_1,
    do_widen: __ctype_wchar_t_stub_do_widen,
    do_widen_1: __ctype_wchar_t_stub_do_widen_1,
    do_narrow: __ctype_wchar_t_stub_do_narrow,
    do_narrow_1: __ctype_wchar_t_stub_do_narrow_1,
    __destructor: __ctype_wchar_t_stub_destructor,
};
pub static STD_CTYPE_BYNAME_WCHAR_T__VTABLE: ctype_wchar_t__vtable = ctype_wchar_t__vtable {
    __type_id: 0, __base_count: 1, __base_type_ids: &CTYPE_WCHAR_T_STUB_BASE_IDS,
    do_is: __ctype_wchar_t_stub_do_is,
    do_is_1: __ctype_wchar_t_stub_do_is_1,
    do_scan_is: __ctype_wchar_t_stub_do_scan_is,
    do_scan_not: __ctype_wchar_t_stub_do_scan_not,
    do_toupper: __ctype_wchar_t_stub_do_toupper,
    do_toupper_1: __ctype_wchar_t_stub_do_toupper_1,
    do_tolower: __ctype_wchar_t_stub_do_tolower,
    do_tolower_1: __ctype_wchar_t_stub_do_tolower_1,
    do_widen: __ctype_wchar_t_stub_do_widen,
    do_widen_1: __ctype_wchar_t_stub_do_widen_1,
    do_narrow: __ctype_wchar_t_stub_do_narrow,
    do_narrow_1: __ctype_wchar_t_stub_do_narrow_1,
    __destructor: __ctype_wchar_t_stub_destructor,
};
