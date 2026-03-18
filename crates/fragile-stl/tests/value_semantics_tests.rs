use fragile_stl::*;

#[test]
fn optional_int_lifecycle_and_value_or_semantics() {
    let mut opt = std_optional_int::new_0();
    assert!(!opt.has_value());
    assert!(!opt.op_bool());
    assert_eq!(opt.value_or(7), 7);
    assert!(opt.value_ptr().is_null());

    let slot = opt.emplace(42);
    assert!(!slot.is_null());
    assert!(opt.has_value());
    assert_eq!(opt.value_or(0), 42);
    unsafe {
        *slot = 43;
        assert_eq!(*opt.op_deref_const(), 43);
    }

    opt.reset();
    assert!(!opt.has_value());
    assert_eq!(opt.value_or(-1), -1);
    assert!(opt.value_ptr_const().is_null());
}

#[test]
fn optional_int_copy_and_assign_are_value_semantics() {
    let mut lhs = std_optional_int::new_1(10);
    let mut rhs = lhs;
    rhs.assign(11);

    assert_eq!(lhs.value_or(0), 10);
    assert_eq!(rhs.value_or(0), 11);

    let lhs_slot = lhs.value_ptr();
    let rhs_slot = rhs.value_ptr();
    assert_ne!(lhs_slot, rhs_slot);
}

#[test]
fn tuple_int_int_getters_assignment_and_copy() {
    let mut tuple = std_tuple_int__int::new_2(3, 4);
    unsafe {
        *tuple.get_0() = 30;
        *tuple.get_1() = 40;
    }
    assert_eq!(unsafe { *tuple.get_0_const() }, 30);
    assert_eq!(unsafe { *tuple.get_1_const() }, 40);

    tuple.assign(1, 2);
    assert_eq!(unsafe { *tuple.get_0_const() }, 1);
    assert_eq!(unsafe { *tuple.get_1_const() }, 2);

    let copied = tuple;
    assert_eq!(copied, tuple);
}

#[test]
fn variant_int_long_switches_alternatives_and_reports_index() {
    let mut variant = std_variant_int__long::new_0();
    assert_eq!(variant.index(), 0);
    assert!(variant.holds_alternative_int());
    assert!(!variant.holds_alternative_long());

    let int_slot = variant.get_int_ptr();
    assert!(!int_slot.is_null());
    unsafe {
        *int_slot = 12;
    }
    assert_eq!(unsafe { *variant.get_int_ptr_const() }, 12);
    assert!(variant.get_long_ptr().is_null());

    let long_slot = variant.emplace_long(1234);
    assert!(!long_slot.is_null());
    assert_eq!(variant.index(), 1);
    assert!(!variant.holds_alternative_int());
    assert!(variant.holds_alternative_long());
    assert_eq!(unsafe { *variant.get_long_ptr_const() }, 1234);
    assert!(variant.get_int_ptr().is_null());

    variant.emplace_int(99);
    assert_eq!(variant.index(), 0);
    assert_eq!(unsafe { *variant.get_int_ptr_const() }, 99);
}

#[test]
fn variant_int_long_copy_has_independent_storage() {
    let mut lhs = std_variant_int__long::new_1_1(5000);
    let mut rhs = lhs;

    unsafe {
        *rhs.get_long_ptr() = 6000;
    }

    assert_eq!(lhs.index(), 1);
    assert_eq!(rhs.index(), 1);
    assert_eq!(unsafe { *lhs.get_long_ptr_const() }, 5000);
    assert_eq!(unsafe { *rhs.get_long_ptr_const() }, 6000);

    assert_ne!(lhs.get_long_ptr(), rhs.get_long_ptr());
}
