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
    /// Template parameter type (used in function/class templates).
    /// Represents a type that will be substituted during template instantiation.
    TemplateParam {
        /// Parameter name (e.g., "T", "U")
        name: String,
        /// Template nesting depth (0 for outermost template)
        depth: u32,
        /// Index in the template parameter list (0-based)
        index: u32,
    },
    /// A dependent type that depends on template parameters.
    /// Used for types like "const T&" where T is a template parameter.
    DependentType {
        /// The base spelling of the type (may contain template param names)
        spelling: String,
    },
    /// A template parameter pack (typename... Args).
    /// Represents a variadic template parameter that can match zero or more types.
    ParameterPack {
        /// Parameter name (e.g., "Args")
        name: String,
        /// Template nesting depth (0 for outermost template)
        depth: u32,
        /// Index in the template parameter list (0-based)
        index: u32,
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
            CppType::TemplateParam { name, .. } => {
                // Template parameters are represented by their name
                // In Rust generics, this would be a generic type parameter
                name.clone()
            }
            CppType::DependentType { spelling } => {
                // Dependent types are preserved as their spelling
                // These need to be resolved during template instantiation
                spelling.clone()
            }
            CppType::ParameterPack { name, .. } => {
                // Parameter packs need special handling during expansion
                // For now, represent as the pack name with ... suffix
                format!("{}...", name)
            }
        }
    }

    /// Check if this type is or contains template parameters.
    pub fn is_dependent(&self) -> bool {
        match self {
            CppType::TemplateParam { .. } | CppType::DependentType { .. } | CppType::ParameterPack { .. } => true,
            CppType::Pointer { pointee, .. } => pointee.is_dependent(),
            CppType::Reference { referent, .. } => referent.is_dependent(),
            CppType::Array { element, .. } => element.is_dependent(),
            CppType::Function {
                return_type,
                params,
                ..
            } => return_type.is_dependent() || params.iter().any(|p| p.is_dependent()),
            _ => false,
        }
    }

    /// Create a template parameter type.
    pub fn template_param(name: &str, depth: u32, index: u32) -> Self {
        CppType::TemplateParam {
            name: name.to_string(),
            depth,
            index,
        }
    }

    /// Create a template parameter pack type.
    pub fn parameter_pack(name: &str, depth: u32, index: u32) -> Self {
        CppType::ParameterPack {
            name: name.to_string(),
            depth,
            index,
        }
    }

    /// Check if this type is a parameter pack.
    pub fn is_parameter_pack(&self) -> bool {
        matches!(self, CppType::ParameterPack { .. })
    }

    /// Substitute template parameters with concrete types.
    ///
    /// Given a mapping of template parameter names to concrete types,
    /// returns a new type with all template parameters replaced.
    ///
    /// # Example
    /// ```ignore
    /// // T* with T = int becomes int*
    /// let ty = CppType::Pointer { pointee: CppType::TemplateParam { name: "T", ... } };
    /// let subst = HashMap::from([("T".to_string(), CppType::Int { signed: true })]);
    /// let result = ty.substitute(&subst); // int*
    /// ```
    pub fn substitute(&self, substitutions: &std::collections::HashMap<String, CppType>) -> CppType {
        match self {
            CppType::TemplateParam { name, .. } => {
                substitutions.get(name).cloned().unwrap_or_else(|| self.clone())
            }
            CppType::DependentType { spelling } => {
                // Try to find a template param in the spelling and substitute
                // This is a simplified approach
                if let Some(replacement) = substitutions.get(spelling) {
                    replacement.clone()
                } else {
                    self.clone()
                }
            }
            CppType::ParameterPack { name, .. } => {
                // Parameter packs require special expansion logic.
                // For now, if a single type is provided, use it directly.
                // Full pack expansion is more complex and handled elsewhere.
                substitutions.get(name).cloned().unwrap_or_else(|| self.clone())
            }
            CppType::Pointer { pointee, is_const } => CppType::Pointer {
                pointee: Box::new(pointee.substitute(substitutions)),
                is_const: *is_const,
            },
            CppType::Reference {
                referent,
                is_const,
                is_rvalue,
            } => CppType::Reference {
                referent: Box::new(referent.substitute(substitutions)),
                is_const: *is_const,
                is_rvalue: *is_rvalue,
            },
            CppType::Array { element, size } => CppType::Array {
                element: Box::new(element.substitute(substitutions)),
                size: *size,
            },
            CppType::Function {
                return_type,
                params,
                is_variadic,
            } => CppType::Function {
                return_type: Box::new(return_type.substitute(substitutions)),
                params: params.iter().map(|p| p.substitute(substitutions)).collect(),
                is_variadic: *is_variadic,
            },
            // Non-dependent types remain unchanged
            _ => self.clone(),
        }
    }

    /// Get the type properties for SFINAE/type trait evaluation.
    /// Returns None for dependent types (template parameters).
    pub fn properties(&self) -> Option<TypeProperties> {
        match self {
            // Template parameters have unknown properties
            CppType::TemplateParam { .. }
            | CppType::DependentType { .. }
            | CppType::ParameterPack { .. } => None,

            CppType::Void => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Bool => Some(TypeProperties {
                is_integral: true,
                is_signed: false,
                is_floating_point: false,
                is_scalar: true,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Char { signed } => Some(TypeProperties {
                is_integral: true,
                is_signed: *signed,
                is_floating_point: false,
                is_scalar: true,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Short { signed }
            | CppType::Int { signed }
            | CppType::Long { signed }
            | CppType::LongLong { signed } => Some(TypeProperties {
                is_integral: true,
                is_signed: *signed,
                is_floating_point: false,
                is_scalar: true,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Float | CppType::Double => Some(TypeProperties {
                is_integral: false,
                is_signed: true, // Floating point types are always signed
                is_floating_point: true,
                is_scalar: true,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Pointer { .. } => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: true,
                is_pointer: true,
                is_reference: false,
                is_trivially_copyable: true,
                is_trivially_destructible: true,
            }),

            CppType::Reference { .. } => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: true,
                is_trivially_copyable: false,
                is_trivially_destructible: true,
            }),

            CppType::Array { .. } => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: false,
                // Arrays of trivially copyable types are trivially copyable
                is_trivially_copyable: false, // Conservative default
                is_trivially_destructible: true,
            }),

            CppType::Named(_) => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: false,
                // Named types need lookup to determine properties
                is_trivially_copyable: false, // Conservative default
                is_trivially_destructible: false, // Conservative default
            }),

            CppType::Function { .. } => Some(TypeProperties {
                is_integral: false,
                is_signed: false,
                is_floating_point: false,
                is_scalar: false,
                is_pointer: false,
                is_reference: false,
                is_trivially_copyable: false,
                is_trivially_destructible: true,
            }),
        }
    }

    /// Check if this is an integral type (bool, char, short, int, long, long long).
    pub fn is_integral(&self) -> Option<bool> {
        self.properties().map(|p| p.is_integral)
    }

    /// Check if this is a signed type.
    pub fn is_signed(&self) -> Option<bool> {
        self.properties().map(|p| p.is_signed)
    }

    /// Check if this is a scalar type (arithmetic types, pointers, enum types).
    pub fn is_scalar(&self) -> Option<bool> {
        self.properties().map(|p| p.is_scalar)
    }

    /// Check if this is a floating point type (float, double).
    pub fn is_floating_point(&self) -> Option<bool> {
        self.properties().map(|p| p.is_floating_point)
    }

    /// Check if this is an arithmetic type (integral or floating point).
    pub fn is_arithmetic(&self) -> Option<bool> {
        self.properties().map(|p| p.is_integral || p.is_floating_point)
    }
}

/// Type properties for SFINAE and type trait evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeProperties {
    /// True for bool, char, short, int, long, long long (signed or unsigned)
    pub is_integral: bool,
    /// True for signed types, false for unsigned
    pub is_signed: bool,
    /// True for float, double, long double
    pub is_floating_point: bool,
    /// True for arithmetic types and pointers
    pub is_scalar: bool,
    /// True for pointer types
    pub is_pointer: bool,
    /// True for reference types (lvalue or rvalue)
    pub is_reference: bool,
    /// True if the type can be safely memcpy'd
    pub is_trivially_copyable: bool,
    /// True if the destructor is trivial
    pub is_trivially_destructible: bool,
}

/// Type trait evaluation results.
/// Used for evaluating Clang's built-in type traits like __is_integral(T).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeTraitResult {
    /// The trait evaluates to a known boolean value
    Value(bool),
    /// The trait cannot be evaluated (e.g., depends on template parameters)
    Dependent,
}

impl TypeTraitResult {
    /// Returns true if this result is a definite true value.
    pub fn is_true(&self) -> bool {
        matches!(self, TypeTraitResult::Value(true))
    }

    /// Returns true if this result is a definite false value.
    pub fn is_false(&self) -> bool {
        matches!(self, TypeTraitResult::Value(false))
    }

    /// Returns true if the result depends on template parameters.
    pub fn is_dependent(&self) -> bool {
        matches!(self, TypeTraitResult::Dependent)
    }

    /// Get the boolean value if known, None if dependent.
    pub fn to_bool(&self) -> Option<bool> {
        match self {
            TypeTraitResult::Value(v) => Some(*v),
            TypeTraitResult::Dependent => None,
        }
    }
}

/// Evaluates type traits against concrete or dependent types.
pub struct TypeTraitEvaluator;

impl TypeTraitEvaluator {
    /// Evaluate __is_integral(T)
    pub fn is_integral(ty: &CppType) -> TypeTraitResult {
        match ty.is_integral() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_signed(T)
    pub fn is_signed(ty: &CppType) -> TypeTraitResult {
        match ty.is_signed() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_unsigned(T)
    pub fn is_unsigned(ty: &CppType) -> TypeTraitResult {
        match ty.is_signed() {
            Some(signed) => TypeTraitResult::Value(!signed),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_floating_point(T)
    pub fn is_floating_point(ty: &CppType) -> TypeTraitResult {
        match ty.is_floating_point() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_arithmetic(T)
    pub fn is_arithmetic(ty: &CppType) -> TypeTraitResult {
        match ty.is_arithmetic() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_scalar(T)
    pub fn is_scalar(ty: &CppType) -> TypeTraitResult {
        match ty.is_scalar() {
            Some(v) => TypeTraitResult::Value(v),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_pointer(T)
    pub fn is_pointer(ty: &CppType) -> TypeTraitResult {
        match ty.properties() {
            Some(p) => TypeTraitResult::Value(p.is_pointer),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_reference(T)
    pub fn is_reference(ty: &CppType) -> TypeTraitResult {
        match ty.properties() {
            Some(p) => TypeTraitResult::Value(p.is_reference),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_same(T, U)
    pub fn is_same(ty1: &CppType, ty2: &CppType) -> TypeTraitResult {
        // If either type is dependent, result is dependent
        if ty1.is_dependent() || ty2.is_dependent() {
            return TypeTraitResult::Dependent;
        }
        TypeTraitResult::Value(ty1 == ty2)
    }

    /// Evaluate __is_trivially_copyable(T)
    pub fn is_trivially_copyable(ty: &CppType) -> TypeTraitResult {
        match ty.properties() {
            Some(p) => TypeTraitResult::Value(p.is_trivially_copyable),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_trivially_destructible(T)
    pub fn is_trivially_destructible(ty: &CppType) -> TypeTraitResult {
        match ty.properties() {
            Some(p) => TypeTraitResult::Value(p.is_trivially_destructible),
            None => TypeTraitResult::Dependent,
        }
    }

    /// Evaluate __is_base_of(Base, Derived)
    /// Note: This requires class hierarchy information which we don't have yet.
    /// For now, returns Dependent for named types.
    pub fn is_base_of(base: &CppType, derived: &CppType) -> TypeTraitResult {
        // If either type is dependent, result is dependent
        if base.is_dependent() || derived.is_dependent() {
            return TypeTraitResult::Dependent;
        }

        // If types are the same, a class is considered a base of itself
        if base == derived {
            return TypeTraitResult::Value(true);
        }

        // For Named types, we would need class hierarchy information
        // For now, return Dependent to indicate we can't evaluate this
        match (base, derived) {
            (CppType::Named(_), CppType::Named(_)) => TypeTraitResult::Dependent,
            // Non-class types: false (not a class hierarchy relationship)
            _ => TypeTraitResult::Value(false),
        }
    }
}
