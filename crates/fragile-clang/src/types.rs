//! C++ type representation.

/// A C++ type that can be converted to Rust types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CppType {
    /// void
    Void,
    /// bool
    Bool,
    /// char, signed char, unsigned char
    Char { signed: bool },
    /// short, unsigned short
    Short { signed: bool },
    /// int, unsigned int
    Int { signed: bool },
    /// long, unsigned long
    Long { signed: bool },
    /// long long, unsigned long long
    LongLong { signed: bool },
    /// float
    Float,
    /// double
    Double,
    /// Pointer type: T*
    Pointer {
        pointee: Box<CppType>,
        is_const: bool,
    },
    /// Reference type: T& (lvalue) or T&& (rvalue)
    Reference {
        referent: Box<CppType>,
        is_const: bool,
        /// Whether this is an rvalue reference (T&&) vs lvalue reference (T&)
        is_rvalue: bool,
    },
    /// Array type: T[N]
    Array {
        element: Box<CppType>,
        size: Option<usize>,
    },
    /// Named type (struct, class, enum, typedef)
    Named(String),
    /// Function type: R(Args...)
    Function {
        return_type: Box<CppType>,
        params: Vec<CppType>,
        is_variadic: bool,
    },
}

impl CppType {
    /// Create a signed int type.
    pub fn int() -> Self {
        CppType::Int { signed: true }
    }

    /// Create an unsigned int type.
    pub fn uint() -> Self {
        CppType::Int { signed: false }
    }

    /// Create a pointer to this type.
    pub fn ptr(self) -> Self {
        CppType::Pointer {
            pointee: Box::new(self),
            is_const: false,
        }
    }

    /// Create a const pointer to this type.
    pub fn const_ptr(self) -> Self {
        CppType::Pointer {
            pointee: Box::new(self),
            is_const: true,
        }
    }

    /// Create an lvalue reference to this type.
    pub fn ref_(self) -> Self {
        CppType::Reference {
            referent: Box::new(self),
            is_const: false,
            is_rvalue: false,
        }
    }

    /// Create a const lvalue reference to this type.
    pub fn const_ref(self) -> Self {
        CppType::Reference {
            referent: Box::new(self),
            is_const: true,
            is_rvalue: false,
        }
    }

    /// Create an rvalue reference to this type.
    pub fn rvalue_ref(self) -> Self {
        CppType::Reference {
            referent: Box::new(self),
            is_const: false,
            is_rvalue: true,
        }
    }

    /// Get the equivalent Rust type name.
    pub fn to_rust_type_str(&self) -> String {
        match self {
            CppType::Void => "()".to_string(),
            CppType::Bool => "bool".to_string(),
            CppType::Char { signed: true } => "i8".to_string(),
            CppType::Char { signed: false } => "u8".to_string(),
            CppType::Short { signed: true } => "i16".to_string(),
            CppType::Short { signed: false } => "u16".to_string(),
            CppType::Int { signed: true } => "i32".to_string(),
            CppType::Int { signed: false } => "u32".to_string(),
            CppType::Long { signed: true } => "i64".to_string(),
            CppType::Long { signed: false } => "u64".to_string(),
            CppType::LongLong { signed: true } => "i64".to_string(),
            CppType::LongLong { signed: false } => "u64".to_string(),
            CppType::Float => "f32".to_string(),
            CppType::Double => "f64".to_string(),
            CppType::Pointer { pointee, is_const } => {
                let ptr_type = if *is_const { "*const" } else { "*mut" };
                format!("{} {}", ptr_type, pointee.to_rust_type_str())
            }
            CppType::Reference { referent, is_const, is_rvalue: _ } => {
                // Both lvalue and rvalue references are lowered to raw pointers for FFI
                // The is_rvalue distinction is semantic for C++ but not for FFI
                let ptr_type = if *is_const { "*const" } else { "*mut" };
                format!("{} {}", ptr_type, referent.to_rust_type_str())
            }
            CppType::Array { element, size } => {
                if let Some(n) = size {
                    format!("[{}; {}]", element.to_rust_type_str(), n)
                } else {
                    format!("*mut {}", element.to_rust_type_str())
                }
            }
            CppType::Named(name) => name.clone(),
            CppType::Function { return_type, params, is_variadic } => {
                let params_str: Vec<_> = params.iter().map(|p| p.to_rust_type_str()).collect();
                let params_joined = if *is_variadic {
                    format!("{}, ...", params_str.join(", "))
                } else {
                    params_str.join(", ")
                };
                format!("extern \"C\" fn({}) -> {}", params_joined, return_type.to_rust_type_str())
            }
        }
    }
}
