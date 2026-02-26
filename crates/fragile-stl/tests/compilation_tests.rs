/// Compilation tests: verify all types, functions, and traits from the STL stubs
/// are accessible and type-correct when used through the crate's public interface.
use fragile_stl::*;

#[test]
fn file_header_types_compile() {
    let _display = FragileCStrDisplay(std::ptr::null());
    let _opaque = FragileOpaqueField;
    assert_eq!(format!("{}", _display), "(null)");
    assert_eq!(format!("{}", _opaque), "");
}

#[test]
fn file_header_option_is_null() {
    let none: Option<i32> = None;
    let some: Option<i32> = Some(42);
    assert!(none.is_null());
    assert!(!some.is_null());
}

#[test]
fn sse_intrinsics_compile() {
    let a = _mm_set1_epi32(1);
    let b = _mm_set1_epi32(2);
    let _xor = _mm_xor_si128(a, b);
    let _add = _mm_add_epi64(a, b);
    let _srl = _mm_srli_epi64(a, 1);
    let _sll = _mm_slli_epi64(a, 1);
    let _set = _mm_set_epi64x(1, 2);
}

#[test]
fn comparison_types_compile() {
    let ordering = partial_ordering::new_0();
    let _less = PARTIAL_ORDERING_LESS;
    let _equiv = PARTIAL_ORDERING_EQUIVALENT;
    // Operator stubs
    let zero = __cmp_cat___unspec::new_1(0);
    assert!(ordering.op_eq(&zero));
    assert!(!ordering.op_ne(&zero));
}

#[test]
fn hash_functions_compile() {
    let data: [u8; 4] = [1, 2, 3, 4];
    let h1 = _Hash_bytes(data.as_ptr() as *const (), 4, 0);
    let h2 = _Fnv_hash_bytes(data.as_ptr() as *const (), 4, 0);
    assert_eq!(h1, h2); // Both use same implementation
    assert_ne!(h1, 0);

    let h3 = _Hash_impl::hash(data.as_ptr() as *const (), 4, 42);
    assert_ne!(h3, 0);
}

#[test]
fn numeric_limits_compile() {
    assert_eq!(numeric_limits::min_i32_i32(), i32::MIN);
    assert_eq!(numeric_limits::max_i32_i32(), i32::MAX);
    assert_eq!(numeric_limits::min_u32_u32(), u32::MIN);
    assert_eq!(numeric_limits::max_u32_u32(), u32::MAX);
    assert_eq!(numeric_limits::min_i64_i64(), i64::MIN);
    assert_eq!(numeric_limits::max_i64_i64(), i64::MAX);
    assert_eq!(numeric_limits::min_u64_u64(), u64::MIN);
    assert_eq!(numeric_limits::max_u64_u64(), u64::MAX);
}

#[test]
fn exception_types_compile() {
    let e = exception::new_0();
    let msg = e.what();
    assert!(!msg.is_null());
    let _e2 = e.clone();
}

#[test]
fn algorithm_forward_as_tuple() {
    let t = forward_as_tuple(42i32);
    assert_eq!(t._0, 42);
}

#[test]
fn piecewise_construct_exists() {
    let _p: piecewise_construct_t = piecewise_construct;
    // Just verify the static exists and has the right type
}

#[test]
fn math_builtins_compile() {
    assert_eq!(__builtin_fabsl(-3.14), 3.14);
    assert_eq!(__builtin_sqrtl(4.0), 2.0);
    assert!(__builtin_isinf(f64::INFINITY));
    assert!(__builtin_isnan(f64::NAN));
    assert!(!__builtin_isinf(1.0));
    assert!(!__builtin_isnan(1.0));
}

#[test]
fn clib_stubs_compile() {
    // strtol/strtoul are stub implementations (return 0)
    let result = strtol(b"42\0".as_ptr() as *const i8, std::ptr::null_mut(), 10);
    assert_eq!(result, 0); // Stub returns 0

    // Just verify they exist and are callable
    let _ = strtoul(b"100\0".as_ptr() as *const i8, std::ptr::null_mut(), 10);
    let _ = strtoll(b"100\0".as_ptr() as *const i8, std::ptr::null_mut(), 10);
    let _ = strtoull(b"100\0".as_ptr() as *const i8, std::ptr::null_mut(), 10);
}

#[test]
fn error_category_stubs_compile() {
    let gc = generic_category();
    let sc = system_category();
    let ic = iostream_category();
    // Just verify they exist and return valid references
    assert!(!std::ptr::from_ref(gc).is_null());
    assert!(!std::ptr::from_ref(sc).is_null());
    assert!(!std::ptr::from_ref(ic).is_null());
}

#[test]
fn locale_facet_types_compile() {
    let _facet = locale_facet::default();
    let _vtable_ptr: *const locale_facet_vtable = std::ptr::null();
    // Just verify types are accessible
}

#[test]
fn tree_types_compile() {
    let _end = __tree_end_node { __left_: std::ptr::null_mut() };
    let _result = __tree_emplace_result::default();
    assert!(!_result.__second);
}
