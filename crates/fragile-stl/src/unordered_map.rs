// Unordered map surface for std::unordered_map<int, int> operations used by
// current runtime fixtures. This implementation uses deterministic fixed
// buckets and real mutable state transitions for insert/lookup/update/erase.
const STD_UNORDERED_MAP_DEFAULT_BUCKET_COUNT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct std_unordered_map_entry_int__int {
    key: i32,
    value: i32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct std_unordered_map_int__int {
    buckets: std::vec::Vec<std::vec::Vec<std::boxed::Box<std_unordered_map_entry_int__int>>>,
    size: usize,
}

impl Default for std_unordered_map_int__int {
    fn default() -> Self {
        Self::new_0()
    }
}

impl std_unordered_map_int__int {
    pub fn new_0() -> Self {
        Self::with_bucket_count(STD_UNORDERED_MAP_DEFAULT_BUCKET_COUNT)
    }

    fn with_bucket_count(bucket_count: usize) -> Self {
        let normalized = bucket_count.max(1);
        let mut buckets = std::vec::Vec::with_capacity(normalized);
        for _ in 0..normalized {
            buckets.push(std::vec::Vec::new());
        }
        Self { buckets, size: 0 }
    }

    #[inline]
    fn bucket_index_for_key(&self, key: i32) -> usize {
        debug_assert!(!self.buckets.is_empty());
        (key as u32 as usize) % self.buckets.len()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn empty(&self) -> bool {
        self.size == 0
    }

    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        self.size = 0;
    }

    pub fn find(&mut self, key: i32) -> *mut i32 {
        let bucket_index = self.bucket_index_for_key(key);
        for entry in &mut self.buckets[bucket_index] {
            if entry.key == key {
                return &mut entry.value as *mut i32;
            }
        }
        std::ptr::null_mut()
    }

    pub fn find_const(&self, key: i32) -> *const i32 {
        let bucket_index = self.bucket_index_for_key(key);
        for entry in &self.buckets[bucket_index] {
            if entry.key == key {
                return &entry.value as *const i32;
            }
        }
        std::ptr::null()
    }

    pub fn count(&self, key: i32) -> usize {
        if self.find_const(key).is_null() {
            0
        } else {
            1
        }
    }

    pub fn contains(&self, key: i32) -> bool {
        self.count(key) == 1
    }

    pub fn op_index(&mut self, key: i32) -> *mut i32 {
        let bucket_index = self.bucket_index_for_key(key);
        for entry in &mut self.buckets[bucket_index] {
            if entry.key == key {
                return &mut entry.value as *mut i32;
            }
        }

        self.buckets[bucket_index].push(std::boxed::Box::new(
            std_unordered_map_entry_int__int { key, value: 0 },
        ));
        self.size += 1;
        let tail = self.buckets[bucket_index]
            .last_mut()
            .expect("unordered_map bucket tail must exist after push");
        &mut tail.value as *mut i32
    }

    pub fn insert_or_assign(&mut self, key: i32, value: i32) -> *mut i32 {
        let value_slot = self.op_index(key);
        unsafe {
            *value_slot = value;
        }
        value_slot
    }

    pub fn insert(&mut self, key: i32, value: i32) -> *mut i32 {
        self.insert_or_assign(key, value)
    }

    pub fn erase(&mut self, key: i32) -> usize {
        let bucket_index = self.bucket_index_for_key(key);
        if let Some(position) = self.buckets[bucket_index]
            .iter()
            .position(|entry| entry.key == key)
        {
            self.buckets[bucket_index].remove(position);
            self.size -= 1;
            1
        } else {
            0
        }
    }

    pub fn as_entries(&self) -> std::vec::Vec<(i32, i32)> {
        let mut out = std::vec::Vec::with_capacity(self.size);
        for bucket in &self.buckets {
            for entry in bucket {
                out.push((entry.key, entry.value));
            }
        }
        out
    }
}
