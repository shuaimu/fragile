// Comparison category stubs for libstdc++/libc++
pub type __cmp_cat_type = i8;
pub type __cmp_cat__Ord = i8;
pub type __cmp_cat__Ncmp = i8;
pub type std___cmp_cat__Ord = i8;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __cmp_cat___unspec { pub value: i8 }
impl __cmp_cat___unspec {
    pub fn new_1(v: i32) -> Self { Self { value: v as i8 } }
}
pub type _CmpUnspecifiedParam = __cmp_cat___unspec;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct partial_ordering { pub _M_value: __cmp_cat_type }
impl partial_ordering {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_1(_v: __cmp_cat__Ord) -> Self { Self { _M_value: 0 } }
    pub fn new_1_1(_v: __cmp_cat__Ncmp) -> Self { Self { _M_value: -127 } }
    pub fn op_eq(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value == 0 }
    pub fn op_ne(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value != 0 }
    pub fn op_lt(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value < 0 && self._M_value != -127 }
    pub fn op_le(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value <= 0 && self._M_value != -127 }
    pub fn op_gt(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value > 0 }
    pub fn op_ge(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value >= 0 }
}
pub static PARTIAL_ORDERING_LESS: partial_ordering = partial_ordering { _M_value: -1 };
pub static PARTIAL_ORDERING_EQUIVALENT: partial_ordering = partial_ordering { _M_value: 0 };
pub static PARTIAL_ORDERING_GREATER: partial_ordering = partial_ordering { _M_value: 1 };
pub static PARTIAL_ORDERING_UNORDERED: partial_ordering = partial_ordering { _M_value: -127 };

// Value-semantics surfaces for fixture-required STL families.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct std_optional_int {
    engaged: bool,
    value: i32,
}

impl std_optional_int {
    pub fn new_0() -> Self {
        Self::default()
    }

    pub fn new_1(value: i32) -> Self {
        Self {
            engaged: true,
            value,
        }
    }

    pub fn has_value(&self) -> bool {
        self.engaged
    }

    pub fn op_bool(&self) -> bool {
        self.has_value()
    }

    pub fn value_or(&self, fallback: i32) -> i32 {
        if self.engaged {
            self.value
        } else {
            fallback
        }
    }

    pub fn emplace(&mut self, value: i32) -> *mut i32 {
        self.engaged = true;
        self.value = value;
        &mut self.value as *mut i32
    }

    pub fn reset(&mut self) {
        self.engaged = false;
        self.value = 0;
    }

    pub fn value_ptr(&mut self) -> *mut i32 {
        if self.engaged {
            &mut self.value as *mut i32
        } else {
            std::ptr::null_mut()
        }
    }

    pub fn value_ptr_const(&self) -> *const i32 {
        if self.engaged {
            &self.value as *const i32
        } else {
            std::ptr::null()
        }
    }

    pub fn assign(&mut self, value: i32) -> *mut i32 {
        self.emplace(value)
    }

    pub fn op_deref(&mut self) -> &mut i32 {
        if self.engaged {
            &mut self.value
        } else {
            panic!("std_optional_int::op_deref called on disengaged optional")
        }
    }

    pub fn op_deref_const(&self) -> &i32 {
        if self.engaged {
            &self.value
        } else {
            panic!("std_optional_int::op_deref_const called on disengaged optional")
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct std_tuple_int__int {
    pub _0: i32,
    pub _1: i32,
}

impl std_tuple_int__int {
    pub fn new_0() -> Self {
        Self::default()
    }

    pub fn new_2(v0: i32, v1: i32) -> Self {
        Self { _0: v0, _1: v1 }
    }

    pub fn get_0(&mut self) -> *mut i32 {
        &mut self._0 as *mut i32
    }

    pub fn get_1(&mut self) -> *mut i32 {
        &mut self._1 as *mut i32
    }

    pub fn get_0_const(&self) -> *const i32 {
        &self._0 as *const i32
    }

    pub fn get_1_const(&self) -> *const i32 {
        &self._1 as *const i32
    }

    pub fn assign(&mut self, v0: i32, v1: i32) {
        self._0 = v0;
        self._1 = v1;
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct std_variant_int__long {
    index: usize,
    int_value: i32,
    long_value: i64,
}

impl std_variant_int__long {
    pub fn new_0() -> Self {
        Self::default()
    }

    pub fn new_1(value: i32) -> Self {
        Self {
            index: 0,
            int_value: value,
            long_value: value as i64,
        }
    }

    pub fn new_1_1(value: i64) -> Self {
        Self {
            index: 1,
            int_value: value as i32,
            long_value: value,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn holds_alternative_int(&self) -> bool {
        self.index == 0
    }

    pub fn holds_alternative_long(&self) -> bool {
        self.index == 1
    }

    pub fn emplace_int(&mut self, value: i32) -> *mut i32 {
        self.index = 0;
        self.int_value = value;
        self.long_value = value as i64;
        &mut self.int_value as *mut i32
    }

    pub fn emplace_long(&mut self, value: i64) -> *mut i64 {
        self.index = 1;
        self.long_value = value;
        self.int_value = value as i32;
        &mut self.long_value as *mut i64
    }

    pub fn get_int_ptr(&mut self) -> *mut i32 {
        if self.holds_alternative_int() {
            &mut self.int_value as *mut i32
        } else {
            std::ptr::null_mut()
        }
    }

    pub fn get_long_ptr(&mut self) -> *mut i64 {
        if self.holds_alternative_long() {
            &mut self.long_value as *mut i64
        } else {
            std::ptr::null_mut()
        }
    }

    pub fn get_int_ptr_const(&self) -> *const i32 {
        if self.holds_alternative_int() {
            &self.int_value as *const i32
        } else {
            std::ptr::null()
        }
    }

    pub fn get_long_ptr_const(&self) -> *const i64 {
        if self.holds_alternative_long() {
            &self.long_value as *const i64
        } else {
            std::ptr::null()
        }
    }
}

// Type trait stubs
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __bool_constant_true;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __bool_constant_false;

// Hash base stubs for std::hash specializations
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__bool;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__char;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__signed_char;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__unsigned_char;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__wchar_t;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__char8_t;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__char16_t;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__char32_t;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__short;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__int;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__long;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__long_long;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__unsigned_short;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__unsigned_int;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__unsigned_long;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__unsigned_long_long;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__float;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__double;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__long_double;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __hash_base_size_t__nullptr_t;

// Numeric traits stubs
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __numeric_traits_floating_float;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __numeric_traits_floating_double;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __numeric_traits_floating_long_double;

// Additional template placeholder stubs
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _dependent_type;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Elt;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Tag;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Sink;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Res;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Ptr;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __size_type;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct integral_constant__Tp____v;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __cv_selector__Unqualified___IsConst___IsVol;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Maybe_unary_or_binary_function__Res___Class___ArgTypes___;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __detected_or_t_ptrdiff_t____diff_t___Ptr;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __detected_or_t_false_type__std___allocator_traits_base___pocca___Alloc;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __detected_or_t_false_type__std___allocator_traits_base___pocs___Alloc;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __strictest_alignment__Types___;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Tuple_impl_0___Elements___;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct std___detail___range_iter_t__Container;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __detail___clamp_iter_cat_typename___traits_type_iterator_category__random_access_iterator_tag;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct integral_constant_size_t__sizeof_____ArgTypes_;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct std_iterator_std_random_access_iterator_tag__bool;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Sp___rep;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Bit_pointer;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Bvector_impl;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct __impl___type_name_t;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct std___libcpp_refstring;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Hash_node_value_type_parameter_0_1__false;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Hash_node_value_type_parameter_0_1__true;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Hash_node_value_type_parameter_0_1____hash_cached_value;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Equal;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _NodeAlloc;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Value;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _Hashtable_alloc_type_parameter_0_0;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct integral_constant_bool____v;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct integral_constant_bool___Constant_iterators;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct integral_constant_bool___Unique_keys;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct _RehashPolicy;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct unsigned_long_const;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct std_pair_bool__std_size_t;
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct std_pair_bool__u64;
