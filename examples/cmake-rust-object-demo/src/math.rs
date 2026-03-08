pub struct RustAccumulator {
    total: i64,
    scale: i64,
}

#[no_mangle]
pub fn rust_accumulator_size() -> usize {
    core::mem::size_of::<RustAccumulator>()
}

#[no_mangle]
pub fn rust_accumulator_align() -> usize {
    core::mem::align_of::<RustAccumulator>()
}

#[no_mangle]
pub fn rust_accumulator_init(ptr: *mut RustAccumulator, seed: i64, scale: i64) -> bool {
    if ptr.is_null() {
        return false;
    }
    unsafe {
        ptr.write(RustAccumulator { total: seed, scale });
    }
    true
}

#[no_mangle]
pub fn rust_accumulator_push(ptr: *mut RustAccumulator, value: i64) -> i64 {
    let Some(acc) = (unsafe { ptr.as_mut() }) else {
        return i64::MIN;
    };
    acc.total = acc.total.wrapping_add(value.wrapping_mul(acc.scale));
    acc.total
}

#[no_mangle]
pub fn rust_accumulator_get(ptr: *const RustAccumulator) -> i64 {
    let Some(acc) = (unsafe { ptr.as_ref() }) else {
        return i64::MIN;
    };
    acc.total
}

#[no_mangle]
pub fn rust_accumulator_drop(ptr: *mut RustAccumulator) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        ptr.drop_in_place();
    }
}
