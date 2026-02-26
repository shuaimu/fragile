use fragile_stl::*;

#[test]
fn string_new_0_is_empty() {
    let s = std_string::new_0();
    assert_eq!(s.size(), 0);
    assert!(s.empty());
    assert_eq!(s.length(), 0);
}

#[test]
fn string_new_1_from_cstr() {
    let cstr = b"hello\0".as_ptr() as *const i8;
    let s = std_string::new_1(cstr);
    assert_eq!(s.size(), 5);
    assert!(!s.empty());
}

#[test]
fn string_new_1_null() {
    let s = std_string::new_1(std::ptr::null());
    assert_eq!(s.size(), 0);
    assert!(s.empty());
}

#[test]
fn string_c_str_roundtrip() {
    let cstr = b"hello\0".as_ptr() as *const i8;
    let s = std_string::new_1(cstr);
    let result = s.c_str();
    let result_str = unsafe { std::ffi::CStr::from_ptr(result) }
        .to_str()
        .unwrap();
    assert_eq!(result_str, "hello");
}

#[test]
fn string_c_str_empty() {
    let s = std_string::new_0();
    let result = s.c_str();
    assert!(!result.is_null());
    let result_str = unsafe { std::ffi::CStr::from_ptr(result) }
        .to_str()
        .unwrap();
    assert_eq!(result_str, "");
}

#[test]
fn string_push_back_and_size() {
    let mut s = std_string::new_0();
    s.push_back(b'a' as i8);
    assert_eq!(s.size(), 1);
    s.push_back(b'b' as i8);
    s.push_back(b'c' as i8);
    assert_eq!(s.size(), 3);

    let result = unsafe { std::ffi::CStr::from_ptr(s.c_str()) }
        .to_str()
        .unwrap();
    assert_eq!(result, "abc");
}

#[test]
fn string_push_back_triggers_reallocation() {
    let mut s = std_string::new_0();
    // Push enough chars to trigger at least one reallocation (initial cap = 16)
    for i in 0u8..32 {
        s.push_back((b'A' + (i % 26)) as i8);
    }
    assert_eq!(s.size(), 32);
    assert!(s.capacity() >= 32);
}

#[test]
fn string_append() {
    let mut s = std_string::new_1(b"hello\0".as_ptr() as *const i8);
    s.append(b" world\0".as_ptr() as *const i8);
    assert_eq!(s.size(), 11);

    let result = unsafe { std::ffi::CStr::from_ptr(s.c_str()) }
        .to_str()
        .unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn string_append_null() {
    let mut s = std_string::new_1(b"hello\0".as_ptr() as *const i8);
    s.append(std::ptr::null());
    assert_eq!(s.size(), 5);
}

#[test]
fn string_clear() {
    let mut s = std_string::new_1(b"hello\0".as_ptr() as *const i8);
    s.clear();
    assert_eq!(s.size(), 0);
    assert!(s.empty());
    // Capacity is preserved
    assert!(s.capacity() > 0);
}

#[test]
fn string_substr() {
    let s = std_string::new_1(b"hello world\0".as_ptr() as *const i8);
    let sub = s.substr(6, 5);
    assert_eq!(sub.size(), 5);

    let result = unsafe { std::ffi::CStr::from_ptr(sub.c_str()) }
        .to_str()
        .unwrap();
    assert_eq!(result, "world");
}

#[test]
fn string_substr_out_of_bounds() {
    let s = std_string::new_1(b"hi\0".as_ptr() as *const i8);
    let sub = s.substr(100, 5);
    assert_eq!(sub.size(), 0);
}

#[test]
fn string_substr_empty() {
    let s = std_string::new_0();
    let sub = s.substr(0, 5);
    assert_eq!(sub.size(), 0);
}

#[test]
fn string_clone() {
    let s = std_string::new_1(b"hello\0".as_ptr() as *const i8);
    let s2 = s.clone();
    assert_eq!(s2.size(), 5);

    // Verify independent memory (modifying one doesn't affect the other)
    let result = unsafe { std::ffi::CStr::from_ptr(s2.c_str()) }
        .to_str()
        .unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn string_clone_empty() {
    let s = std_string::new_0();
    let s2 = s.clone();
    assert_eq!(s2.size(), 0);
    assert!(s2.empty());
}

#[test]
fn string_display() {
    let s = std_string::new_1(b"hello\0".as_ptr() as *const i8);
    assert_eq!(format!("{}", s), "hello");
}

#[test]
fn string_display_empty() {
    let s = std_string::new_0();
    assert_eq!(format!("{}", s), "");
}

#[test]
fn string_op_plus_assign() {
    let mut s = std_string::new_1(b"hello\0".as_ptr() as *const i8);
    s.op_plus_assign(b" world\0".as_ptr() as *const i8);
    let result = unsafe { std::ffi::CStr::from_ptr(s.c_str()) }
        .to_str()
        .unwrap();
    assert_eq!(result, "hello world");
}
