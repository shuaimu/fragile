// std::unique_ptr<int> stub implementation
#[repr(C)]
pub struct std_unique_ptr_int {
    _ptr: *mut i32,
}

impl Default for std_unique_ptr_int {
    fn default() -> Self { Self { _ptr: std::ptr::null_mut() } }
}

impl std_unique_ptr_int {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_1(ptr: *mut i32) -> Self { Self { _ptr: ptr } }
    pub fn get(&self) -> *mut i32 { self._ptr }
    pub fn op_deref(&self) -> &mut i32 {
        unsafe { &mut *self._ptr }
    }
    pub fn op_arrow(&self) -> *mut i32 { self._ptr }
    pub fn release(&mut self) -> *mut i32 {
        let ptr = self._ptr;
        self._ptr = std::ptr::null_mut();
        ptr
    }
    pub fn reset(&mut self) {
        if !self._ptr.is_null() {
            unsafe { drop(Box::from_raw(self._ptr)); }
        }
        self._ptr = std::ptr::null_mut();
    }
}

impl Drop for std_unique_ptr_int {
    fn drop(&mut self) {
        if !self._ptr.is_null() {
            unsafe { drop(Box::from_raw(self._ptr)); }
        }
    }
}

// std::shared_ptr<int> stub implementation
#[repr(C)]
pub struct std_shared_ptr_int {
    _ptr: *mut i32,
    _refcount: *mut usize,
}

impl Default for std_shared_ptr_int {
    fn default() -> Self { Self { _ptr: std::ptr::null_mut(), _refcount: std::ptr::null_mut() } }
}

impl std_shared_ptr_int {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_1(ptr: *mut i32) -> Self {
        let refcount = Box::into_raw(Box::new(1usize));
        Self { _ptr: ptr, _refcount: refcount }
    }
    pub fn get(&self) -> *mut i32 { self._ptr }
    pub fn op_deref(&self) -> &mut i32 {
        unsafe { &mut *self._ptr }
    }
    pub fn use_count(&self) -> usize {
        if self._refcount.is_null() { 0 } else { unsafe { *self._refcount } }
    }
    pub fn reset(&mut self) {
        if !self._refcount.is_null() {
            unsafe {
                *self._refcount -= 1;
                if *self._refcount == 0 {
                    if !self._ptr.is_null() { drop(Box::from_raw(self._ptr)); }
                    drop(Box::from_raw(self._refcount));
                }
            }
        }
        self._ptr = std::ptr::null_mut();
        self._refcount = std::ptr::null_mut();
    }
}

impl Clone for std_shared_ptr_int {
    fn clone(&self) -> Self {
        if !self._refcount.is_null() {
            unsafe { *self._refcount += 1; }
        }
        Self { _ptr: self._ptr, _refcount: self._refcount }
    }
}

impl Drop for std_shared_ptr_int {
    fn drop(&mut self) {
        if !self._refcount.is_null() {
            unsafe {
                *self._refcount -= 1;
                if *self._refcount == 0 {
                    if !self._ptr.is_null() { drop(Box::from_raw(self._ptr)); }
                    drop(Box::from_raw(self._refcount));
                }
            }
        }
    }
}
