// std::string stub implementation
#[repr(C)]
#[derive(Default)]
pub struct std_string {
    _data: *mut i8,
    _size: usize,
    _capacity: usize,
}

impl std_string {
    pub const npos: u64 = u64::MAX;

    fn alloc_i8(capacity: usize) -> Option<*mut i8> {
        let layout = match std::alloc::Layout::array::<i8>(capacity) {
            Ok(layout) => layout,
            Err(_) => return None,
        };
        let ptr = unsafe { std::alloc::alloc(layout) as *mut i8 };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Some(ptr)
    }

    fn dealloc_i8(ptr: *mut i8, capacity: usize) {
        if ptr == std::ptr::null_mut() {
            return;
        }
        if capacity == 0 {
            return;
        }
        let layout = match std::alloc::Layout::array::<i8>(capacity) {
            Ok(layout) => layout,
            Err(_) => return,
        };
        unsafe {
            std::alloc::dealloc(ptr as *mut u8, layout);
        }
    }

    pub fn new_0() -> Self {
        Self {
            _data: std::ptr::null_mut(),
            _size: 0,
            _capacity: 0,
        }
    }
    pub fn new_1(s: *const i8) -> Self {
        if s.is_null() {
            return Self::new_0();
        }
        let mut len = 0usize;
        unsafe {
            while *s.add(len) != 0 {
                len += 1;
            }
        }
        let cap = len.saturating_add(1);
        let buf_ptr = match Self::alloc_i8(cap) {
            Some(ptr) => ptr,
            None => return Self::new_0(),
        };
        unsafe {
            std::ptr::copy_nonoverlapping(s, buf_ptr, len);
        }
        unsafe {
            *buf_ptr.add(len) = 0;
        }
        Self {
            _data: buf_ptr,
            _size: len,
            _capacity: cap,
        }
    }
    fn ensure_capacity(&mut self, min_capacity: usize) -> bool {
        if self._capacity >= min_capacity && self._data != std::ptr::null_mut() {
            return true;
        }
        let mut new_cap = if self._capacity == 0 {
            16
        } else {
            self._capacity
        };
        while new_cap < min_capacity {
            if new_cap > (usize::MAX / 2) {
                new_cap = min_capacity;
                break;
            }
            new_cap *= 2;
        }
        if new_cap < min_capacity {
            new_cap = min_capacity;
        }
        let new_buf_ptr = match Self::alloc_i8(new_cap) {
            Some(ptr) => ptr,
            None => return false,
        };
        if self._data != std::ptr::null_mut() {
            if self._size > 0 {
                unsafe {
                    std::ptr::copy_nonoverlapping(self._data, new_buf_ptr, self._size);
                }
            }
            Self::dealloc_i8(self._data, self._capacity);
        }
        self._data = new_buf_ptr;
        self._capacity = new_cap;
        true
    }
    pub fn resize(&mut self, new_size: u64) {
        let new_size = new_size as usize;
        if self.ensure_capacity(new_size.saturating_add(1)) == false {
            return;
        }
        if new_size > self._size {
            for i in self._size..new_size {
                unsafe {
                    *self._data.add(i) = 0;
                }
            }
        }
        self._size = new_size;
        unsafe {
            *self._data.add(self._size) = 0;
        }
    }
    pub fn op_index(&mut self, index: u64) -> &mut i8 {
        let idx = index as usize;
        if idx >= self._size {
            self.resize((idx + 1) as u64);
            if idx >= self._size || self._data == std::ptr::null_mut() {
                panic!("std::string index expansion failed");
            }
        }
        unsafe { &mut *self._data.add(idx) }
    }
    pub fn op_index_const(&self, index: u64) -> &i8 {
        let idx = index as usize;
        if self._data == std::ptr::null_mut() || idx >= self._size {
            static ZERO: i8 = 0;
            return &ZERO;
        }
        unsafe { &*self._data.add(idx) }
    }
    pub fn c_str(&self) -> *const i8 {
        if self._data == std::ptr::null_mut() {
            b"\0".as_ptr() as *const i8
        } else {
            self._data as *const i8
        }
    }
    pub fn size(&self) -> usize {
        self._size
    }
    pub fn length(&self) -> usize {
        self._size
    }
    pub fn empty(&self) -> bool {
        self._size == 0
    }
    pub fn push_back(&mut self, c: i8) {
        if self.ensure_capacity(self._size.saturating_add(2)) == false {
            return;
        }
        unsafe {
            *self._data.add(self._size) = c;
        }
        self._size += 1;
        unsafe {
            *self._data.add(self._size) = 0;
        }
    }
    pub fn append(&mut self, s: *const i8) -> &mut Self {
        if s.is_null() {
            return self;
        }
        let mut len = 0usize;
        unsafe {
            while *s.add(len) != 0 {
                len += 1;
            }
        }
        for i in 0..len {
            self.push_back(unsafe { *s.add(i) });
        }
        self
    }
    pub fn op_plus_assign(&mut self, s: *const i8) -> &mut Self {
        self.append(s)
    }
    pub fn clear(&mut self) {
        self._size = 0;
        if self._data != std::ptr::null_mut() {
            unsafe {
                *self._data = 0;
            }
        }
    }
    pub fn capacity(&self) -> usize {
        self._capacity
    }
    pub fn substr(&self, pos: u64, count: u64) -> std_string {
        let pos = pos as usize;
        let count = (count as usize).min(self._size.saturating_sub(pos));
        if pos >= self._size || self._data == std::ptr::null_mut() || count == 0 {
            return std_string::new_0();
        }
        let mut result = std_string::new_0();
        for i in 0..count {
            result.push_back(unsafe { *self._data.add(pos + i) });
        }
        result
    }
}

impl Drop for std_string {
    fn drop(&mut self) {
        Self::dealloc_i8(self._data, self._capacity);
    }
}

impl Clone for std_string {
    fn clone(&self) -> Self {
        if self._data == std::ptr::null_mut() || self._size == 0 {
            return Self::new_0();
        }
        Self::new_1(self._data as *const i8)
    }
}

impl std::fmt::Display for std_string {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self._data == std::ptr::null_mut() || self._size == 0 {
            return Ok(());
        }
        let slice = unsafe { std::slice::from_raw_parts(self._data as *const u8, self._size) };
        let s = std::str::from_utf8(slice).unwrap_or("<invalid utf8>");
        f.write_str(s)
    }
}
