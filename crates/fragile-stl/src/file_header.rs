pub struct FragileCStrDisplay(pub *const i8);
impl std::fmt::Display for FragileCStrDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0.is_null() { return write!(f, "(null)"); }
        let cstr = unsafe { std::ffi::CStr::from_ptr(self.0) };
        write!(f, "{}", cstr.to_str().unwrap_or(""))
    }
}
#[repr(C)] #[derive(Copy, Clone, Default)]
pub struct FragileOpaqueField;
impl std::fmt::Display for FragileOpaqueField {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "") }
}
#[allow(dead_code)]
fn fragile_cstr_substr(p: *const i8, pos: u64, len: u64) -> std::string::String {
    if p.is_null() { return std::string::String::new(); }
    let cstr = unsafe { std::ffi::CStr::from_ptr(p) };
    let s = cstr.to_str().unwrap_or("");
    let start = pos as usize;
    if start >= s.len() { return std::string::String::new(); }
    let end = std::cmp::min(start + len as usize, s.len());
    s[start..end].to_string()
}
pub trait FragileOptionIsNull {
    fn is_null(&self) -> bool;
}
impl<T> FragileOptionIsNull for Option<T> {
    #[inline]
    fn is_null(&self) -> bool { self.is_none() }
}
#[inline]
pub fn pthread_setcanceltype(_newtype: i32, _oldtype: *mut i32) -> i32 { 0 }
// SSE2-compatible vector/intrinsic helpers used by real-world C code (e.g., xxHash)
pub type __fragile_m128i = [u64; 2];
pub type __attribute______vector_size___2_sizeof_long_long_____long_long = __fragile_m128i;
pub type __attribute______vector_size___2_sizeof_long_long_____long_long_const = __fragile_m128i;
#[inline]
pub fn _mm_loadu_si128(mem_addr: *const __attribute______vector_size___2_sizeof_long_long_____long_long_const) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    unsafe { std::ptr::read_unaligned(mem_addr as *const __fragile_m128i) }
}
#[inline]
pub fn _mm_load_si128(mem_addr: *const __attribute______vector_size___2_sizeof_long_long_____long_long_const) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    unsafe { std::ptr::read_unaligned(mem_addr as *const __fragile_m128i) }
}
#[inline]
pub fn _mm_xor_si128(a: __attribute______vector_size___2_sizeof_long_long_____long_long, b: __attribute______vector_size___2_sizeof_long_long_____long_long) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    [a[0] ^ b[0], a[1] ^ b[1]]
}
#[inline]
pub fn _mm_mul_epu32(a: __attribute______vector_size___2_sizeof_long_long_____long_long, b: __attribute______vector_size___2_sizeof_long_long_____long_long) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    let lo0 = (a[0] & 0xFFFF_FFFF).wrapping_mul(b[0] & 0xFFFF_FFFF);
    let lo1 = (a[1] & 0xFFFF_FFFF).wrapping_mul(b[1] & 0xFFFF_FFFF);
    [lo0, lo1]
}
#[inline]
pub fn _mm_add_epi64(a: __attribute______vector_size___2_sizeof_long_long_____long_long, b: __attribute______vector_size___2_sizeof_long_long_____long_long) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    [a[0].wrapping_add(b[0]), a[1].wrapping_add(b[1])]
}
#[inline]
pub fn _mm_set1_epi32(i: i32) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    let v = i as u32 as u64;
    let lane = v | (v << 32);
    [lane, lane]
}
#[inline]
pub fn _mm_srli_epi64(a: __attribute______vector_size___2_sizeof_long_long_____long_long, imm8: i32) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    let sh = if imm8 < 0 { 0 } else { imm8 as u32 };
    if sh >= 64 { [0, 0] } else { [a[0] >> sh, a[1] >> sh] }
}
#[inline]
pub fn _mm_slli_epi64(a: __attribute______vector_size___2_sizeof_long_long_____long_long, imm8: i32) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    let sh = if imm8 < 0 { 0 } else { imm8 as u32 };
    if sh >= 64 { [0, 0] } else { [a[0] << sh, a[1] << sh] }
}
#[inline]
pub fn _mm_set_epi64x(e1: i64, e0: i64) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    [e0 as u64, e1 as u64]
}

pub type std_ffi_c_void = std::ffi::c_void;

#[repr(C)] pub struct _IO_marker { _private: [u8; 0] }
#[repr(C)] pub struct _IO_codecvt { _private: [u8; 0] }
#[repr(C)] pub struct _IO_wide_data { _private: [u8; 0] }

unsafe extern "C" {
    pub static mut stdin: *mut std::ffi::c_void;
    pub static mut stdout: *mut std::ffi::c_void;
    pub static mut stderr: *mut std::ffi::c_void;
    pub fn strchr(s: *const i8, c: i32) -> *mut i8;
    pub fn strrchr(s: *const i8, c: i32) -> *mut i8;
    pub fn strstr(haystack: *const i8, needle: *const i8) -> *mut i8;
    pub fn strerror(errnum: i32) -> *const i8;
    pub fn clock() -> i64;
    pub fn isatty(fd: i32) -> i32;
    pub fn vfprintf(stream: *mut std::ffi::c_void, format: *const i8, ap: [std::ffi::VaList; 1]) -> i32;
    pub fn stat(path: *const i8, buf: *mut std::ffi::c_void) -> i32;
    pub fn __errno_location() -> *mut i32;
    pub fn __assert_fail(assertion: *const i8, file: *const i8, line: u32, function: *const i8) -> !;
}
#[inline]
pub fn exit(code: i32) -> ! { std::process::exit(code) }
