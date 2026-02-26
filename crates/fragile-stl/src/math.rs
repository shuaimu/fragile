// Builtin function stubs
#[inline]
pub fn __builtin_addressof<T>(x: &T) -> *const T { x as *const T }
#[inline]
pub fn addressof<T>(x: &T) -> *const T { x as *const T }

// Long double math builtins (using f64 approximation)
#[inline]
pub fn __builtin_huge_vall() -> f64 { f64::INFINITY }
#[inline]
pub fn __builtin_nanl(_s: *const i8) -> f64 { f64::NAN }
#[inline]
pub fn __builtin_nansl(_s: *const i8) -> f64 { f64::NAN }
#[inline]
pub fn __builtin_expl(x: f64) -> f64 { x.exp() }
#[inline]
pub fn __builtin_frexpl(x: f64, exp: *mut i32) -> f64 { unsafe { *exp = 0 }; x }
#[inline]
pub fn __builtin_ldexpl(x: f64, exp: i32) -> f64 { x * (2.0f64).powi(exp) }
#[inline]
pub fn __builtin_exp2l(x: f64) -> f64 { (2.0f64).powf(x) }
#[inline]
pub fn __builtin_expm1l(x: f64) -> f64 { x.exp() - 1.0 }
#[inline]
pub fn __builtin_scalblnl(x: f64, n: i64) -> f64 { x * (2.0f64).powi(n as i32) }
#[inline]
pub fn __builtin_scalbnl(x: f64, n: i32) -> f64 { x * (2.0f64).powi(n) }
#[inline]
pub fn __builtin_powl(x: f64, y: f64) -> f64 { x.powf(y) }
#[inline]
pub fn __builtin_fmaxl(x: f64, y: f64) -> f64 { x.max(y) }
#[inline]
pub fn __builtin_fminl(x: f64, y: f64) -> f64 { x.min(y) }
#[inline]
pub fn __builtin_sqrtl(x: f64) -> f64 { x.sqrt() }
#[inline]
pub fn __builtin_cbrtl(x: f64) -> f64 { x.cbrt() }
#[inline]
pub fn __builtin_hypotl(x: f64, y: f64) -> f64 { x.hypot(y) }
#[inline]
pub fn __builtin_copysignl(x: f64, y: f64) -> f64 { x.copysign(y) }
#[inline]
pub fn __builtin_logl(x: f64) -> f64 { x.ln() }
#[inline]
pub fn __builtin_log2l(x: f64) -> f64 { x.log2() }
#[inline]
pub fn __builtin_log10l(x: f64) -> f64 { x.log10() }
#[inline]
pub fn __builtin_log1pl(x: f64) -> f64 { (1.0 + x).ln() }
#[inline]
pub fn __builtin_fabsl(x: f64) -> f64 { x.abs() }
#[inline]
pub fn __builtin_floorl(x: f64) -> f64 { x.floor() }
#[inline]
pub fn __builtin_ceill(x: f64) -> f64 { x.ceil() }
#[inline]
pub fn __builtin_truncl(x: f64) -> f64 { x.trunc() }
#[inline]
pub fn __builtin_roundl(x: f64) -> f64 { x.round() }
#[inline]
pub fn __builtin_sinl(x: f64) -> f64 { x.sin() }
#[inline]
pub fn __builtin_cosl(x: f64) -> f64 { x.cos() }
#[inline]
pub fn __builtin_tanl(x: f64) -> f64 { x.tan() }
#[inline]
pub fn __builtin_asinl(x: f64) -> f64 { x.asin() }
#[inline]
pub fn __builtin_acosl(x: f64) -> f64 { x.acos() }
#[inline]
pub fn __builtin_atanl(x: f64) -> f64 { x.atan() }
#[inline]
pub fn __builtin_atan2l(y: f64, x: f64) -> f64 { y.atan2(x) }
#[inline]
pub fn __builtin_sinhl(x: f64) -> f64 { x.sinh() }
#[inline]
pub fn __builtin_coshl(x: f64) -> f64 { x.cosh() }
#[inline]
pub fn __builtin_tanhl(x: f64) -> f64 { x.tanh() }
#[inline]
pub fn __builtin_asinhl(x: f64) -> f64 { x.asinh() }
#[inline]
pub fn __builtin_acoshl(x: f64) -> f64 { x.acosh() }
#[inline]
pub fn __builtin_atanhl(x: f64) -> f64 { x.atanh() }
#[inline]
pub fn __builtin_fmodl(x: f64, y: f64) -> f64 { x % y }
#[inline]
pub fn __builtin_remainderl(x: f64, y: f64) -> f64 { x % y }
#[inline]
pub fn __builtin_fmal(x: f64, y: f64, z: f64) -> f64 { x * y + z }
#[inline] pub fn __builtin_erfl(x: f64) -> f64 { x }
#[inline] pub fn __builtin_erfcl(x: f64) -> f64 { 1.0 - x }
#[inline] pub fn __builtin_fdiml(x: f64, y: f64) -> f64 { if x > y { x - y } else { 0.0 } }
#[inline] pub fn __builtin_lgammal(x: f64) -> f64 { if x > 0.0 { x.ln() } else { f64::NAN } }
#[inline] pub fn __builtin_tgammal(x: f64) -> f64 { x }
#[inline] pub fn __builtin_ilogbl(x: f64) -> i32 { if x == 0.0 { i32::MIN } else { x.abs().log2().floor() as i32 } }
#[inline] pub fn __builtin_logbl(x: f64) -> f64 { if x == 0.0 { f64::NEG_INFINITY } else { x.abs().log2().floor() } }
#[inline] pub fn __builtin_modfl(x: f64, iptr: *mut f64) -> f64 { unsafe { if !iptr.is_null() { *iptr = x.trunc(); } } x.fract() }
#[inline] pub fn __builtin_remquol(x: f64, y: f64, quo: *mut i32) -> f64 { unsafe { if !quo.is_null() && y != 0.0 { *quo = (x / y).trunc() as i32; } } x % y }
#[inline] pub fn __builtin_llrintl(x: f64) -> i64 { x.round() as i64 }
#[inline] pub fn __builtin_llroundl(x: f64) -> i64 { x.round() as i64 }
#[inline] pub fn __builtin_lrintl(x: f64) -> i64 { x.round() as i64 }
#[inline] pub fn __builtin_lroundl(x: f64) -> i64 { x.round() as i64 }
#[inline] pub fn __builtin_nearbyintl(x: f64) -> f64 { x.round() }
#[inline] pub fn __builtin_nextafterl(x: f64, y: f64) -> f64 { if x < y { f64::from_bits(x.to_bits().wrapping_add(1)) } else if x > y { f64::from_bits(x.to_bits().wrapping_sub(1)) } else { y } }
#[inline] pub fn __builtin_nexttowardl(x: f64, y: f64) -> f64 { __builtin_nextafterl(x, y) }
#[inline] pub fn __builtin_rintl(x: f64) -> f64 { x.round() }

// Float classification builtins (type-generic: f32 and f64 variants)
pub trait __FloatClassify { fn __is_normal(self) -> bool; fn __is_nan(self) -> bool; fn __is_infinite(self) -> bool; fn __is_finite(self) -> bool; }
impl __FloatClassify for f64 { fn __is_normal(self) -> bool { self.is_normal() } fn __is_nan(self) -> bool { self.is_nan() } fn __is_infinite(self) -> bool { self.is_infinite() } fn __is_finite(self) -> bool { self.is_finite() } }
impl __FloatClassify for f32 { fn __is_normal(self) -> bool { self.is_normal() } fn __is_nan(self) -> bool { self.is_nan() } fn __is_infinite(self) -> bool { self.is_infinite() } fn __is_finite(self) -> bool { self.is_finite() } }
#[inline]
pub fn __builtin_isnormal(x: impl __FloatClassify) -> bool { x.__is_normal() }
#[inline]
pub fn __builtin_isnan(x: impl __FloatClassify) -> bool { x.__is_nan() }
#[inline]
pub fn __builtin_isinf(x: impl __FloatClassify) -> bool { x.__is_infinite() }
#[inline]
pub fn __builtin_isfinite(x: impl __FloatClassify) -> bool { x.__is_finite() }

// f32 (float) builtins
#[inline] pub fn __builtin_huge_valf() -> f32 { f32::INFINITY }
#[inline] pub fn __builtin_nanf(_s: *const i8) -> f32 { f32::NAN }
#[inline] pub fn __builtin_nansf(_s: *const i8) -> f32 { f32::NAN }
#[inline] pub fn __builtin_expf(x: f32) -> f32 { x.exp() }
#[inline] pub fn __builtin_frexpf(x: f32, exp: *mut i32) -> f32 { unsafe { *exp = 0 }; x }
#[inline] pub fn __builtin_ldexpf(x: f32, exp: i32) -> f32 { x * (2.0f32).powi(exp) }
#[inline] pub fn __builtin_exp2f(x: f32) -> f32 { (2.0f32).powf(x) }
#[inline] pub fn __builtin_expm1f(x: f32) -> f32 { x.exp() - 1.0 }
#[inline] pub fn __builtin_scalblnf(x: f32, n: i64) -> f32 { x * (2.0f32).powi(n as i32) }
#[inline] pub fn __builtin_scalbnf(x: f32, n: i32) -> f32 { x * (2.0f32).powi(n) }
#[inline] pub fn __builtin_powf(x: f32, y: f32) -> f32 { x.powf(y) }
#[inline] pub fn __builtin_fmaxf(x: f32, y: f32) -> f32 { x.max(y) }
#[inline] pub fn __builtin_fminf(x: f32, y: f32) -> f32 { x.min(y) }
#[inline] pub fn __builtin_sqrtf(x: f32) -> f32 { x.sqrt() }
#[inline] pub fn __builtin_cbrtf(x: f32) -> f32 { x.cbrt() }
#[inline] pub fn __builtin_hypotf(x: f32, y: f32) -> f32 { x.hypot(y) }
#[inline] pub fn __builtin_copysignf(x: f32, y: f32) -> f32 { x.copysign(y) }
#[inline] pub fn __builtin_logf(x: f32) -> f32 { x.ln() }
#[inline] pub fn __builtin_log2f(x: f32) -> f32 { x.log2() }
#[inline] pub fn __builtin_log10f(x: f32) -> f32 { x.log10() }
#[inline] pub fn __builtin_log1pf(x: f32) -> f32 { (1.0 + x).ln() }
#[inline] pub fn __builtin_fabsf(x: f32) -> f32 { x.abs() }
#[inline] pub fn __builtin_floorf(x: f32) -> f32 { x.floor() }
#[inline] pub fn __builtin_ceilf(x: f32) -> f32 { x.ceil() }
#[inline] pub fn __builtin_truncf(x: f32) -> f32 { x.trunc() }
#[inline] pub fn __builtin_roundf(x: f32) -> f32 { x.round() }
#[inline] pub fn __builtin_sinf(x: f32) -> f32 { x.sin() }
#[inline] pub fn __builtin_cosf(x: f32) -> f32 { x.cos() }
#[inline] pub fn __builtin_tanf(x: f32) -> f32 { x.tan() }
#[inline] pub fn __builtin_asinf(x: f32) -> f32 { x.asin() }
#[inline] pub fn __builtin_acosf(x: f32) -> f32 { x.acos() }
#[inline] pub fn __builtin_atanf(x: f32) -> f32 { x.atan() }
#[inline] pub fn __builtin_atan2f(y: f32, x: f32) -> f32 { y.atan2(x) }
#[inline] pub fn __builtin_sinhf(x: f32) -> f32 { x.sinh() }
#[inline] pub fn __builtin_coshf(x: f32) -> f32 { x.cosh() }
#[inline] pub fn __builtin_tanhf(x: f32) -> f32 { x.tanh() }
#[inline] pub fn __builtin_asinhf(x: f32) -> f32 { x.asinh() }
#[inline] pub fn __builtin_acoshf(x: f32) -> f32 { x.acosh() }
#[inline] pub fn __builtin_atanhf(x: f32) -> f32 { x.atanh() }
#[inline] pub fn __builtin_fmodf(x: f32, y: f32) -> f32 { x % y }
#[inline] pub fn __builtin_remainderf(x: f32, y: f32) -> f32 { x % y }
#[inline] pub fn __builtin_fmaf(x: f32, y: f32, z: f32) -> f32 { x.mul_add(y, z) }
#[inline] pub fn __builtin_erff(x: f32) -> f32 { x }
#[inline] pub fn __builtin_erfcf(x: f32) -> f32 { 1.0 - x }
#[inline] pub fn __builtin_fdimf(x: f32, y: f32) -> f32 { if x > y { x - y } else { 0.0 } }
#[inline] pub fn __builtin_lgammaf(x: f32) -> f32 { if x > 0.0 { x.ln() } else { f32::NAN } }
#[inline] pub fn __builtin_tgammaf(x: f32) -> f32 { x }
#[inline] pub fn __builtin_ilogbf(x: f32) -> i32 { if x == 0.0 { i32::MIN } else { x.abs().log2().floor() as i32 } }
#[inline] pub fn __builtin_logbf(x: f32) -> f32 { if x == 0.0 { f32::NEG_INFINITY } else { x.abs().log2().floor() } }
#[inline] pub fn __builtin_modff(x: f32, iptr: *mut f32) -> f32 { unsafe { if !iptr.is_null() { *iptr = x.trunc(); } } x.fract() }
#[inline] pub fn __builtin_remquof(x: f32, y: f32, quo: *mut i32) -> f32 { unsafe { if !quo.is_null() && y != 0.0 { *quo = (x / y).trunc() as i32; } } x % y }
#[inline] pub fn __builtin_llrintf(x: f32) -> i64 { x.round() as i64 }
#[inline] pub fn __builtin_llroundf(x: f32) -> i64 { x.round() as i64 }
#[inline] pub fn __builtin_lrintf(x: f32) -> i64 { x.round() as i64 }
#[inline] pub fn __builtin_lroundf(x: f32) -> i64 { x.round() as i64 }
#[inline] pub fn __builtin_nearbyintf(x: f32) -> f32 { x.round() }
#[inline] pub fn __builtin_nextafterf(x: f32, y: f32) -> f32 { if x < y { f32::from_bits(x.to_bits().wrapping_add(1)) } else if x > y { f32::from_bits(x.to_bits().wrapping_sub(1)) } else { y } }
#[inline] pub fn __builtin_nexttowardf(x: f32, y: f64) -> f32 { __builtin_nextafterf(x, y as f32) }
#[inline] pub fn __builtin_rintf(x: f32) -> f32 { x.round() }

// f64 (double) builtins
#[inline] pub fn __builtin_huge_val() -> f64 { f64::INFINITY }
#[inline] pub fn __builtin_nan(_s: *const i8) -> f64 { f64::NAN }
#[inline] pub fn __builtin_nans(_s: *const i8) -> f64 { f64::NAN }
#[inline] pub fn __builtin_exp(x: f64) -> f64 { x.exp() }
#[inline] pub fn __builtin_frexp(x: f64, exp: *mut i32) -> f64 { unsafe { *exp = 0 }; x }
#[inline] pub fn __builtin_ldexp(x: f64, exp: i32) -> f64 { x * (2.0f64).powi(exp) }
#[inline] pub fn __builtin_exp2(x: f64) -> f64 { (2.0f64).powf(x) }
#[inline] pub fn __builtin_expm1(x: f64) -> f64 { x.exp() - 1.0 }
#[inline] pub fn __builtin_scalbln(x: f64, n: i64) -> f64 { x * (2.0f64).powi(n as i32) }
#[inline] pub fn __builtin_scalbn(x: f64, n: i32) -> f64 { x * (2.0f64).powi(n) }
#[inline] pub fn __builtin_pow(x: f64, y: f64) -> f64 { x.powf(y) }
#[inline] pub fn __builtin_fmax(x: f64, y: f64) -> f64 { x.max(y) }
#[inline] pub fn __builtin_fmin(x: f64, y: f64) -> f64 { x.min(y) }
#[inline] pub fn __builtin_sqrt(x: f64) -> f64 { x.sqrt() }
#[inline] pub fn __builtin_cbrt(x: f64) -> f64 { x.cbrt() }
#[inline] pub fn __builtin_hypot(x: f64, y: f64) -> f64 { x.hypot(y) }
#[inline] pub fn __builtin_copysign(x: f64, y: f64) -> f64 { x.copysign(y) }
#[inline] pub fn __builtin_log(x: f64) -> f64 { x.ln() }
#[inline] pub fn __builtin_log2(x: f64) -> f64 { x.log2() }
#[inline] pub fn __builtin_log10(x: f64) -> f64 { x.log10() }
#[inline] pub fn __builtin_log1p(x: f64) -> f64 { (1.0 + x).ln() }
#[inline] pub fn __builtin_fabs(x: f64) -> f64 { x.abs() }
#[inline] pub fn __builtin_abs(x: i32) -> i32 { x.abs() }
#[inline] pub fn __builtin_labs(x: i64) -> i64 { x.abs() }
#[inline] pub fn __builtin_llabs(x: i64) -> i64 { x.abs() }
#[inline] pub fn __builtin_floor(x: f64) -> f64 { x.floor() }
#[inline] pub fn __builtin_ceil(x: f64) -> f64 { x.ceil() }
#[inline] pub fn __builtin_trunc(x: f64) -> f64 { x.trunc() }
#[inline] pub fn __builtin_round(x: f64) -> f64 { x.round() }
#[inline] pub fn __builtin_sin(x: f64) -> f64 { x.sin() }
#[inline] pub fn __builtin_cos(x: f64) -> f64 { x.cos() }
#[inline] pub fn __builtin_tan(x: f64) -> f64 { x.tan() }
#[inline] pub fn __builtin_asin(x: f64) -> f64 { x.asin() }
#[inline] pub fn __builtin_acos(x: f64) -> f64 { x.acos() }
#[inline] pub fn __builtin_atan(x: f64) -> f64 { x.atan() }
#[inline] pub fn __builtin_atan2(y: f64, x: f64) -> f64 { y.atan2(x) }
#[inline] pub fn __builtin_sinh(x: f64) -> f64 { x.sinh() }
#[inline] pub fn __builtin_cosh(x: f64) -> f64 { x.cosh() }
#[inline] pub fn __builtin_tanh(x: f64) -> f64 { x.tanh() }
#[inline] pub fn __builtin_asinh(x: f64) -> f64 { x.asinh() }
#[inline] pub fn __builtin_acosh(x: f64) -> f64 { x.acosh() }
#[inline] pub fn __builtin_atanh(x: f64) -> f64 { x.atanh() }
#[inline] pub fn __builtin_fmod(x: f64, y: f64) -> f64 { x % y }
#[inline] pub fn __builtin_remainder(x: f64, y: f64) -> f64 { x % y }
#[inline] pub fn __builtin_fma(x: f64, y: f64, z: f64) -> f64 { x.mul_add(y, z) }

// Wide character builtins
#[inline] pub fn __builtin_wcslen(s: *const i32) -> u64 { unsafe { let mut len = 0u64; while *s.add(len as usize) != 0 { len += 1; } len } }
#[inline] pub fn __builtin_wmemcmp(s1: *const i32, s2: *const i32, n: u64) -> i32 { unsafe { for i in 0..n as usize { let a = *s1.add(i); let b = *s2.add(i); if a != b { return if a < b { -1 } else { 1 }; } } 0 } }
