use fragile_stl::*;

// --- unique_ptr tests ---

#[test]
fn unique_ptr_new_0_is_null() {
    let p = std_unique_ptr::<i32>::new_0();
    assert!(p.get().is_null());
}

#[test]
fn unique_ptr_new_1_holds_value() {
    let raw = Box::into_raw(Box::new(42i32));
    let p = std_unique_ptr::<i32>::new_1(raw);
    assert_eq!(p.get(), raw);
    assert_eq!(unsafe { *p.get() }, 42);
}

#[test]
fn unique_ptr_op_deref() {
    let raw = Box::into_raw(Box::new(99i32));
    let p = std_unique_ptr::<i32>::new_1(raw);
    assert_eq!(*p.op_deref(), 99);
}

#[test]
fn unique_ptr_release() {
    let raw = Box::into_raw(Box::new(42i32));
    let mut p = std_unique_ptr::<i32>::new_1(raw);
    let released = p.release();
    assert_eq!(released, raw);
    assert!(p.get().is_null());
    // Clean up manually since release transfers ownership
    unsafe { drop(Box::from_raw(released)); }
}

#[test]
fn unique_ptr_reset() {
    let raw = Box::into_raw(Box::new(42i32));
    let mut p = std_unique_ptr::<i32>::new_1(raw);
    p.reset();
    assert!(p.get().is_null());
    // No double-free — reset deallocated the pointer
}

#[test]
fn unique_ptr_default_is_null() {
    let p: std_unique_ptr<i32> = Default::default();
    assert!(p.get().is_null());
}

#[test]
fn unique_ptr_drop_deallocates() {
    // Just verify no crash/leak (Miri would detect actual leaks)
    let raw = Box::into_raw(Box::new(42i32));
    let _p = std_unique_ptr::<i32>::new_1(raw);
    // drop occurs here — should not crash
}

// --- shared_ptr tests ---

#[test]
fn shared_ptr_new_0_is_null() {
    let p = std_shared_ptr::<i32>::new_0();
    assert!(p.get().is_null());
    assert_eq!(p.use_count(), 0);
}

#[test]
fn shared_ptr_new_1_holds_value() {
    let raw = Box::into_raw(Box::new(42i32));
    let p = std_shared_ptr::<i32>::new_1(raw);
    assert_eq!(unsafe { *p.get() }, 42);
    assert_eq!(p.use_count(), 1);
}

#[test]
fn shared_ptr_clone_increments_refcount() {
    let raw = Box::into_raw(Box::new(42i32));
    let p1 = std_shared_ptr::<i32>::new_1(raw);
    assert_eq!(p1.use_count(), 1);
    let p2 = p1.clone();
    assert_eq!(p1.use_count(), 2);
    assert_eq!(p2.use_count(), 2);
    assert_eq!(p1.get(), p2.get());
}

#[test]
fn shared_ptr_drop_decrements_refcount() {
    let raw = Box::into_raw(Box::new(42i32));
    let p1 = std_shared_ptr::<i32>::new_1(raw);
    let p2 = p1.clone();
    assert_eq!(p1.use_count(), 2);
    drop(p2);
    assert_eq!(p1.use_count(), 1);
}

#[test]
fn shared_ptr_last_drop_deallocates() {
    // Verify no crash on final drop
    let raw = Box::into_raw(Box::new(42i32));
    let p1 = std_shared_ptr::<i32>::new_1(raw);
    let p2 = p1.clone();
    drop(p1);
    drop(p2);
    // No crash = success
}

#[test]
fn shared_ptr_reset() {
    let raw = Box::into_raw(Box::new(42i32));
    let mut p1 = std_shared_ptr::<i32>::new_1(raw);
    let p2 = p1.clone();
    assert_eq!(p1.use_count(), 2);
    p1.reset();
    assert!(p1.get().is_null());
    assert_eq!(p1.use_count(), 0);
    assert_eq!(p2.use_count(), 1);
}

#[test]
fn shared_ptr_op_deref() {
    let raw = Box::into_raw(Box::new(99i32));
    let p = std_shared_ptr::<i32>::new_1(raw);
    assert_eq!(*p.op_deref(), 99);
}

#[test]
fn shared_ptr_default_is_null() {
    let p: std_shared_ptr<i32> = Default::default();
    assert!(p.get().is_null());
    assert_eq!(p.use_count(), 0);
}

#[test]
fn unique_ptr_generic_string_supports_deref() {
    let raw = Box::into_raw(Box::new(String::from("fragile")));
    let p = std_unique_ptr::<String>::new_1(raw);
    assert_eq!(p.op_deref().as_str(), "fragile");
}

#[test]
fn smart_ptr_int_aliases_still_work() {
    let u = std_unique_ptr_int::new_0();
    let s = std_shared_ptr_int::new_0();
    assert!(u.get().is_null());
    assert!(s.get().is_null());
}
