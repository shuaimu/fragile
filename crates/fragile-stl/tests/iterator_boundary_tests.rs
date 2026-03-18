use fragile_stl::*;

struct RbTreeFixture {
    header: *mut __tree_node_base,
    root: *mut __tree_node_base,
    left: *mut __tree_node_base,
    right: *mut __tree_node_base,
}

impl RbTreeFixture {
    unsafe fn alloc_node(is_black: bool) -> *mut __tree_node_base {
        Box::into_raw(Box::new(__tree_node_base {
            __left_: std::ptr::null_mut(),
            __right_: std::ptr::null_mut(),
            __parent_: std::ptr::null_mut(),
            __is_black_: is_black,
        }))
    }

    unsafe fn new_three_node_tree() -> Self {
        let header = Self::alloc_node(false);
        let root = Self::alloc_node(true);
        let left = Self::alloc_node(true);
        let right = Self::alloc_node(true);

        (*header).__parent_ = root as *mut __tree_end_node;
        (*header).__left_ = left;
        (*header).__right_ = right;

        (*root).__parent_ = header as *mut __tree_end_node;
        (*root).__left_ = left;
        (*root).__right_ = right;

        (*left).__parent_ = root as *mut __tree_end_node;
        (*right).__parent_ = root as *mut __tree_end_node;

        Self {
            header,
            root,
            left,
            right,
        }
    }

    fn increment(node: *mut __tree_node_base) -> *mut __tree_node_base {
        _Rb_tree_increment(node as *mut std::ffi::c_void) as *mut __tree_node_base
    }

    fn decrement(node: *mut __tree_node_base) -> *mut __tree_node_base {
        _Rb_tree_decrement(node as *mut std::ffi::c_void) as *mut __tree_node_base
    }
}

impl Drop for RbTreeFixture {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.left));
            drop(Box::from_raw(self.right));
            drop(Box::from_raw(self.root));
            drop(Box::from_raw(self.header));
        }
    }
}

#[test]
fn rb_tree_increment_walks_successors_and_hits_header_boundary() {
    unsafe {
        let fixture = RbTreeFixture::new_three_node_tree();

        assert_eq!(RbTreeFixture::increment(fixture.left), fixture.root);
        assert_eq!(RbTreeFixture::increment(fixture.root), fixture.right);
        assert_eq!(RbTreeFixture::increment(fixture.right), fixture.header);
    }
}

#[test]
fn rb_tree_decrement_walks_predecessors_and_supports_end_decrement() {
    unsafe {
        let fixture = RbTreeFixture::new_three_node_tree();

        assert_eq!(RbTreeFixture::decrement(fixture.header), fixture.right);
        assert_eq!(RbTreeFixture::decrement(fixture.right), fixture.root);
        assert_eq!(RbTreeFixture::decrement(fixture.root), fixture.left);
        assert_eq!(RbTreeFixture::decrement(fixture.left), fixture.header);
    }
}

#[test]
fn rb_tree_increment_and_decrement_null_or_orphan_nodes_return_null() {
    assert!(RbTreeFixture::increment(std::ptr::null_mut()).is_null());
    assert!(RbTreeFixture::decrement(std::ptr::null_mut()).is_null());

    unsafe {
        let orphan = RbTreeFixture::alloc_node(true);
        assert!(RbTreeFixture::increment(orphan).is_null());
        assert!(RbTreeFixture::decrement(orphan).is_null());
        drop(Box::from_raw(orphan));
    }
}
