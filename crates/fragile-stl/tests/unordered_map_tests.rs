use fragile_stl::*;

#[test]
fn unordered_map_default_state_and_clear() {
    let mut map = std_unordered_map_int__int::new_0();
    assert!(map.empty());
    assert_eq!(map.size(), 0);
    assert_eq!(map.bucket_count(), 16);

    map.insert_or_assign(1, 10);
    map.insert_or_assign(2, 20);
    assert!(!map.empty());
    assert_eq!(map.size(), 2);

    map.clear();
    assert!(map.empty());
    assert_eq!(map.size(), 0);
    assert_eq!(map.count(1), 0);
    assert!(!map.contains(1));
}

#[test]
fn unordered_map_op_index_insert_update_and_pointer_stability() {
    let mut map = std_unordered_map_int__int::new_0();

    let first = map.op_index(1);
    unsafe {
        *first = 10;
    }
    assert_eq!(map.size(), 1);

    // 1 and 17 collide in the default 16-bucket map.
    let collision = map.op_index(17);
    unsafe {
        *collision = 170;
    }
    assert_eq!(map.size(), 2);

    let first_again = map.op_index(1);
    assert_eq!(first, first_again);
    unsafe {
        *first_again = 11;
    }

    let found = map.find(1);
    assert_eq!(found, first_again);
    unsafe {
        assert_eq!(*found, 11);
    }
}

#[test]
fn unordered_map_lookup_and_count_semantics() {
    let mut map = std_unordered_map_int__int::new_0();
    map.insert_or_assign(3, 30);
    map.insert_or_assign(3, 33);
    map.insert(4, 40);

    assert_eq!(map.size(), 2);
    assert_eq!(map.count(3), 1);
    assert_eq!(map.count(4), 1);
    assert_eq!(map.count(5), 0);
    assert!(map.contains(3));
    assert!(!map.contains(5));

    let found_three = map.find(3);
    assert!(!found_three.is_null());
    unsafe {
        assert_eq!(*found_three, 33);
    }
    assert!(map.find(5).is_null());
    assert!(map.find_const(5).is_null());
}

#[test]
fn unordered_map_erase_and_deterministic_bucket_iteration() {
    let mut map = std_unordered_map_int__int::new_0();
    map.insert_or_assign(18, 180);
    map.insert_or_assign(2, 20);
    map.insert_or_assign(3, 30);

    assert_eq!(map.as_entries(), vec![(18, 180), (2, 20), (3, 30)]);

    assert_eq!(map.erase(2), 1);
    assert_eq!(map.size(), 2);
    assert_eq!(map.count(2), 0);
    assert!(!map.contains(2));
    assert!(map.find(2).is_null());

    assert_eq!(map.erase(2), 0);
    assert_eq!(map.erase(999), 0);
    assert_eq!(map.size(), 2);
    assert_eq!(map.as_entries(), vec![(18, 180), (3, 30)]);
}
