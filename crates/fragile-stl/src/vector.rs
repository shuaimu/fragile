// std::vector<int> instantiation stub
#[repr(C)]
#[derive(Default)]
pub struct std_vector_int {
    _data: *mut i32,
    _size: usize,
    _capacity: usize,
}

impl std_vector_int {
    pub fn new_0() -> Self { Self { _data: std::ptr::null_mut(), _size: 0, _capacity: 0 } }
    pub fn push_back(&mut self, val: i32) {
        if self._size >= self._capacity {
            let new_cap = if self._capacity == 0 { 4 } else { self._capacity * 2 };
            let new_layout = std::alloc::Layout::array::<i32>(new_cap).unwrap();
            let new_data = unsafe { std::alloc::alloc(new_layout) as *mut i32 };
            if !self._data.is_null() {
                unsafe { std::ptr::copy_nonoverlapping(self._data, new_data, self._size); }
                let old_layout = std::alloc::Layout::array::<i32>(self._capacity).unwrap();
                unsafe { std::alloc::dealloc(self._data as *mut u8, old_layout); }
            }
            self._data = new_data;
            self._capacity = new_cap;
        }
        unsafe { *self._data.add(self._size) = val; }
        self._size += 1;
    }
    pub fn size(&self) -> usize { self._size }
    pub fn capacity(&self) -> usize { self._capacity }
    pub fn reserve(&mut self, new_cap: i32) {
    let new_cap = new_cap as usize;
        if new_cap > self._capacity {
            let new_layout = std::alloc::Layout::array::<i32>(new_cap).unwrap();
            let new_data = unsafe { std::alloc::alloc(new_layout) as *mut i32 };
            if !self._data.is_null() && self._size > 0 {
                unsafe { std::ptr::copy_nonoverlapping(self._data, new_data, self._size); }
                let old_layout = std::alloc::Layout::array::<i32>(self._capacity).unwrap();
                unsafe { std::alloc::dealloc(self._data as *mut u8, old_layout); }
            }
            self._data = new_data;
            self._capacity = new_cap;
        }
    }
    pub fn resize(&mut self, new_size: i32) {
    let new_size = new_size as usize;
        if new_size > self._capacity {
            self.reserve(new_size as i32);
        }
        while self._size < new_size {
            unsafe { *self._data.add(self._size) = 0; }
            self._size += 1;
        }
        self._size = new_size;
    }
}

impl IntoIterator for std_vector_int {
    type Item = i32;
    type IntoIter = std_vector_int_iter;
    fn into_iter(self) -> Self::IntoIter {
        std_vector_int_iter { vec: self, index: 0 }
    }
}

pub struct std_vector_int_iter {
    vec: std_vector_int,
    index: usize,
}

impl Iterator for std_vector_int_iter {
    type Item = i32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vec._size {
            let val = unsafe { *self.vec._data.add(self.index) };
            self.index += 1;
            Some(val)
        } else {
            None
        }
    }
}
