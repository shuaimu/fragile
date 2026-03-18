use fragile_stl::*;

#[test]
fn ordered_map_default_state_and_clear() {
    let mut map = std_map_int__int::new_0();
    assert!(map.empty());
    assert_eq!(map.size(), 0);

    map.insert_or_assign(3, 30);
    map.insert_or_assign(1, 10);
    assert!(!map.empty());
    assert_eq!(map.size(), 2);

    map.clear();
    assert!(map.empty());
    assert_eq!(map.size(), 0);
}

#[test]
fn ordered_map_op_index_inserts_updates_and_returns_stable_slot() {
    let mut map = std_map_int__int::new_0();

    let slot = map.op_index(7);
    assert!(!slot.is_null());
    unsafe {
        *slot = 70;
    }
    assert_eq!(map.size(), 1);

    let same_slot = map.op_index(7);
    assert_eq!(slot, same_slot);
    unsafe {
        *same_slot = 77;
    }

    let found = map.find(7);
    assert_eq!(found, same_slot);
    unsafe {
        assert_eq!(*found, 77);
    }
}

#[test]
fn ordered_map_maintains_key_order_and_count_semantics() {
    let mut map = std_map_int__int::new_0();
    map.insert_or_assign(30, 3);
    map.insert_or_assign(10, 1);
    map.insert_or_assign(20, 2);

    let keys = map.as_slice().iter().map(|pair| pair.first).collect::<Vec<_>>();
    assert_eq!(keys, vec![10, 20, 30], "ordered map keys should stay sorted");

    assert_eq!(map.count(10), 1);
    assert_eq!(map.count(11), 0);
    assert!(map.find(11).is_null());
}

#[test]
fn ordered_map_erase_reports_removed_count() {
    let mut map = std_map_int__int::new_0();
    map.insert_or_assign(1, 10);
    map.insert_or_assign(2, 20);
    map.insert_or_assign(3, 30);
    assert_eq!(map.size(), 3);

    assert_eq!(map.erase(2), 1);
    assert_eq!(map.size(), 2);
    assert_eq!(map.count(2), 0);
    assert!(map.find(2).is_null());

    assert_eq!(map.erase(2), 0);
    assert_eq!(map.erase(999), 0);
    assert_eq!(map.size(), 2);
}

