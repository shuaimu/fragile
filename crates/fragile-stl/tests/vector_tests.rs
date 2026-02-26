use fragile_stl::*;

#[test]
fn vector_new_0_is_empty() {
    let v: std_vector<i32> = std_vector::new_0();
    assert_eq!(v.size(), 0);
    assert_eq!(v.capacity(), 0);
}

#[test]
fn vector_push_back_and_size() {
    let mut v: std_vector<i32> = std_vector::new_0();
    v.push_back(10);
    assert_eq!(v.size(), 1);
    v.push_back(20);
    v.push_back(30);
    assert_eq!(v.size(), 3);
}

#[test]
fn vector_push_back_values_correct() {
    let mut v: std_vector<i32> = std_vector::new_0();
    v.push_back(42);
    v.push_back(99);
    v.push_back(-1);
    // Verify values via iteration
    let values: Vec<i32> = v.into_iter().collect();
    assert_eq!(values, vec![42, 99, -1]);
}

#[test]
fn vector_push_back_triggers_reallocation() {
    let mut v: std_vector<i32> = std_vector::new_0();
    // Initial capacity is 4, push beyond that
    for i in 0..20 {
        v.push_back(i);
    }
    assert_eq!(v.size(), 20);
    assert!(v.capacity() >= 20);
}

#[test]
fn vector_reserve() {
    let mut v: std_vector<i32> = std_vector::new_0();
    v.reserve(100);
    assert!(v.capacity() >= 100);
    assert_eq!(v.size(), 0);
}

#[test]
fn vector_reserve_preserves_data() {
    let mut v: std_vector<i32> = std_vector::new_0();
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);
    v.reserve(100);
    assert_eq!(v.size(), 3);
    let values: Vec<i32> = v.into_iter().collect();
    assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn vector_resize_grow() {
    let mut v: std_vector<i32> = std_vector::new_0();
    v.push_back(1);
    v.push_back(2);
    v.resize(5);
    assert_eq!(v.size(), 5);
    let values: Vec<i32> = v.into_iter().collect();
    assert_eq!(values, vec![1, 2, 0, 0, 0]);
}

#[test]
fn vector_resize_shrink() {
    let mut v: std_vector<i32> = std_vector::new_0();
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);
    v.push_back(4);
    v.resize(2);
    assert_eq!(v.size(), 2);
}

#[test]
fn vector_into_iterator() {
    let mut v: std_vector<i32> = std_vector::new_0();
    v.push_back(10);
    v.push_back(20);
    v.push_back(30);
    let mut sum = 0;
    for val in v {
        sum += val;
    }
    assert_eq!(sum, 60);
}

#[test]
fn vector_default() {
    let v: std_vector<i32> = Default::default();
    assert_eq!(v.size(), 0);
    assert_eq!(v.capacity(), 0);
}

#[test]
fn vector_generic_push_back_from_reference_for_clone_type() {
    let mut v: std_vector<String> = std_vector::new_0();
    let value = String::from("employee");
    v.push_back(&value);
    assert_eq!(v.size(), 1);
    assert_eq!(v.back(), "employee");
}

#[test]
fn vector_int_alias_still_works() {
    let mut v = std_vector_int::new_0();
    v.push_back(7);
    assert_eq!(v.size(), 1);
}
