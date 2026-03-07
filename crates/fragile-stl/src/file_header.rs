pub struct FragileCStrDisplay(pub *const i8);
impl std::fmt::Display for FragileCStrDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0.is_null() {
            return write!(f, "(null)");
        }
        let cstr = unsafe { std::ffi::CStr::from_ptr(self.0) };
        write!(f, "{}", cstr.to_str().unwrap_or(""))
    }
}
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct FragileOpaqueField;
impl std::fmt::Display for FragileOpaqueField {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}
#[allow(dead_code)]
fn fragile_cstr_substr(p: *const i8, pos: u64, len: u64) -> std::string::String {
    if p.is_null() {
        return std::string::String::new();
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(p) };
    let s = cstr.to_str().unwrap_or("");
    let start = pos as usize;
    if start >= s.len() {
        return std::string::String::new();
    }
    let end = std::cmp::min(start + len as usize, s.len());
    s[start..end].to_string()
}
pub trait FragileOptionIsNull {
    fn is_null(&self) -> bool;
}
impl<T> FragileOptionIsNull for std::option::Option<T> {
    #[inline]
    fn is_null(&self) -> bool {
        self.is_none()
    }
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct FragileVaList {
    _private: [u8; 0],
}
#[inline]
pub fn pthread_setcanceltype(_newtype: i32, _oldtype: *mut i32) -> i32 {
    0
}
// SSE2-compatible vector/intrinsic helpers used by real-world C code (e.g., xxHash)
pub type __fragile_m128i = [u64; 2];
pub type __attribute______vector_size___2_sizeof_long_long_____long_long = __fragile_m128i;
pub type __attribute______vector_size___2_sizeof_long_long_____long_long_const = __fragile_m128i;
#[inline]
pub fn _mm_loadu_si128(
    mem_addr: *const __attribute______vector_size___2_sizeof_long_long_____long_long_const,
) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    unsafe { std::ptr::read_unaligned(mem_addr as *const __fragile_m128i) }
}
#[inline]
pub fn _mm_load_si128(
    mem_addr: *const __attribute______vector_size___2_sizeof_long_long_____long_long_const,
) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    unsafe { std::ptr::read_unaligned(mem_addr as *const __fragile_m128i) }
}
#[inline]
pub fn _mm_xor_si128(
    a: __attribute______vector_size___2_sizeof_long_long_____long_long,
    b: __attribute______vector_size___2_sizeof_long_long_____long_long,
) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    [a[0] ^ b[0], a[1] ^ b[1]]
}
#[inline]
pub fn _mm_mul_epu32(
    a: __attribute______vector_size___2_sizeof_long_long_____long_long,
    b: __attribute______vector_size___2_sizeof_long_long_____long_long,
) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    let lo0 = (a[0] & 0xFFFF_FFFF).wrapping_mul(b[0] & 0xFFFF_FFFF);
    let lo1 = (a[1] & 0xFFFF_FFFF).wrapping_mul(b[1] & 0xFFFF_FFFF);
    [lo0, lo1]
}
#[inline]
pub fn _mm_add_epi64(
    a: __attribute______vector_size___2_sizeof_long_long_____long_long,
    b: __attribute______vector_size___2_sizeof_long_long_____long_long,
) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    [a[0].wrapping_add(b[0]), a[1].wrapping_add(b[1])]
}
#[inline]
pub fn _mm_set1_epi32(
    i: i32,
) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    let v = i as u32 as u64;
    let lane = v | (v << 32);
    [lane, lane]
}
#[inline]
pub fn _mm_srli_epi64(
    a: __attribute______vector_size___2_sizeof_long_long_____long_long,
    imm8: i32,
) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    let sh = if imm8 < 0 { 0 } else { imm8 as u32 };
    if sh >= 64 {
        [0, 0]
    } else {
        [a[0] >> sh, a[1] >> sh]
    }
}
#[inline]
pub fn _mm_slli_epi64(
    a: __attribute______vector_size___2_sizeof_long_long_____long_long,
    imm8: i32,
) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    let sh = if imm8 < 0 { 0 } else { imm8 as u32 };
    if sh >= 64 {
        [0, 0]
    } else {
        [a[0] << sh, a[1] << sh]
    }
}
#[inline]
pub fn _mm_set_epi64x(
    e1: i64,
    e0: i64,
) -> __attribute______vector_size___2_sizeof_long_long_____long_long_const {
    [e0 as u64, e1 as u64]
}

pub type std_ffi_c_void = std::ffi::c_void;

#[repr(C)]
pub struct _IO_marker {
    _private: [u8; 0],
}
#[repr(C)]
pub struct _IO_codecvt {
    _private: [u8; 0],
}
#[repr(C)]
pub struct _IO_wide_data {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub static mut stdin: *mut std::ffi::c_void;
    pub static mut stdout: *mut std::ffi::c_void;
    pub static mut stderr: *mut std::ffi::c_void;
    pub fn strchr(s: *const i8, c: i32) -> *mut i8;
    pub fn strrchr(s: *const i8, c: i32) -> *mut i8;
    #[link_name = "strncmp"]
    fn __fragile_strncmp_ffi(s1: *const i8, s2: *const i8, n: usize) -> i32;
    pub fn strstr(haystack: *const i8, needle: *const i8) -> *mut i8;
    pub fn strerror(errnum: i32) -> *const i8;
    #[link_name = "strlen"]
    fn __fragile_strlen_ffi(s: *const i8) -> usize;
    #[link_name = "sysconf"]
    fn __fragile_sysconf_ffi(name: i32) -> i64;
    #[link_name = "getpid"]
    fn __fragile_getpid_ffi() -> i32;
    #[link_name = "readlink"]
    fn __fragile_readlink_ffi(path: *const i8, buf: *mut i8, bufsz: usize) -> i64;
    #[link_name = "getdelim"]
    fn __fragile_getdelim_ffi(
        lineptr: *mut *mut i8,
        n: *mut u64,
        delim: i32,
        stream: *mut std::ffi::c_void,
    ) -> i64;
    #[link_name = "fcntl"]
    fn __fragile_fcntl_ffi(fd: i32, cmd: i32, arg: i32) -> i32;
    pub fn clock() -> i64;
    pub fn isatty(fd: i32) -> i32;
    pub fn printf(format: *const i8, ...) -> i32;
    pub fn fprintf(stream: *mut std::ffi::c_void, format: *const i8, ...) -> i32;
    pub fn sprintf(buffer: *mut i8, format: *const i8, ...) -> i32;
    pub fn vfprintf(
        stream: *mut std::ffi::c_void,
        format: *const i8,
        ap: [FragileVaList; 1],
    ) -> i32;
    pub fn stat(path: *const i8, buf: *mut std::ffi::c_void) -> i32;
    pub fn __errno_location() -> *mut i32;
    pub fn __assert_fail(
        assertion: *const i8,
        file: *const i8,
        line: u32,
        function: *const i8,
    ) -> !;
}

pub const _SC_NPROCESSORS_ONLN: i32 = 84;

#[inline]
pub fn strlen(s: *const i8) -> i32 {
    unsafe { __fragile_strlen_ffi(s) as i32 }
}

#[inline]
pub fn strncmp(s1: *const i8, s2: *const i8, n: u64) -> i32 {
    unsafe { __fragile_strncmp_ffi(s1, s2, n as usize) }
}

#[inline]
pub fn sysconf(name: i32) -> i32 {
    unsafe { __fragile_sysconf_ffi(name) as i32 }
}

#[inline]
pub fn getpid() -> i32 {
    unsafe { __fragile_getpid_ffi() }
}

#[inline]
pub fn readlink(path: *const i8, buf: *mut i8, bufsz: usize) -> i32 {
    unsafe { __fragile_readlink_ffi(path, buf, bufsz) as i32 }
}

#[inline]
pub fn getdelim(lineptr: *mut *mut i8, n: *mut u64, delim: i8, stream: *mut std::ffi::c_void) -> i64 {
    unsafe { __fragile_getdelim_ffi(lineptr, n, delim as i32, stream) }
}

#[inline]
pub fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32 {
    unsafe { __fragile_fcntl_ffi(fd, cmd, arg) }
}

#[inline]
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[inline]
pub fn rotl32(x: u32, r: i32) -> u32 {
    x.rotate_left((r & 31) as u32)
}

#[inline]
pub fn rotl64(x: u64, r: i32) -> u64 {
    x.rotate_left((r & 63) as u32)
}

#[inline]
pub fn getblock32(p: *const u32, i: i32) -> u32 {
    unsafe { *p.offset(i as isize) }
}

#[inline]
pub fn getblock64(p: *const u64, i: i32) -> u64 {
    unsafe { *p.offset(i as isize) }
}

#[inline]
pub fn fmix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}

#[inline]
pub fn fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51afd7ed558ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
    k ^= k >> 33;
    k
}

#[inline]
pub fn exit(code: i32) -> ! {
    std::process::exit(code)
}
