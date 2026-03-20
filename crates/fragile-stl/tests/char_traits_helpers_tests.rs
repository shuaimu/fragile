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
