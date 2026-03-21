use fragile_stl::{__fragile_char_traits_eq_i8, __fragile_char_traits_lt_i8};

#[test]
fn char_traits_eq_i8_accepts_wider_zero_lanes() {
    assert!(__fragile_char_traits_eq_i8(0i8, 0u16));
    assert!(__fragile_char_traits_eq_i8(0i8, 0u32));
    assert!(__fragile_char_traits_eq_i8(0u16, 0u32));
    assert!(!__fragile_char_traits_eq_i8(1i8, 0u16));
    assert!(!__fragile_char_traits_eq_i8(1u16, 0u32));
}

#[test]
fn char_traits_lt_i8_keeps_unsigned_char_ordering() {
    assert!(!__fragile_char_traits_lt_i8(-1i8, 1i8));
    assert!(__fragile_char_traits_lt_i8(1i8, -1i8));
}

#[test]
fn char_traits_lt_i8_accepts_wider_lanes() {
    assert!(__fragile_char_traits_lt_i8(0u16, 1u16));
    assert!(__fragile_char_traits_lt_i8(1u16, 2u32));
    assert!(!__fragile_char_traits_lt_i8(2u32, 1u16));
}

#[test]
fn char_traits_eq_i8_accepts_references() {
    // Transpiled code often passes &i8, &u16, &u32 from &*ptr.add(...)
    let a: i8 = 42;
    let b: i8 = 42;
    assert!(__fragile_char_traits_eq_i8(&a, &b));
    assert!(!__fragile_char_traits_eq_i8(&a, &0i8));
    let c: u16 = 42;
    assert!(__fragile_char_traits_eq_i8(&a, &c)); // cross-type ref comparison
    let d: u32 = 42;
    assert!(__fragile_char_traits_eq_i8(&a, &d));
}

#[test]
fn char_traits_lt_i8_accepts_references() {
    let a: i8 = 1;
    let b: i8 = 2;
    assert!(__fragile_char_traits_lt_i8(&a, &b));
    assert!(!__fragile_char_traits_lt_i8(&b, &a));
    let c: u16 = 100;
    assert!(__fragile_char_traits_lt_i8(&a, &c));
}

#[test]
fn char_traits_eq_i8_handles_void_unit() {
    // Degraded/placeholder code passes () (void) — should not panic
    assert!(__fragile_char_traits_eq_i8((), ()));
    assert!(__fragile_char_traits_eq_i8(&(), &()));
    assert!(!__fragile_char_traits_eq_i8(1i8, ()));
    assert!(__fragile_char_traits_eq_i8(0i8, ()));
}

#[test]
fn char_traits_lt_i8_handles_void_unit() {
    assert!(!__fragile_char_traits_lt_i8((), ()));
    assert!(!__fragile_char_traits_lt_i8(&(), &()));
    assert!(__fragile_char_traits_lt_i8((), 1i8)); // 0 < 1
    assert!(!__fragile_char_traits_lt_i8(1i8, ())); // 1 < 0 = false
}

#[test]
fn char_traits_mixed_ref_and_value() {
    // One arg by reference, other by value
    let x: u8 = 5;
    assert!(__fragile_char_traits_eq_i8(&x, 5u8));
    assert!(__fragile_char_traits_lt_i8(3i8, &x));
}
