use fragile_stl::*;

#[test]
fn sort_ascending() {
    let mut data = vec![5, 3, 1, 4, 2];
    let first = data.as_mut_ptr();
    let last = unsafe { first.add(data.len()) };
    std_sort_int(first, last);
    assert_eq!(data, vec![1, 2, 3, 4, 5]);
}

#[test]
fn sort_already_sorted() {
    let mut data = vec![1, 2, 3, 4, 5];
    let first = data.as_mut_ptr();
    let last = unsafe { first.add(data.len()) };
    std_sort_int(first, last);
    assert_eq!(data, vec![1, 2, 3, 4, 5]);
}

#[test]
fn sort_single_element() {
    let mut data = vec![42];
    let first = data.as_mut_ptr();
    let last = unsafe { first.add(1) };
    std_sort_int(first, last);
    assert_eq!(data, vec![42]);
}

#[test]
fn sort_null_ptrs() {
    std_sort_int(std::ptr::null_mut(), std::ptr::null_mut());
    // No crash = success
}

#[test]
fn find_present() {
    let data = vec![10, 20, 30, 40, 50];
    let first = data.as_ptr();
    let last = unsafe { first.add(data.len()) };
    let result = std_find_int(first, last, 30);
    assert_eq!(result, unsafe { first.add(2) });
}

#[test]
fn find_not_present() {
    let data = vec![10, 20, 30];
    let first = data.as_ptr();
    let last = unsafe { first.add(data.len()) };
    let result = std_find_int(first, last, 99);
    assert_eq!(result, last);
}

#[test]
fn find_first_element() {
    let data = vec![42, 1, 2];
    let first = data.as_ptr();
    let last = unsafe { first.add(data.len()) };
    let result = std_find_int(first, last, 42);
    assert_eq!(result, first);
}

#[test]
fn count_occurrences() {
    let data = vec![1, 2, 3, 2, 1, 2];
    let first = data.as_ptr();
    let last = unsafe { first.add(data.len()) };
    assert_eq!(std_count_int(first, last, 2), 3);
    assert_eq!(std_count_int(first, last, 1), 2);
    assert_eq!(std_count_int(first, last, 99), 0);
}

#[test]
fn fill_range() {
    let mut data = vec![0, 0, 0, 0, 0];
    let first = data.as_mut_ptr();
    let last = unsafe { first.add(data.len()) };
    std_fill_int(first, last, 42);
    assert_eq!(data, vec![42, 42, 42, 42, 42]);
}

#[test]
fn reverse_range() {
    let mut data = vec![1, 2, 3, 4, 5];
    let first = data.as_mut_ptr();
    let last = unsafe { first.add(data.len()) };
    std_reverse_int(first, last);
    assert_eq!(data, vec![5, 4, 3, 2, 1]);
}

#[test]
fn copy_range() {
    let src = vec![1, 2, 3, 4];
    let mut dest = vec![0i32; 4];
    let first = src.as_ptr();
    let last = unsafe { first.add(src.len()) };
    let result = std_copy_int(first, last, dest.as_mut_ptr());
    assert_eq!(dest, vec![1, 2, 3, 4]);
    assert_eq!(result, unsafe { dest.as_mut_ptr().add(4) });
}
