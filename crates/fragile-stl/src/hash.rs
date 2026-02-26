// Hash function stubs for libstdc++
#[inline]
pub fn _Hash_bytes(_ptr: *const (), _len: u64, _seed: u64) -> u64 {
    // Simple FNV-1a hash stub
    let mut hash: u64 = 14695981039346656037;
    let slice = unsafe { std::slice::from_raw_parts(_ptr as *const u8, _len as usize) };
    for b in slice {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash ^ _seed
}

#[inline]
pub fn _Fnv_hash_bytes(_ptr: *const (), _len: u64, _seed: u64) -> u64 {
    // FNV-1a hash
    _Hash_bytes(_ptr, _len, _seed)
}

// _Hash_impl struct for libstdc++ hash support
#[derive(Default, Clone, Copy)]
pub struct _Hash_impl;
impl _Hash_impl {
    pub fn new_0() -> Self { Self }
    pub fn hash(_ptr: *const (), _len: u64, _seed: u64) -> u64 { _Hash_bytes(_ptr, _len, _seed) }
    pub fn hash_i32(_v: &i32) -> u64 { *_v as u64 }
    pub fn hash_ptr_const___<T>(_ptr: *const T, _len: u64, _seed: u64) -> u64 { _Hash_bytes(_ptr as *const (), _len, _seed) }
    pub fn __hash_combine_std_error_categoryconst(_cat: *const error_category, _hash: u64) -> u64 { _hash }
}
