use fragile_stl::*;

/// Helper: allocate a tree_node_kv on the heap with given key/value.
unsafe fn alloc_node(key: i32, value: i32) -> *mut __tree_node_kv {
    let node = Box::into_raw(Box::new(__tree_node_kv {
        base: __tree_node_base {
            __left_: std::ptr::null_mut(),
            __right_: std::ptr::null_mut(),
            __parent_: std::ptr::null_mut(),
            __is_black_: false,
        },
        first: key,
        second: value,
    }));
    node
}

/// Helper: free a node allocated by alloc_node.
unsafe fn free_node(node: *mut __tree_node_kv) {
    if !node.is_null() {
        // Recursively free children
        let base = &(*node).base;
        if !base.__left_.is_null() {
            free_node(base.__left_ as *mut __tree_node_kv);
        }
        if !base.__right_.is_null() {
            free_node(base.__right_ as *mut __tree_node_kv);
        }
        drop(Box::from_raw(node));
    }
}

/// Helper: count nodes in a tree rooted at `root`.
unsafe fn count_nodes(root: *mut __tree_node_base) -> usize {
    if root.is_null() {
        return 0;
    }
    1 + count_nodes((*root).__left_) + count_nodes((*root).__right_)
}

/// Helper: verify black-height is consistent (returns black-height, or 0 on error).
unsafe fn verify_black_height(node: *mut __tree_node_base) -> usize {
    if node.is_null() {
        return 1; // null nodes count as black
    }
    let left_bh = verify_black_height((*node).__left_);
    let right_bh = verify_black_height((*node).__right_);
    assert_eq!(left_bh, right_bh, "Black-height mismatch");
    left_bh + if (*node).__is_black_ { 1 } else { 0 }
}

#[test]
fn tree_node_layout() {
    // Verify node struct sizes are reasonable
    assert!(std::mem::size_of::<__tree_end_node>() > 0);
    assert!(std::mem::size_of::<__tree_node_base>() > 0);
    assert!(std::mem::size_of::<__tree_node_kv>() >= std::mem::size_of::<__tree_node_base>());
}

#[test]
fn tree_single_insert() {
    unsafe {
        let mut end_node = __tree_end_node {
            __left_: std::ptr::null_mut(),
        };

        let node = alloc_node(10, 100);
        // Set up as root: parent = end_node, end_node.__left_ = root
        (*node).base.__parent_ = &mut end_node as *mut __tree_end_node;
        end_node.__left_ = &mut (*node).base as *mut __tree_node_base;
        (*node).base.__is_black_ = true; // Root is always black

        assert_eq!((*node).first, 10);
        assert_eq!((*node).second, 100);
        assert!((*node).base.__is_black_);
        assert_eq!(count_nodes(end_node.__left_), 1);

        free_node(node);
    }
}

#[test]
fn tree_emplace_result_default() {
    let result = __tree_emplace_result::default();
    assert!(!result.__second);
    assert!(result.first.__ptr_.is_null());
}

#[test]
fn tree_end_node_left_is_accessible() {
    let end_node = __tree_end_node {
        __left_: std::ptr::null_mut(),
    };
    assert!(end_node.__left_.is_null());
}
