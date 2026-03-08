#[no_mangle]
pub fn rust_add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

#[no_mangle]
pub fn rust_mul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}
