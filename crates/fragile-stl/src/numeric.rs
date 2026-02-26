// numeric_limits stub for libstdc++ allocator
pub mod numeric_limits {
    #[inline]
    pub fn min() -> isize { isize::MIN }
    #[inline]
    pub fn max() -> isize { isize::MAX }
    #[inline] pub fn min_bool_bool() -> bool { false }
    #[inline] pub fn max_bool_bool() -> bool { true }
    #[inline] pub fn min_i8_i8() -> i8 { i8::MIN }
    #[inline] pub fn max_i8_i8() -> i8 { i8::MAX }
    #[inline] pub fn min_u8_u8() -> u8 { u8::MIN }
    #[inline] pub fn max_u8_u8() -> u8 { u8::MAX }
    #[inline] pub fn min_i16_i16() -> i16 { i16::MIN }
    #[inline] pub fn max_i16_i16() -> i16 { i16::MAX }
    #[inline] pub fn min_u16_u16() -> u16 { u16::MIN }
    #[inline] pub fn max_u16_u16() -> u16 { u16::MAX }
    #[inline] pub fn min_i32_i32() -> i32 { i32::MIN }
    #[inline] pub fn max_i32_i32() -> i32 { i32::MAX }
    #[inline] pub fn min_u32_u32() -> u32 { u32::MIN }
    #[inline] pub fn max_u32_u32() -> u32 { u32::MAX }
    #[inline] pub fn min_i64_i64() -> i64 { i64::MIN }
    #[inline] pub fn max_i64_i64() -> i64 { i64::MAX }
    #[inline] pub fn min_u64_u64() -> u64 { u64::MIN }
    #[inline] pub fn max_u64_u64() -> u64 { u64::MAX }
    #[inline] pub fn min_i128_i128() -> i128 { i128::MIN }
    #[inline] pub fn max_i128_i128() -> i128 { i128::MAX }
    #[inline] pub fn min_u128_u128() -> u128 { u128::MIN }
    #[inline] pub fn max_u128_u128() -> u128 { u128::MAX }
    #[inline] pub fn min___float128___float128() -> f64 { f64::MIN }
    #[inline] pub fn max___float128___float128() -> f64 { f64::MAX }
    #[inline] pub fn _S_1pm16352() -> f64 { 0.0 }
    #[inline] pub fn _S_1p16256() -> f64 { f64::MAX }
    #[inline] pub fn _S_1pm4088() -> f64 { f64::MIN_POSITIVE }
    #[inline] pub fn _S_1p4064() -> f64 { f64::MAX }
    #[inline] pub fn _S_4p() -> f64 { f64::EPSILON }
}

pub mod this_thread {
    #[inline] pub fn sleep_for_chrono_duration_long__ratio_1__1000___(_d: i64) { }
    #[inline] pub fn sleep_for_chrono_duration_long__ratio_1__1000000000___(_d: i64) { }
    #[inline] pub fn r#yield() { std::thread::yield_now() }
}

pub mod chrono {
    #[inline] pub fn duration_cast_duration_long__ratio_1__1000000000___enable_if_is_duration_duration_long__ratio_1__1000000000___enable_if_is_duration_duration_long__ratio_1__1000000000(_d: i64) -> i64 { _d }
    #[inline] pub fn time_point_cast_time_point_system_clock__duration_long__ratio_1__1000000000___enable_if_t___is_duration_duration_long__ratio_1__1_value__time_point_system_clock__duration_long__ratio_1__1___enable_if_t___is_duration_duration_long__ratio_1__1_value__time_point_system_clock__duration_long__ratio_1__1(_t: i64) -> i64 { _t }
}

pub mod literals {
    pub mod chrono_literals {
        #[inline] pub fn op_literal_ms_() -> i64 { 1 }
        #[inline] pub fn op_literal_us_() -> i64 { 1 }
        #[inline] pub fn op_literal_ns_() -> i64 { 1 }
        #[inline] pub fn op_literal_s_() -> i64 { 1 }
        #[inline] pub fn op_literal_min_() -> i64 { 60 }
        #[inline] pub fn op_literal_h_() -> i64 { 3600 }
    }
}
