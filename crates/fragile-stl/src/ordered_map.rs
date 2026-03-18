// Ordered map surface for std::map<int, int> operations used by current
// runtime fixtures. This keeps deterministic key order and supports
// insert/lookup/update/erase semantics.
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct std_pair_int__int {
    pub first: i32,
    pub second: i32,
}

#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct std_map_int__int {
    entries: std::vec::Vec<std_pair_int__int>,
}

impl std_map_int__int {
    pub fn new_0() -> Self {
        Self {
            entries: std::vec::Vec::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn begin(&mut self) -> *mut std_pair_int__int {
        self.entries.as_mut_ptr()
    }

    pub fn end(&mut self) -> *mut std_pair_int__int {
        unsafe { self.entries.as_mut_ptr().add(self.entries.len()) }
    }

    pub fn find(&mut self, key: i32) -> *mut i32 {
        match self.lower_bound(key) {
            Ok(idx) => &mut self.entries[idx].second as *mut i32,
            Err(_) => std::ptr::null_mut(),
        }
    }

    pub fn count(&self, key: i32) -> usize {
        if self.lower_bound(key).is_ok() {
            1
        } else {
            0
        }
    }

    pub fn erase(&mut self, key: i32) -> usize {
        match self.lower_bound(key) {
            Ok(idx) => {
                self.entries.remove(idx);
                1
            }
            Err(_) => 0,
        }
    }

    pub fn op_index(&mut self, key: i32) -> *mut i32 {
        let idx = match self.lower_bound(key) {
            Ok(idx) => idx,
            Err(idx) => {
                self.entries.insert(
                    idx,
                    std_pair_int__int {
                        first: key,
                        second: 0,
                    },
                );
                idx
            }
        };
        &mut self.entries[idx].second as *mut i32
    }

    pub fn insert_or_assign(&mut self, key: i32, value: i32) -> *mut i32 {
        let value_slot = self.op_index(key);
        unsafe {
            *value_slot = value;
        }
        value_slot
    }

    pub fn as_slice(&self) -> &[std_pair_int__int] {
        self.entries.as_slice()
    }

    fn lower_bound(&self, key: i32) -> Result<usize, usize> {
        self.entries.binary_search_by_key(&key, |pair| pair.first)
    }
}

