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

    let all_bits = _mm_set1_epi8(-1);
    assert_eq!(all_bits, [u64::MAX, u64::MAX]);

    let sign_mask = _mm_movemask_epi8(all_bits);
    assert_eq!(sign_mask, 0xFFFF);

    let no_sign = _mm_set1_epi8(0x7f);
    assert_eq!(_mm_movemask_epi8(no_sign), 0);

    let eq = _mm_cmpeq_epi8(no_sign, no_sign);
    assert_eq!(eq, [u64::MAX, u64::MAX]);

    let anded = _mm_and_si128(all_bits, no_sign);
    assert_eq!(anded, no_sign);
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
    // Numeric conversion wrappers should be callable and forward valid inputs.
    let result = strtol(b"42\0".as_ptr() as *const i8, std::ptr::null_mut(), 10);
    assert_eq!(result, 42);

    let as_u64 = strtoul(b"100\0".as_ptr() as *const i8, std::ptr::null_mut(), 10);
    let as_i64 = strtoll(b"100\0".as_ptr() as *const i8, std::ptr::null_mut(), 10);
    let as_u64_wide = strtoull(b"100\0".as_ptr() as *const i8, std::ptr::null_mut(), 10);
    assert_eq!(as_u64, 100);
    assert_eq!(as_i64, 100);
    assert_eq!(as_u64_wide, 100);
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
    let _end = __tree_end_node {
        __left_: std::ptr::null_mut(),
    };
    let _result = __tree_emplace_result::default();
    assert!(!_result.__second);
}

#[test]
fn generic_container_aliases_compile() {
    let mut v_alias: std_vector_int = std_vector_int::new_0();
    v_alias.push_back(1);
    assert_eq!(v_alias.size(), 1);

    let mut v_generic: std_vector<std_string> = std_vector::new_0();
    let empty: std_string = Default::default();
    v_generic.push_back(&empty);
    assert_eq!(v_generic.size(), 1);

    let u_alias: std_unique_ptr_int = std_unique_ptr_int::new_0();
    let s_alias: std_shared_ptr_int = std_shared_ptr_int::new_0();
    assert!(u_alias.get().is_null());
    assert!(s_alias.get().is_null());
}

#[test]
fn generic_container_adapter_aliases_compile() {
    let mut d_alias: std_deque_int = std_deque_int::new_0();
    d_alias.push_back(4);
    d_alias.push_front(3);
    assert_eq!(d_alias.size(), 2);
    assert_eq!(*d_alias.front(), 3);

    let mut q_alias: std_queue_int = std_queue_int::new_0();
    q_alias.push(1);
    q_alias.push(2);
    assert_eq!(q_alias.size(), 2);
    assert_eq!(*q_alias.front(), 1);
    q_alias.pop();
    assert_eq!(*q_alias.front(), 2);

    let mut s_alias: std_stack_int = std_stack_int::new_0();
    s_alias.push(7);
    s_alias.push(9);
    assert_eq!(s_alias.size(), 2);
    assert_eq!(*s_alias.top(), 9);
    s_alias.pop();
    assert_eq!(*s_alias.top(), 7);
}

#[test]
fn generic_containers_accept_ref_values_for_clone_types() {
    #[derive(Clone)]
    struct RefItem(i32);

    let mut v: std_vector<RefItem> = std_vector::new_0();
    let v_item = RefItem(1);
    v.push_back(&v_item);
    assert_eq!(v.size(), 1);
    assert_eq!(v.front().0, 1);

    let mut q: std_queue<RefItem> = std_queue::new_0();
    let q_item = RefItem(2);
    q.push(&q_item);
    assert_eq!(q.size(), 1);
    assert_eq!(q.front().0, 2);

    let mut s: std_stack<RefItem> = std_stack::new_0();
    let s_item = RefItem(3);
    s.push(&s_item);
    assert_eq!(s.size(), 1);
    assert_eq!(s.top().0, 3);
}
