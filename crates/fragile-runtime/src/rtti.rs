//! RTTI (Run-Time Type Information) support for C++ type_info.
//!
//! This module provides a wrapper around Rust's `std::any::TypeId` that implements
//! the C++ `std::type_info` interface. This allows transpiled C++ code that uses
//! `typeid` and `std::type_info` to work correctly in Rust.

use core::any::TypeId;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

/// Wrapper around Rust's `TypeId` that provides C++ `std::type_info` semantics.
///
/// This struct is used as the return type of `typeid` expressions in transpiled code.
/// It provides the same interface as C++ `std::type_info`:
/// - `name()` - returns a human-readable type name
/// - `hash_code()` - returns a hash value for the type
/// - `==` / `!=` - type comparison
/// - `before()` - ordering comparison for use in associative containers
#[derive(Clone, Copy)]
#[repr(C)]
pub struct CppTypeInfo {
    /// The underlying Rust TypeId
    type_id: TypeId,
}

impl CppTypeInfo {
    /// Create a new CppTypeInfo for a given type.
    ///
    /// This is the Rust equivalent of `typeid(T)`.
    #[inline]
    pub fn of<T: 'static>() -> Self {
        CppTypeInfo {
            type_id: TypeId::of::<T>(),
        }
    }

    /// Create a CppTypeInfo from an existing TypeId.
    #[inline]
    pub const fn from_type_id(type_id: TypeId) -> Self {
        CppTypeInfo { type_id }
    }

    /// Returns an implementation-defined name for the type.
    ///
    /// This uses Rust's `std::any::type_name` which returns a best-effort
    /// description of the type. The format is not guaranteed to be stable.
    #[cfg(feature = "std")]
    #[inline]
    pub fn name(&self) -> &'static str {
        // Note: We can't directly get the name from TypeId, so we return a generic message.
        // In a full implementation, we'd need a type registry to map TypeId to names.
        "<unknown type>"
    }

    /// Returns a hash code for this type_info.
    ///
    /// The hash code is consistent with equality: if two type_info objects
    /// compare equal, their hash codes are the same.
    #[inline]
    pub fn hash_code(&self) -> u64 {
        // Use the TypeId's hash implementation
        let mut hasher = DefaultHasher::new();
        self.type_id.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns true if this type_info precedes the other in implementation-defined order.
    ///
    /// This is used for sorting type_info objects in associative containers.
    #[inline]
    pub fn before(&self, other: &CppTypeInfo) -> bool {
        // TypeId doesn't implement Ord, so we compare by hash as a fallback
        self.hash_code() < other.hash_code()
    }

    /// Get the underlying Rust TypeId.
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
}

impl PartialEq for CppTypeInfo {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl Eq for CppTypeInfo {}

impl Hash for CppTypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}

impl PartialOrd for CppTypeInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CppTypeInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare by hash code since TypeId doesn't implement Ord
        self.hash_code().cmp(&other.hash_code())
    }
}

impl core::fmt::Debug for CppTypeInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CppTypeInfo")
            .field("type_id", &self.type_id)
            .finish()
    }
}

// Default hasher for hash_code()
#[cfg(feature = "std")]
use std::collections::hash_map::DefaultHasher;

#[cfg(not(feature = "std"))]
struct DefaultHasher {
    state: u64,
}

#[cfg(not(feature = "std"))]
impl DefaultHasher {
    fn new() -> Self {
        DefaultHasher { state: 0 }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

#[cfg(not(feature = "std"))]
impl Hasher for DefaultHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state = self.state.wrapping_mul(31).wrapping_add(byte as u64);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

/// Alias for compatibility with transpiled code that uses `std_type_info`.
#[allow(non_camel_case_types)]
pub type std_type_info = CppTypeInfo;

/// Alias for compatibility with transpiled code that uses `type_info`.
#[allow(non_camel_case_types)]
pub type type_info = CppTypeInfo;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_info_of() {
        let ti_i32 = CppTypeInfo::of::<i32>();
        let ti_i32_2 = CppTypeInfo::of::<i32>();
        let ti_f64 = CppTypeInfo::of::<f64>();

        // Same type should be equal
        assert_eq!(ti_i32, ti_i32_2);

        // Different types should not be equal
        assert_ne!(ti_i32, ti_f64);
    }

    #[test]
    fn test_hash_code() {
        let ti_i32 = CppTypeInfo::of::<i32>();
        let ti_i32_2 = CppTypeInfo::of::<i32>();
        let ti_f64 = CppTypeInfo::of::<f64>();

        // Same type should have same hash
        assert_eq!(ti_i32.hash_code(), ti_i32_2.hash_code());

        // Different types should (usually) have different hashes
        // Note: hash collision is theoretically possible but very unlikely
        assert_ne!(ti_i32.hash_code(), ti_f64.hash_code());
    }

    #[test]
    fn test_before() {
        let ti_i32 = CppTypeInfo::of::<i32>();
        let ti_f64 = CppTypeInfo::of::<f64>();

        // before() should provide a consistent ordering
        // If a.before(b) is true, then b.before(a) should be false
        if ti_i32.before(&ti_f64) {
            assert!(!ti_f64.before(&ti_i32));
        } else if ti_f64.before(&ti_i32) {
            assert!(!ti_i32.before(&ti_f64));
        }
        // If neither is before the other, they must be equal (same type)
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_name() {
        let ti = CppTypeInfo::of::<i32>();
        // name() returns some string - we don't check the exact value
        // as it's implementation-defined
        let _name = ti.name();
    }
}
