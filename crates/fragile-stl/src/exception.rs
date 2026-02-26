// Exception class stub (std::exception base class)
// Forward declaration of exception_vtable
#[repr(C)]
pub struct exception_vtable {
    pub __type_id: u64,
    pub __base_count: usize,
    pub __base_type_ids: &'static [u64],
    pub what: unsafe fn(*const exception) -> *const i8,
    pub __destructor: unsafe fn(*mut exception),
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct exception {
    pub __vtable: *const exception_vtable,
    pub _M_msg: *const i8,  // Message storage for what()
}
impl Default for exception {
    fn default() -> Self { Self { __vtable: std::ptr::null(), _M_msg: std::ptr::null() } }
}
impl exception {
    pub fn new_0() -> Self { Default::default() }
    /// Construct exception with message
    pub fn new_1(msg: *const i8) -> Self {
        Self { __vtable: std::ptr::null(), _M_msg: msg }
    }
    /// Returns exception message
    pub fn what(&self) -> *const i8 {
        if self._M_msg.is_null() {
            b"exception\0".as_ptr() as *const i8
        } else {
            self._M_msg
        }
    }
}

pub mod _V2 {
    use super::error_category;
    static GENERIC_CATEGORY: error_category = error_category { __vtable: std::ptr::null() };
    static SYSTEM_CATEGORY: error_category = error_category { __vtable: std::ptr::null() };
    static IOSTREAM_CATEGORY: error_category = error_category { __vtable: std::ptr::null() };
    
    pub fn generic_category() -> &'static error_category { &GENERIC_CATEGORY }
    pub fn system_category() -> &'static error_category { &SYSTEM_CATEGORY }
    pub fn iostream_category() -> &'static error_category { &IOSTREAM_CATEGORY }
}
// Re-export _V2 functions at module level for convenience
pub use _V2::generic_category;
pub use _V2::system_category;
pub use _V2::iostream_category;
