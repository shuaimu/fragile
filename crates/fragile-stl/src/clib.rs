// Locale-specific conversion stubs
#[inline] pub fn strtof_l(_s: *const i8, _endptr: *mut *mut i8, _loc: *mut std::ffi::c_void) -> f32 { 0.0 }
#[inline] pub fn strtod_l(_s: *const i8, _endptr: *mut *mut i8, _loc: *mut std::ffi::c_void) -> f64 { 0.0 }
#[inline] pub fn strtold_l(_s: *const i8, _endptr: *mut *mut i8, _loc: *mut std::ffi::c_void) -> f64 { 0.0 }

// Variadic C stdio shims
unsafe extern "C" {
    #[link_name = "vsnprintf"]
    fn __fragile_extern_vsnprintf(_s: *mut i8, _n: u64, _fmt: *const i8, _args: *mut std::ffi::VaList) -> i32;
}
#[inline] pub fn vsnprintf(_s: *mut i8, _n: u64, _fmt: *const i8, mut _args: [std::ffi::VaList; 1]) -> i32 { unsafe { __fragile_extern_vsnprintf(_s, _n, _fmt, _args.as_mut_ptr()) } }
unsafe extern "C" {
    #[link_name = "vasprintf"]
    fn __fragile_extern_vasprintf(_strp: *mut *mut i8, _fmt: *const i8, _args: *mut std::ffi::VaList) -> i32;
}
#[inline] pub fn vasprintf(_strp: *mut *mut i8, _fmt: *const i8, mut _args: [std::ffi::VaList; 1]) -> i32 { unsafe { __fragile_extern_vasprintf(_strp, _fmt, _args.as_mut_ptr()) } }

// sizeof pseudo-function
#[inline] pub fn sizeof___<T>() -> usize { std::mem::size_of::<T>() }

// min/max function variants
#[inline] pub fn min_bool(a: bool, b: bool) -> bool { a && b }
#[inline] pub fn max_f32(a: f32, b: f32) -> f32 { a.max(b) }

// Hypot and lerp variants
#[inline] pub fn __hypot_f32(x: f32, y: f32) -> f32 { x.hypot(y) }
#[inline] pub fn __hypot_f32_3(x: f32, y: f32, z: f32) -> f32 { (x*x + y*y + z*z).sqrt() }
#[inline] pub fn __lerp_f32(a: f32, b: f32, t: f32) -> f32 { a + t * (b - a) }

// Memory search functions
#[inline] pub fn __constexpr_memchr_i8_i8(s: *const i8, c: i8, n: u64) -> *const i8 { unsafe { for i in 0..n as usize { if *s.add(i) == c { return s.add(i); } } std::ptr::null() } }
#[inline] pub fn __constexpr_memchr_u8_u8(s: *const u8, c: u8, n: u64) -> *const u8 { unsafe { for i in 0..n as usize { if *s.add(i) == c { return s.add(i); } } std::ptr::null() } }
#[inline] pub fn __constexpr_strlen_i8(s: *const i8) -> u64 { unsafe { let mut len = 0u64; while *s.add(len as usize) != 0 { len += 1; } len } }
#[inline] pub fn __constexpr_strlen_u8(s: *const u8) -> u64 { unsafe { let mut len = 0u64; while *s.add(len as usize) != 0 { len += 1; } len } }
#[inline] pub fn __constexpr_wmemchr_i32_i32(s: *mut i32, c: i32, n: u64) -> *mut i32 { unsafe { for i in 0..n as usize { let p = s.add(i); if *p == c { return p; } } std::ptr::null_mut() } }
#[inline] pub fn fill_n_char_u64_i8(dest: *mut i8, n: u64, c: i8) -> *mut i8 { unsafe { for i in 0..n as usize { *dest.add(i) = c; } dest.add(n as usize) } }
#[inline] pub fn __find_ptr_mut_u16_ptr_mut_u16_u16(first: *mut u16, last: *mut u16, val: u16) -> *mut u16 { unsafe { let mut p = first; while p != last { if *p == val { return p; } p = p.add(1); } last } }
#[inline] pub fn __find_ptr_mut_u32_ptr_mut_u32_u32(first: *mut u32, last: *mut u32, val: u32) -> *mut u32 { unsafe { let mut p = first; while p != last { if *p == val { return p; } p = p.add(1); } last } }
#[inline] pub fn __find_ptr_mut_u16_ptr_mut_u16_u16_4(first: *mut u16, last: *mut u16, val: u16, _proj: &mut std::ffi::c_void) -> *const u16 { unsafe { let mut p = first; while p != last { if *p == val { return p; } p = p.add(1); } last } }
#[inline] pub fn __find_ptr_mut_u32_ptr_mut_u32_u32_4(first: *mut u32, last: *mut u32, val: u32, _proj: &mut std::ffi::c_void) -> *const u32 { unsafe { let mut p = first; while p != last { if *p == val { return p; } p = p.add(1); } last } }

// Atomic fence functions
#[inline] pub fn __c11_atomic_thread_fence(_order: i32) { std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst); }
#[inline] pub fn __c11_atomic_signal_fence(_order: i32) { std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst); }
#[inline] pub const fn __atomic_always_lock_free(_size: u64, _ptr: *const std::ffi::c_void) -> bool { true }

// Thread and time functions
#[inline] pub fn sched_yield() -> i32 { 0 }
#[repr(C)] #[derive(Default, Clone, Copy)] pub struct timespec { pub tv_sec: i64, pub tv_nsec: i64 }
#[inline] pub fn __convert_to_timespec_chrono_nanoseconds(_ns: i64) -> timespec { timespec { tv_sec: _ns / 1000000000, tv_nsec: _ns % 1000000000 } }
#[inline] pub fn nanosleep(_req: *const timespec, _rem: *mut timespec) -> i32 { 0 }

// Comparison and conversion functions
#[inline] pub fn __lt_impl<T: PartialOrd>(a: T, b: T) -> bool { a < b }
#[inline] pub fn copy_n_char_i32_char(src: *const i8, n: i32, dest: *mut i8) -> *mut i8 { unsafe { std::ptr::copy_nonoverlapping(src, dest, n as usize); dest.add(n as usize) } }
#[inline] pub fn __to_chars_itoa_i8(_val: i8, _buf: *mut i8) -> *mut i8 { _buf }
#[inline] pub fn __width_u128(_val: u128) -> u32 { if _val == 0 { 1 } else { (128 - _val.leading_zeros()) } }
#[inline] pub fn __convert<T, U>(_val: T) -> U where U: Default { Default::default() }
#[inline] pub fn __seed() -> u64 { 0 }

// Format spec constants
pub static __binary_lower_case: u8 = 1;
pub static __binary_upper_case: u8 = 2;
pub static __decimal: u8 = 3;
pub static __octal: u8 = 4;
pub static __hexadecimal_lower_case: u8 = 5;
pub static __hexadecimal_upper_case: u8 = 6;
pub static __string: u8 = 7;
pub static __debug: u8 = 8;
pub static __pointer_lower_case: u8 = 9;
pub static __pointer_upper_case: u8 = 10;
pub static __zero_padding: u8 = 1;
pub static __left: u8 = 1;
pub static __center: u8 = 2;
pub static __right: u8 = 3;
pub static less: i8 = -1;
pub static greater: i8 = 1;

// Unicode grapheme break constants
pub static __SpacingMark: u32 = 10;
pub static __Prepend: u32 = 8;
pub static __Linker: u32 = 16;

// Currency/locale constants
pub static _International: bool = false;

// Power of 10 lookup table
pub static __pow10_128: [u128; 40] = [1, 10, 100, 1000, 10000, 100000, 1000000, 10000000, 100000000, 1000000000, 10000000000, 100000000000, 1000000000000, 10000000000000, 100000000000000, 1000000000000000, 10000000000000000, 100000000000000000, 1000000000000000000, 10000000000000000000, 100000000000000000000, 1000000000000000000000, 10000000000000000000000, 100000000000000000000000, 1000000000000000000000000, 10000000000000000000000000, 100000000000000000000000000, 1000000000000000000000000000, 10000000000000000000000000000, 100000000000000000000000000000, 1000000000000000000000000000000, 10000000000000000000000000000000, 100000000000000000000000000000000, 1000000000000000000000000000000000, 10000000000000000000000000000000000, 100000000000000000000000000000000000, 1000000000000000000000000000000000000, 10000000000000000000000000000000000000, 100000000000000000000000000000000000000, 0];

// C library function stubs
#[inline]
pub fn strtol(_s: *const i8, _endptr: *mut *mut i8, _base: i32) -> i64 {
    // Stub: just return 0 for now
    0
}
#[inline]
pub fn strtoul(_s: *const i8, _endptr: *mut *mut i8, _base: i32) -> u64 { 0 }
#[inline]
pub fn strtoll(_s: *const i8, _endptr: *mut *mut i8, _base: i32) -> i64 { 0 }
#[inline]
pub fn strtoull(_s: *const i8, _endptr: *mut *mut i8, _base: i32) -> u64 { 0 }
#[inline]
pub fn strtof(_s: *const i8, _endptr: *mut *mut i8) -> f32 { 0.0 }
#[inline]
pub fn strtod(_s: *const i8, _endptr: *mut *mut i8) -> f64 { 0.0 }
#[inline]
pub fn strtold(_s: *const i8, _endptr: *mut *mut i8) -> f64 { 0.0 }
#[inline]
pub fn wcstol(_s: *const i32, _endptr: *mut *mut i32, _base: i32) -> i64 { 0 }
#[inline]
pub fn wcstoul(_s: *const i32, _endptr: *mut *mut i32, _base: i32) -> u64 { 0 }
#[inline]
pub fn wcstoll(_s: *const i32, _endptr: *mut *mut i32, _base: i32) -> i64 { 0 }
#[inline]
pub fn wcstoull(_s: *const i32, _endptr: *mut *mut i32, _base: i32) -> u64 { 0 }
#[inline]
pub fn wcstof(_s: *const i32, _endptr: *mut *mut i32) -> f32 { 0.0 }
#[inline]
pub fn wcstod(_s: *const i32, _endptr: *mut *mut i32) -> f64 { 0.0 }
#[inline]
pub fn wcstold(_s: *const i32, _endptr: *mut *mut i32) -> f64 { 0.0 }
#[inline]
pub fn wmemcmp(_s1: *const i32, _s2: *const i32, _n: u64) -> i32 { 0 }
#[inline]
pub fn wcslen(_s: *const i32) -> u64 { 0 }
#[inline]
pub fn wmemcpy(_dest: *mut i32, _src: *const i32, _n: u64) -> *mut i32 { _dest }
#[inline]
pub fn wmemset(_s: *mut i32, _c: i32, _n: u64) -> *mut i32 { _s }
#[inline]
pub fn wmemmove(_dest: *mut i32, _src: *const i32, _n: u64) -> *mut i32 { _dest }
#[inline]
pub fn wcscpy(_dest: *mut i32, _src: *const i32) -> *mut i32 { _dest }
#[inline]
pub fn wcsncpy(_dest: *mut i32, _src: *const i32, _n: u64) -> *mut i32 { _dest }
#[inline]
pub fn wcscmp(_s1: *const i32, _s2: *const i32) -> i32 { 0 }
#[inline]
pub fn wcsncmp(_s1: *const i32, _s2: *const i32, _n: u64) -> i32 { 0 }
#[inline]
pub fn wmemchr(_s: *const i32, _c: i32, _n: u64) -> *const i32 { std::ptr::null() }
#[inline]
pub fn __throw_out_of_range_fmt(_fmt: *const i8, _s: *const i8, _pos: u64, _size: u64) { panic!("out of range"); }
#[inline]
pub fn pthread_mutex_timedlock(_mutex: *mut std::ffi::c_void, _abs_timeout: *const std::ffi::c_void) -> i32 { 0 }
#[inline]
pub fn __stoa_extern__C__fn_ptr_const_i8__ptr_mut_ptr_mut_i8__i32___ret__i64_i8_i8_u64(_f: &dyn Fn(*const i8, *mut *mut i8, i32) -> i64, _name: *const i8, _str: *const i8, _idx: *mut u64, _base: i32) -> i64 { 0 }
#[inline]
pub fn __stoa_extern__C__fn_ptr_const_i8__ptr_mut_ptr_mut_i8__i32___ret__u64_i8_i8_u64(_f: &dyn Fn(*const i8, *mut *mut i8, i32) -> u64, _name: *const i8, _str: *const i8, _idx: *mut u64, _base: i32) -> u64 { 0 }
#[inline]
pub fn __stoa_extern__C__fn_ptr_const_i8__ptr_mut_ptr_mut_i8___ret__f64_i8_i8_u64(_f: &dyn Fn(*const i8, *mut *mut i8) -> f64, _name: *const i8, _str: *const i8, _idx: *mut u64) -> f64 { 0.0 }
#[inline]
pub fn __stoa_extern__C__fn_ptr_const_i8__ptr_mut_ptr_mut_i8___ret__f32_i8_i8_u64(_f: &dyn Fn(*const i8, *mut *mut i8) -> f32, _name: *const i8, _str: *const i8, _idx: *mut u64) -> f32 { 0.0 }
#[inline]
pub fn __stoa_extern__C__fn_ptr_const_i32__ptr_mut_ptr_mut_i32__i32___ret__i64_i8_i32_u64(_f: &dyn Fn(*const i32, *mut *mut i32, i32) -> i64, _name: *const i8, _str: *const i32, _idx: *mut u64, _base: i32) -> i64 { 0 }
#[inline]
pub fn __stoa_extern__C__fn_ptr_const_i32__ptr_mut_ptr_mut_i32__i32___ret__u64_i8_i32_u64(_f: &dyn Fn(*const i32, *mut *mut i32, i32) -> u64, _name: *const i8, _str: *const i32, _idx: *mut u64, _base: i32) -> u64 { 0 }
#[inline]
pub fn __stoa_extern__C__fn_ptr_const_i32__ptr_mut_ptr_mut_i32___ret__f32_i8_i32_u64(_f: &dyn Fn(*const i32, *mut *mut i32) -> f32, _name: *const i8, _str: *const i32, _idx: *mut u64) -> f32 { 0.0 }
#[inline]
pub fn __stoa_extern__C__fn_ptr_const_i32__ptr_mut_ptr_mut_i32___ret__f64_i8_i32_u64(_f: &dyn Fn(*const i32, *mut *mut i32) -> f64, _name: *const i8, _str: *const i32, _idx: *mut u64) -> f64 { 0.0 }
#[inline]
pub fn uncaught_exception() -> bool { false }

// to_string stubs (placeholder implementations)
pub struct __to_string_result { data: [i8; 32], len: usize }
impl __to_string_result {
    pub fn op_basic_string_view(&self) -> *const i8 { self.data.as_ptr() }
}
#[inline]
pub fn to_string(_val: i32) -> __to_string_result { __to_string_result { data: [0; 32], len: 0 } }
#[inline]
pub fn to_string_1(_val: u32) -> __to_string_result { __to_string_result { data: [0; 32], len: 0 } }
#[inline]
pub fn to_string_2(_val: i64) -> __to_string_result { __to_string_result { data: [0; 32], len: 0 } }
#[inline]
pub fn to_string_3(_val: u64) -> __to_string_result { __to_string_result { data: [0; 32], len: 0 } }
#[inline]
pub fn to_string_4(_val: f32) -> __to_string_result { __to_string_result { data: [0; 32], len: 0 } }
#[inline]
pub fn to_string_5(_val: f64) -> __to_string_result { __to_string_result { data: [0; 32], len: 0 } }

// __to_underlying stubs
#[inline]
pub fn __to_underlying_u32(_val: u32) -> u32 { _val }
#[inline]
pub fn __to_underlying_i32(_val: i32) -> i32 { _val }

// glibc internal variable stubs
pub static __libc_single_threaded: i8 = 0;

// Math constants
pub static inf: f64 = f64::INFINITY;
