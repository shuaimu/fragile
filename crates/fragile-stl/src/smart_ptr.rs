// Generic std::unique_ptr<T> stub implementation.
#[repr(C)]
pub struct std_unique_ptr<T> {
    _ptr: *mut T,
}

impl<T> Default for std_unique_ptr<T> {
    fn default() -> Self {
        Self {
            _ptr: std::ptr::null_mut(),
        }
    }
}

impl<T> std_unique_ptr<T> {
    pub fn new_0() -> Self {
        Default::default()
    }

    pub fn new_1(ptr: *mut T) -> Self {
        Self { _ptr: ptr }
    }

    pub fn get(&self) -> *mut T {
        self._ptr
    }

    pub fn op_deref(&self) -> &mut T {
        unsafe { &mut *self._ptr }
    }

    pub fn op_arrow(&self) -> *mut T {
        self._ptr
    }

    pub fn release(&mut self) -> *mut T {
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

impl<T> Drop for std_unique_ptr<T> {
    fn drop(&mut self) {
        if !self._ptr.is_null() {
            unsafe { drop(Box::from_raw(self._ptr)); }
        }
    }
}

// Generic std::shared_ptr<T> stub implementation.
#[repr(C)]
pub struct std_shared_ptr<T> {
    _ptr: *mut T,
    _refcount: *mut usize,
}

impl<T> Default for std_shared_ptr<T> {
    fn default() -> Self {
        Self {
            _ptr: std::ptr::null_mut(),
            _refcount: std::ptr::null_mut(),
        }
    }
}

impl<T> std_shared_ptr<T> {
    pub fn new_0() -> Self {
        Default::default()
    }

    pub fn new_1(ptr: *mut T) -> Self {
        let refcount = Box::into_raw(Box::new(1usize));
        Self {
            _ptr: ptr,
            _refcount: refcount,
        }
    }

    pub fn get(&self) -> *mut T {
        self._ptr
    }

    pub fn op_deref(&self) -> &mut T {
        unsafe { &mut *self._ptr }
    }

    pub fn use_count(&self) -> usize {
        if self._refcount.is_null() {
            0
        } else {
            unsafe { *self._refcount }
        }
    }

    pub fn reset(&mut self) {
        if !self._refcount.is_null() {
            unsafe {
                *self._refcount -= 1;
                if *self._refcount == 0 {
                    if !self._ptr.is_null() {
                        drop(Box::from_raw(self._ptr));
                    }
                    drop(Box::from_raw(self._refcount));
                }
            }
        }
        self._ptr = std::ptr::null_mut();
        self._refcount = std::ptr::null_mut();
    }
}

impl<T> Clone for std_shared_ptr<T> {
    fn clone(&self) -> Self {
        if !self._refcount.is_null() {
            unsafe { *self._refcount += 1; }
        }
        Self {
            _ptr: self._ptr,
            _refcount: self._refcount,
        }
    }
}

impl<T> Drop for std_shared_ptr<T> {
    fn drop(&mut self) {
        if !self._refcount.is_null() {
            unsafe {
                *self._refcount -= 1;
                if *self._refcount == 0 {
                    if !self._ptr.is_null() {
                        drop(Box::from_raw(self._ptr));
                    }
                    drop(Box::from_raw(self._refcount));
                }
            }
        }
    }
}

pub type std_unique_ptr_int = std_unique_ptr<i32>;
pub type std_shared_ptr_int = std_shared_ptr<i32>;
