// Red-black tree node types (libc++ ABI compatible)
#[repr(C)]
pub struct __tree_end_node {
    pub __left_: *mut __tree_node_base,
}

#[repr(C)]
pub struct __tree_node_base {
    pub __left_: *mut __tree_node_base,
    pub __right_: *mut __tree_node_base,
    pub __parent_: *mut __tree_end_node,
    pub __is_black_: bool,
}

#[repr(C)]
pub struct __tree_node_kv {
    pub base: __tree_node_base,
    pub first: i32,
    pub second: i32,
}

#[repr(C)]
pub struct __tree_emplace_result {
    pub first: __tree_emplace_iterator,
    pub __second: bool,
}

#[repr(C)]
pub struct __tree_emplace_iterator {
    pub __ptr_: *mut __tree_node_kv,
}

pub struct __tree_node_value_ref {
    pub first: i32,
    pub second: *mut i32,
}

impl Default for __tree_emplace_result {
    fn default() -> Self { Self { first: __tree_emplace_iterator { __ptr_: std::ptr::null_mut() }, __second: false } }
}

impl std::ops::Deref for __tree_emplace_iterator {
    type Target = __tree_node_value_ref;
    fn deref(&self) -> &Self::Target {
        thread_local! {
            static REF: std::cell::UnsafeCell<__tree_node_value_ref> = std::cell::UnsafeCell::new(
                __tree_node_value_ref { first: 0, second: std::ptr::null_mut() }
            );
        }
        REF.with(|cell| unsafe {
            let r = &mut *cell.get();
            r.first = (*self.__ptr_).first;
            r.second = &mut (*self.__ptr_).second as *mut i32;
            &*cell.get()
        })
    }
}

type __tree_node_base_void = __tree_node_base;
type __tree_node_base_void_ = __tree_node_base;
type __tree_end_node_std___tree_node_base_void = __tree_end_node;
type __tree_end_node_std___tree_node_base_void_ = __tree_end_node;
type __tree_end_node___tree_node_base_void = __tree_end_node;
type __tree_node_std___value_type_int__int__void = __tree_node_kv;
type __tree_node_std___value_type_int__int__void_ = __tree_node_kv;
type __tree_node___value_type_int__int__void = __tree_node_kv;

#[inline]
unsafe fn __tree_is_left_child(x: *mut __tree_node_base) -> bool {
    x == (*(*x).__parent_).__left_
}

unsafe fn __tree_left_rotate(x: *mut __tree_node_base) {
    let y = (*x).__right_;
    (*x).__right_ = (*y).__left_;
    if !(*x).__right_.is_null() { (*(*x).__right_).__parent_ = x as *mut __tree_end_node; }
    (*y).__parent_ = (*x).__parent_;
    if __tree_is_left_child(x) {
        (*(*x).__parent_).__left_ = y;
    } else {
        (*((*x).__parent_ as *mut __tree_node_base)).__right_ = y;
    }
    (*y).__left_ = x;
    (*x).__parent_ = y as *mut __tree_end_node;
}

unsafe fn __tree_right_rotate(x: *mut __tree_node_base) {
    let y = (*x).__left_;
    (*x).__left_ = (*y).__right_;
    if !(*x).__left_.is_null() { (*(*x).__left_).__parent_ = x as *mut __tree_end_node; }
    (*y).__parent_ = (*x).__parent_;
    if __tree_is_left_child(x) {
        (*(*x).__parent_).__left_ = y;
    } else {
        (*((*x).__parent_ as *mut __tree_node_base)).__right_ = y;
    }
    (*y).__right_ = x;
    (*x).__parent_ = y as *mut __tree_end_node;
}

unsafe fn __tree_balance_after_insert(root: *mut __tree_node_base, x: *mut __tree_node_base) {
    (*x).__is_black_ = x == root;
    let mut x = x;
    while x != root && !(*((*x).__parent_ as *mut __tree_node_base)).__is_black_ {
        let xp = (*x).__parent_ as *mut __tree_node_base;
        let xpp = (*xp).__parent_ as *mut __tree_node_base;
        if __tree_is_left_child(xp) {
            let y = (*xpp).__right_;
            if !y.is_null() && !(*y).__is_black_ {
                let xp = (*x).__parent_ as *mut __tree_node_base;
                (*xp).__is_black_ = true;
                let xpp = (*xp).__parent_ as *mut __tree_node_base;
                (*xpp).__is_black_ = xpp == root;
                (*y).__is_black_ = true;
                x = xpp;
            } else {
                if !__tree_is_left_child(x) {
                    x = (*x).__parent_ as *mut __tree_node_base;
                    __tree_left_rotate(x);
                }
                let xp = (*x).__parent_ as *mut __tree_node_base;
                (*xp).__is_black_ = true;
                let xpp = (*xp).__parent_ as *mut __tree_node_base;
                (*xpp).__is_black_ = false;
                __tree_right_rotate(xpp);
                break;
            }
        } else {
            let y = (*(*xp).__parent_).__left_;
            if !y.is_null() && !(*y).__is_black_ {
                let xp = (*x).__parent_ as *mut __tree_node_base;
                (*xp).__is_black_ = true;
                let xpp = (*xp).__parent_ as *mut __tree_node_base;
                (*xpp).__is_black_ = xpp == root;
                (*y).__is_black_ = true;
                x = xpp;
            } else {
                if __tree_is_left_child(x) {
                    x = (*x).__parent_ as *mut __tree_node_base;
                    __tree_right_rotate(x);
                }
                let xp = (*x).__parent_ as *mut __tree_node_base;
                (*xp).__is_black_ = true;
                let xpp = (*xp).__parent_ as *mut __tree_node_base;
                (*xpp).__is_black_ = false;
                __tree_left_rotate(xpp);
                break;
            }
        }
    }
}
