//! C++ type representation.

/// Parse comma-separated template arguments, respecting nested templates.
/// Returns a vector of trimmed argument strings.
///
/// # Example
/// ```ignore
/// let args = parse_template_args("int, std::vector<int>, double");
/// assert_eq!(args, vec!["int", "std::vector<int>", "double"]);
/// ```
pub fn parse_template_args(args: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in args.chars() {
        match ch {
            '<' => {
                depth += 1;
                current.push(ch);
            }
            '>' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    result.push(trimmed);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        result.push(trimmed);
    }

    result
}

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

    /// Get the pointee type for a pointer type.
    /// Returns None if this is not a pointer type.
    pub fn pointee(&self) -> Option<&CppType> {
        match self {
            CppType::Pointer { pointee, .. } => Some(pointee.as_ref()),
            _ => None,
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
                // Special case: function pointers use Option<fn(...)> syntax in Rust
                if let CppType::Function {
                    return_type,
                    params,
                    is_variadic,
                } = pointee.as_ref()
                {
                    let params_str: Vec<_> = params.iter().map(|p| p.to_rust_type_str()).collect();
                    let params_joined = if *is_variadic {
                        format!("{}, ...", params_str.join(", "))
                    } else {
                        params_str.join(", ")
                    };
                    // Use Option to handle nullable function pointers
                    // Note: We don't use extern "C" since transpiled functions use Rust calling convention
                    format!(
                        "Option<fn({}) -> {}>",
                        params_joined,
                        return_type.to_rust_type_str()
                    )
                } else {
                    // Regular pointer - respect const
                    let ptr_type = if *is_const { "*const" } else { "*mut" };
                    format!("{} {}", ptr_type, pointee.to_rust_type_str())
                }
            }
            CppType::Reference {
                referent,
                is_const,
                is_rvalue: _,
            } => {
                // C++ references map to Rust references for transpilation
                let ref_type = if *is_const { "&" } else { "&mut " };
                format!("{}{}", ref_type, referent.to_rust_type_str())
            }
            CppType::Array { element, size } => {
                if let Some(n) = size {
                    format!("[{}; {}]", element.to_rust_type_str(), n)
                } else {
                    format!("*mut {}", element.to_rust_type_str())
                }
            }
            CppType::Named(name) => {
                // Normalize the name by stripping const/volatile qualifiers for matching
                let normalized_name = name
                    .trim_start_matches("const ")
                    .trim_start_matches("volatile ")
                    .trim();
                // Handle special C++ types that don't map directly to Rust
                match normalized_name {
                    "float" => "f32".to_string(),
                    "double" | "long double" => "f64".to_string(), // Rust doesn't have long double
                    "bool" => "bool".to_string(),
                    "long long" | "long long int" | "long_long" | "long_long_int" => {
                        "i64".to_string()
                    }
                    "unsigned long long"
                    | "unsigned long long int"
                    | "unsigned_long_long"
                    | "unsigned_long_long_int" => "u64".to_string(),
                    "long" | "long int" | "long_int" => "i64".to_string(),
                    "unsigned long" | "unsigned long int" | "unsigned_long"
                    | "unsigned_long_int" => "u64".to_string(),
                    "int" => "i32".to_string(),
                    "unsigned" | "unsigned int" => "u32".to_string(),
                    "short" | "short int" => "i16".to_string(),
                    "unsigned short" | "unsigned short int" => "u16".to_string(),
                    "signed char" => "i8".to_string(),
                    "unsigned char" => "u8".to_string(),
                    "char" => "i8".to_string(),
                    "wchar_t" => "i32".to_string(),
                    "char8_t" => "u8".to_string(),
                    "char16_t" => "u16".to_string(),
                    "char32_t" => "u32".to_string(),
                    // Standard library size types (handle both with and without std:: prefix)
                    "size_t" | "std::size_t" => "usize".to_string(),
                    "ssize_t" | "ptrdiff_t" | "std::ptrdiff_t" => "isize".to_string(),
                    "intptr_t" | "std::intptr_t" => "isize".to_string(),
                    "uintptr_t" | "std::uintptr_t" => "usize".to_string(),
                    // Fixed-width integer types from <cstdint>
                    "int8_t" | "std::int8_t" => "i8".to_string(),
                    "int16_t" | "std::int16_t" => "i16".to_string(),
                    "int32_t" | "std::int32_t" => "i32".to_string(),
                    "int64_t" | "std::int64_t" => "i64".to_string(),
                    "uint8_t" | "std::uint8_t" => "u8".to_string(),
                    "uint16_t" | "std::uint16_t" => "u16".to_string(),
                    "uint32_t" | "std::uint32_t" => "u32".to_string(),
                    "uint64_t" | "std::uint64_t" => "u64".to_string(),
                    // 128-bit integer types
                    "__int128" | "__int128_t" => "i128".to_string(),
                    "unsigned __int128" | "__uint128_t" => "u128".to_string(),
                    // C variadic function support
                    "va_list" | "__builtin_va_list" | "__va_list_tag" | "struct __va_list_tag" => {
                        "std::ffi::VaList".to_string()
                    }
                    // C standard I/O
                    "FILE" | "struct _IO_FILE" => "std::ffi::c_void".to_string(), // Opaque file handle
                    // nullptr_t type
                    "std::nullptr_t" | "nullptr_t" | "decltype(nullptr)" => {
                        "*mut std::ffi::c_void".to_string()
                    }
                    // Common STL member type aliases used across container types
                    // These are typedefs like vector<T>::size_type that appear in template code
                    "size_type" => "usize".to_string(),
                    "difference_type" => "isize".to_string(),
                    // STL value access types - use c_void as placeholder for generic element type
                    "value_type" => "std::ffi::c_void".to_string(),
                    "reference" | "const_reference" => "&std::ffi::c_void".to_string(),
                    "pointer" | "const_pointer" => "*mut std::ffi::c_void".to_string(),
                    // STL iterator types - use raw pointers as placeholder
                    "iterator" | "const_iterator" => "*mut std::ffi::c_void".to_string(),
                    "reverse_iterator" | "const_reverse_iterator" => {
                        "*mut std::ffi::c_void".to_string()
                    }
                    // Allocator types
                    "allocator_type" => "std::ffi::c_void".to_string(),
                    // Common template parameter names that appear unresolved
                    "_Tp" | "_CharT" | "_Traits" | "_Allocator" | "_Alloc" => {
                        "std::ffi::c_void".to_string()
                    }
                    "_Pointer" | "_Iter" | "_Iterator" | "_Size" | "_Ep" => {
                        "std::ffi::c_void".to_string()
                    }
                    "_Rp" | "_Ip" | "_Container" | "_BaseT" | "_It" => {
                        "std::ffi::c_void".to_string()
                    }
                    "_Gen" | "_Func" | "_Rollback" | "_StorageAlloc" => {
                        "std::ffi::c_void".to_string()
                    }
                    "_ControlBlockAlloc" | "_ControlBlockAllocator" => {
                        "std::ffi::c_void".to_string()
                    }
                    "_Sp" | "_Dp" | "_Up" | "_Yp" => "std::ffi::c_void".to_string(), // Smart pointer params
                    // libstdc++ bit vector internal types
                    "_Bit_type" => "u64".to_string(), // Typically unsigned long
                    "_Tp_alloc_type" | "_Bit_alloc_type" => "std::ffi::c_void".to_string(), // Allocator type alias
                    // Smart pointer internal types
                    "_Sp___rep" => "std::ffi::c_void".to_string(), // shared_ptr refcount
                    // Dependent types from templates
                    "_dependent_type" => "std::ffi::c_void".to_string(),
                    // libstdc++ comparison category types
                    "__cmp_cat_type" | "__cmp_cat__Ord" | "__cmp_cat__Ncmp" => "i8".to_string(),
                    "__cmp_cat___unspec" => "i8".to_string(),
                    // libc++ internal proxy and impl types
                    "__proxy" | "__value_type" => "std::ffi::c_void".to_string(),
                    "std___libcpp_refstring" => "std::ffi::c_void".to_string(),
                    // Stream types
                    "__stream_type" | "ostream_type" | "istream_type" => {
                        "std::ffi::c_void".to_string()
                    }
                    "fmtflags" => "u32".to_string(), // ios_base::fmtflags is an integer type
                    // Optional type
                    "nullopt_t" => "()".to_string(),
                    // Time value types
                    "timeval" => "i64".to_string(),
                    // libc++ internal string representation types
                    "__long" | "__rep" | "rep" => "std::ffi::c_void".to_string(),
                    // Duration types
                    "duration" => "i64".to_string(),
                    // libc++ internal string types
                    "__self_view" | "string" | "std::string" => "std::ffi::c_void".to_string(),
                    "__storage_pointer" => "*mut std::ffi::c_void".to_string(),
                    // Allocator-related types that appear in container implementations
                    "__alloc_traits_difference_type" => "isize".to_string(),
                    // libc++ internal types
                    "__syscall_slong_t" | "__syscall_ulong_t" => "i64".to_string(),
                    "__type_name_t" => "*const i8".to_string(), // RTTI type name pointer
                    // Boolean type traits used for tag dispatching
                    "true_type" | "std::true_type" => "bool".to_string(),
                    "false_type" | "std::false_type" => "bool".to_string(),
                    // C++ exception types - these are rarely instantiated directly
                    // Map to c_void to avoid generating complex inheritance hierarchies
                    "logic_error" | "std::logic_error" => "std::ffi::c_void".to_string(),
                    "runtime_error" | "std::runtime_error" => "std::ffi::c_void".to_string(),
                    "bad_alloc" | "std::bad_alloc" => "std::ffi::c_void".to_string(),
                    "exception" | "std::exception" => "std::ffi::c_void".to_string(),
                    // Time and stream types
                    "timespec" => "i64".to_string(), // Simplify to i64 timestamp
                    "streambuf_type" | "char_type" => "std::ffi::c_void".to_string(),
                    "memory_resource" => "std::ffi::c_void".to_string(),
                    // More template parameter placeholders
                    "_ValueType" | "_Sent" | "_Hp" => "std::ffi::c_void".to_string(),
                    "__storage_type" => "usize".to_string(),
                    // NOTE: STL string type mappings removed - types pass through as-is
                    // See Section 22 in TODO.md for rationale
                    _ => {
                        // Map std::vector<T> instantiations to the base template struct
                        // The transpiler generates a single `vector__Tp___Alloc` struct
                        if normalized_name.starts_with("std::vector<")
                            || normalized_name.starts_with("vector<")
                        {
                            return "vector__Tp___Alloc".to_string();
                        }
                        // Map std::_Bit_iterator to _Bit_iterator (strip std:: prefix)
                        if normalized_name == "std::_Bit_iterator" {
                            return "_Bit_iterator".to_string();
                        }
                        if normalized_name == "std::_Bit_const_iterator" {
                            return "_Bit_const_iterator".to_string();
                        }
                        // NOTE: STL type mappings removed - types pass through as-is
                        // std::vector, std::string, std::optional, std::array, std::span
                        // See Section 22 in TODO.md for rationale
                        // NOTE: std::map and std::unordered_map mappings removed - types pass through as-is
                        // See Section 22 in TODO.md for rationale
                        // NOTE: All remaining STL mappings removed - types pass through as-is
                        // smart pointers, I/O streams, std::variant
                        // See Section 22 in TODO.md for rationale
                        // Handle decltype expressions - replace with unit type placeholder
                        if name.starts_with("decltype(") {
                            return "()".to_string();
                        }
                        // Handle typeof expressions similarly
                        if name.starts_with("typeof(") || name.starts_with("__typeof__(") {
                            return "()".to_string();
                        }
                        // Handle lambda types - use inference placeholder
                        // Lambda types look like "(lambda at /path/file.cpp:line:col)"
                        if name.starts_with("(lambda at ") || name.contains("lambda at ") {
                            return "_".to_string(); // Let Rust infer the closure type
                        }
                        // Handle auto type (C++11) - use Rust type inference
                        if name == "auto" {
                            return "_".to_string();
                        }
                        // Handle Clang template parameter placeholders like type-parameter-0-0
                        // These are unresolved template parameters from template definitions
                        // Note: Use normalized_name to handle const-qualified types
                        if normalized_name.starts_with("type-parameter-")
                            || normalized_name.starts_with("type_parameter_")
                        {
                            return "std::ffi::c_void".to_string();
                        }
                        // Handle complex conditional types from libc++ template metaprogramming
                        // These are SFINAE/conditional type expressions that can't be represented
                        // Check both the template form (_If<...>) and sanitized form (_If_...)
                        if normalized_name.starts_with("__conditional_t")
                            || normalized_name.starts_with("_If<")  // Original template form
                            || normalized_name.starts_with("_If_")  // Sanitized form
                            || normalized_name.contains("__conditional_t")
                        // Also catch it in middle
                        {
                            return "std::ffi::c_void".to_string();
                        }
                        // Handle typename-prefixed dependent types
                        if normalized_name.starts_with("typename")
                            || normalized_name.starts_with("typename_")
                        {
                            return "std::ffi::c_void".to_string();
                        }
                        // Handle libc++ variant implementation detail types
                        if normalized_name.starts_with("__variant_detail") {
                            return "std::ffi::c_void".to_string();
                        }
                        // Handle iterator traits types
                        if normalized_name.starts_with("iter_") {
                            return "std::ffi::c_void".to_string();
                        }
                        // Handle type trait result types (add_pointer_t, make_unsigned_t, etc.)
                        if normalized_name.starts_with("add_pointer_t")
                            || normalized_name.starts_with("make_unsigned_t")
                            || normalized_name.starts_with("sentinel_t")
                            || normalized_name.starts_with("iterator_t")
                            || normalized_name.starts_with("__insert_iterator")
                            || normalized_name.starts_with("__impl_")
                        {
                            return "std::ffi::c_void".to_string();
                        }
                        // Strip C++ qualifiers that aren't valid in Rust type names
                        let cleaned = name
                            .trim_start_matches("const ")
                            .trim_start_matches("volatile ")
                            .trim_start_matches("struct ")
                            .trim_start_matches("class ")
                            .trim_start_matches("enum ")
                            .trim_end(); // Remove trailing whitespace

                        // Strip inline namespace versioning used by libc++ (e.g., std::__1:: -> std::)
                        // libc++ uses __1, __2, etc. as ABI versioning namespaces
                        let cleaned = cleaned
                            .replace("::__1::", "::")
                            .replace("::__2::", "::")
                            .replace("::__ndk1::", "::"); // Android NDK uses __ndk1

                        // Handle remaining "unsigned TYPE" patterns
                        let cleaned: String = if cleaned.starts_with("unsigned ") {
                            match cleaned.trim_start_matches("unsigned ") {
                                "int" => "u32".to_string(),
                                "long" => "u64".to_string(),
                                "short" => "u16".to_string(),
                                "char" => "u8".to_string(),
                                _ => cleaned.clone(),
                            }
                        } else if cleaned.starts_with("signed ") {
                            match cleaned.trim_start_matches("signed ") {
                                "int" => "i32".to_string(),
                                "long" => "i64".to_string(),
                                "short" => "i16".to_string(),
                                "char" => "i8".to_string(),
                                _ => cleaned.clone(),
                            }
                        } else {
                            cleaned
                        };

                        // Handle C++ array types that appear as Named types with bracket notation
                        // e.g., _Tp[_Size] -> [_Tp; _Size] (Rust array syntax)
                        // This happens with template parameters like std::array's __elems_ field
                        // BUT skip if the result would be used as a struct name (no size specified)
                        // e.g., type-parameter-0-0[] should become type_parameter_0_0_Arr not [type; ]
                        if let Some(bracket_idx) = cleaned.find('[') {
                            let element_type = &cleaned[..bracket_idx];
                            let rest = &cleaned[bracket_idx + 1..];
                            if let Some(close_bracket) = rest.find(']') {
                                let size = &rest[..close_bracket].trim();
                                // Only convert to array syntax if size is non-empty (looks like actual array)
                                if !size.is_empty()
                                    && element_type
                                        .chars()
                                        .all(|c| c.is_alphanumeric() || c == '_')
                                {
                                    // Recursively convert the element type and size
                                    let elem_rust =
                                        CppType::Named(element_type.to_string()).to_rust_type_str();
                                    let size_rust = size.replace("-", "_").replace(".", "_");
                                    return format!("[{}; {}]", elem_rust, size_rust);
                                }
                                // Empty size like T[] - just convert to Arr suffix
                                // This is used for unique_ptr<T[]> style types
                            }
                        }

                        // Replace :: with _ for namespaced types
                        // Convert template syntax to valid Rust identifiers:
                        // e.g., std::vector<int> -> std_vector_int
                        // e.g., type-parameter-0-0 -> type_parameter_0_0
                        // Note: replace "::" first, then single ":" for line:col references
                        cleaned
                            .replace("::", "_")
                            .replace(":", "_") // Single colon in file line:col references
                            .replace("<", "_") // Convert template open bracket
                            .replace(">", "") // Remove template close bracket
                            .replace(",", "_") // Handle multiple template params
                            .replace(" *", "") // Remove trailing pointer indicators
                            .replace("*", "")
                            .replace("&&", "_") // C++ rvalue reference in type names
                            .replace("&", "_") // C++ reference in type names
                            .replace("[]", "_Arr") // Array type notation (e.g., T[] -> T_Arr)
                            .replace("[", "_") // Expression grouping
                            .replace("]", "_") // Expression grouping
                            .replace(" ", "_")
                            .replace("-", "_") // Clang uses dashes in template param names
                            .replace(".", "_") // Variadic pack expansion uses ...
                            .replace("+", "_") // Template expressions (Index + 1)
                            .replace("(", "_") // Expression grouping
                            .replace(")", "_")
                            .replace("/", "_") // File paths in anonymous union names from system headers
                    }
                }
            }
            CppType::Function {
                return_type,
                params,
                is_variadic,
            } => {
                let params_str: Vec<_> = params.iter().map(|p| p.to_rust_type_str()).collect();
                let params_joined = if *is_variadic {
                    format!("{}, ...", params_str.join(", "))
                } else {
                    params_str.join(", ")
                };
                format!(
                    "extern \"C\" fn({}) -> {}",
                    params_joined,
                    return_type.to_rust_type_str()
                )
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
            CppType::TemplateParam { .. }
            | CppType::DependentType { .. }
            | CppType::ParameterPack { .. } => true,
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
    pub fn substitute(
        &self,
        substitutions: &std::collections::HashMap<String, CppType>,
    ) -> CppType {
        match self {
            CppType::TemplateParam { name, .. } => substitutions
                .get(name)
                .cloned()
                .unwrap_or_else(|| self.clone()),
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
                substitutions
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| self.clone())
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
                is_trivially_copyable: false,     // Conservative default
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
        self.properties()
            .map(|p| p.is_integral || p.is_floating_point)
    }

    /// Get the bit width of this type.
    ///
    /// Returns None for types that don't have a fixed bit width (named types,
    /// dependent types, function types, etc.).
    ///
    /// Assumes LP64 data model (common on 64-bit Unix):
    /// - char: 8 bits
    /// - short: 16 bits
    /// - int: 32 bits
    /// - long: 64 bits
    /// - long long: 64 bits
    pub fn bit_width(&self) -> Option<u32> {
        match self {
            CppType::Bool => Some(8), // Rust bool is 1 byte for FFI compatibility
            CppType::Char { .. } => Some(8),
            CppType::Short { .. } => Some(16),
            CppType::Int { .. } => Some(32),
            CppType::Long { .. } => Some(64), // LP64 model
            CppType::LongLong { .. } => Some(64),
            CppType::Float => Some(32),
            CppType::Double => Some(64),
            CppType::Pointer { .. } => Some(64), // 64-bit pointers
            CppType::Reference { .. } => Some(64), // References are pointer-sized
            // Types without fixed bit width
            CppType::Void
            | CppType::Array { .. }
            | CppType::Named(_)
            | CppType::Function { .. }
            | CppType::TemplateParam { .. }
            | CppType::DependentType { .. }
            | CppType::ParameterPack { .. } => None,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_width_primitive_types() {
        // Bool
        assert_eq!(CppType::Bool.bit_width(), Some(8));

        // Char
        assert_eq!(CppType::Char { signed: true }.bit_width(), Some(8));
        assert_eq!(CppType::Char { signed: false }.bit_width(), Some(8));

        // Short
        assert_eq!(CppType::Short { signed: true }.bit_width(), Some(16));
        assert_eq!(CppType::Short { signed: false }.bit_width(), Some(16));

        // Int
        assert_eq!(CppType::Int { signed: true }.bit_width(), Some(32));
        assert_eq!(CppType::Int { signed: false }.bit_width(), Some(32));

        // Long (LP64 model)
        assert_eq!(CppType::Long { signed: true }.bit_width(), Some(64));
        assert_eq!(CppType::Long { signed: false }.bit_width(), Some(64));

        // Long Long
        assert_eq!(CppType::LongLong { signed: true }.bit_width(), Some(64));
        assert_eq!(CppType::LongLong { signed: false }.bit_width(), Some(64));

        // Float/Double
        assert_eq!(CppType::Float.bit_width(), Some(32));
        assert_eq!(CppType::Double.bit_width(), Some(64));
    }

    #[test]
    fn test_bit_width_pointer_and_reference() {
        // Pointers are 64-bit on LP64
        let ptr = CppType::Pointer {
            pointee: Box::new(CppType::Int { signed: true }),
            is_const: false,
        };
        assert_eq!(ptr.bit_width(), Some(64));

        // References are also pointer-sized
        let ref_ = CppType::Reference {
            referent: Box::new(CppType::Int { signed: true }),
            is_const: false,
            is_rvalue: false,
        };
        assert_eq!(ref_.bit_width(), Some(64));
    }

    #[test]
    fn test_bit_width_no_fixed_width() {
        // Void
        assert_eq!(CppType::Void.bit_width(), None);

        // Named types
        assert_eq!(CppType::Named("Foo".to_string()).bit_width(), None);

        // Template parameters
        let tp = CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        };
        assert_eq!(tp.bit_width(), None);
    }

    #[test]
    fn test_is_signed_integer_types() {
        // Signed types return Some(true)
        assert_eq!(CppType::Char { signed: true }.is_signed(), Some(true));
        assert_eq!(CppType::Short { signed: true }.is_signed(), Some(true));
        assert_eq!(CppType::Int { signed: true }.is_signed(), Some(true));
        assert_eq!(CppType::Long { signed: true }.is_signed(), Some(true));
        assert_eq!(CppType::LongLong { signed: true }.is_signed(), Some(true));

        // Unsigned types return Some(false)
        assert_eq!(CppType::Char { signed: false }.is_signed(), Some(false));
        assert_eq!(CppType::Short { signed: false }.is_signed(), Some(false));
        assert_eq!(CppType::Int { signed: false }.is_signed(), Some(false));
        assert_eq!(CppType::Long { signed: false }.is_signed(), Some(false));
        assert_eq!(CppType::LongLong { signed: false }.is_signed(), Some(false));

        // Bool is unsigned
        assert_eq!(CppType::Bool.is_signed(), Some(false));

        // Floating point is signed
        assert_eq!(CppType::Float.is_signed(), Some(true));
        assert_eq!(CppType::Double.is_signed(), Some(true));
    }

    #[test]
    fn test_smart_pointer_type_mappings() {
        // NOTE: Smart pointer mappings removed - types pass through as-is
        // See Section 22 in TODO.md for rationale
        // Template syntax converted to valid Rust identifiers

        // std::unique_ptr<T> passes through (no longer mapped to Box<T>)
        assert_eq!(
            CppType::Named("std::unique_ptr<int>".to_string()).to_rust_type_str(),
            "std_unique_ptr_int"
        );
        assert_eq!(
            CppType::Named("std::unique_ptr<int, std::default_delete<int>>".to_string())
                .to_rust_type_str(),
            "std_unique_ptr_int__std_default_delete_int"
        );
        assert_eq!(
            CppType::Named("std::unique_ptr<MyClass>".to_string()).to_rust_type_str(),
            "std_unique_ptr_MyClass"
        );

        // __detail::__unique_ptr_t<T> passes through
        assert_eq!(
            CppType::Named("__detail::__unique_ptr_t<int>".to_string()).to_rust_type_str(),
            "__detail___unique_ptr_t_int"
        );

        // std::shared_ptr<T> passes through (no longer mapped to Arc<T>)
        assert_eq!(
            CppType::Named("std::shared_ptr<int>".to_string()).to_rust_type_str(),
            "std_shared_ptr_int"
        );
        assert_eq!(
            CppType::Named("std::shared_ptr<MyClass>".to_string()).to_rust_type_str(),
            "std_shared_ptr_MyClass"
        );

        // shared_ptr<_NonArray<T>> passes through
        assert_eq!(
            CppType::Named("shared_ptr<_NonArray<int>>".to_string()).to_rust_type_str(),
            "shared_ptr__NonArray_int"
        );

        // std::weak_ptr<T> passes through (no longer mapped to Weak<T>)
        assert_eq!(
            CppType::Named("std::weak_ptr<int>".to_string()).to_rust_type_str(),
            "std_weak_ptr_int"
        );
        assert_eq!(
            CppType::Named("std::weak_ptr<MyClass>".to_string()).to_rust_type_str(),
            "std_weak_ptr_MyClass"
        );
    }

    #[test]
    fn test_std_array_type_mapping() {
        // NOTE: STL mappings removed - all types pass through as-is
        // See Section 22 in TODO.md for rationale

        // std::array passes through (no longer mapped to [T; N])
        // Template syntax converted to valid Rust identifiers
        assert_eq!(
            CppType::Named("std::array<int, 5>".to_string()).to_rust_type_str(),
            "std_array_int__5"
        );
        assert_eq!(
            CppType::Named("std::array<double, 10>".to_string()).to_rust_type_str(),
            "std_array_double__10"
        );

        // Nested template types also pass through
        assert_eq!(
            CppType::Named("std::array<std::vector<int>, 2>".to_string()).to_rust_type_str(),
            "std_array_std_vector_int__2"
        );
    }

    #[test]
    fn test_std_span_type_mapping() {
        // NOTE: STL mappings removed - all types pass through as-is
        // See Section 22 in TODO.md for rationale
        // Template syntax converted to valid Rust identifiers

        // std::span passes through (no longer mapped to &[T])
        assert_eq!(
            CppType::Named("std::span<int>".to_string()).to_rust_type_str(),
            "std_span_int"
        );
        assert_eq!(
            CppType::Named("std::span<const int>".to_string()).to_rust_type_str(),
            "std_span_const_int"
        );
        assert_eq!(
            CppType::Named("std::span<int, 10>".to_string()).to_rust_type_str(),
            "std_span_int__10"
        );
    }

    #[test]
    fn test_std_variant_type_mapping() {
        // NOTE: STL mappings removed - all types pass through as-is
        // See Section 22 in TODO.md for rationale
        // Template syntax converted to valid Rust identifiers

        // std::variant passes through (no longer mapped to Variant_...)
        assert_eq!(
            CppType::Named("std::variant<int, double>".to_string()).to_rust_type_str(),
            "std_variant_int__double"
        );
        assert_eq!(
            CppType::Named("std::variant<int, std::string>".to_string()).to_rust_type_str(),
            "std_variant_int__std_string"
        );
        assert_eq!(
            CppType::Named("std::variant<MyClass, OtherClass>".to_string()).to_rust_type_str(),
            "std_variant_MyClass__OtherClass"
        );
    }

    #[test]
    fn test_stream_type_mappings() {
        // NOTE: STL mappings removed - all types pass through as-is
        // See Section 22 in TODO.md for rationale

        // Stream types pass through (no longer mapped to Rust I/O types)
        assert_eq!(
            CppType::Named("std::ostream".to_string()).to_rust_type_str(),
            "std_ostream"
        );
        assert_eq!(
            CppType::Named("std::istream".to_string()).to_rust_type_str(),
            "std_istream"
        );
        assert_eq!(
            CppType::Named("std::iostream".to_string()).to_rust_type_str(),
            "std_iostream"
        );
        assert_eq!(
            CppType::Named("std::stringstream".to_string()).to_rust_type_str(),
            "std_stringstream"
        );
        assert_eq!(
            CppType::Named("std::ofstream".to_string()).to_rust_type_str(),
            "std_ofstream"
        );
        assert_eq!(
            CppType::Named("std::ifstream".to_string()).to_rust_type_str(),
            "std_ifstream"
        );
        assert_eq!(
            CppType::Named("std::fstream".to_string()).to_rust_type_str(),
            "std_fstream"
        );
    }

    #[test]
    fn test_inline_namespace_stripping() {
        // libc++ uses inline namespaces like std::__1:: for ABI versioning
        // These should be stripped to produce cleaner type names

        // std::__1::vector<int> -> std_vector_int
        assert_eq!(
            CppType::Named("std::__1::vector<int>".to_string()).to_rust_type_str(),
            "std_vector_int"
        );

        // std::__1::string -> std_string
        assert_eq!(
            CppType::Named("std::__1::string".to_string()).to_rust_type_str(),
            "std_string"
        );

        // std::__1::basic_string<char> -> std_basic_string_char
        assert_eq!(
            CppType::Named("std::__1::basic_string<char>".to_string()).to_rust_type_str(),
            "std_basic_string_char"
        );

        // Nested inline namespaces: std::__1::__detail::__helper -> std___detail___helper
        assert_eq!(
            CppType::Named("std::__1::__detail::__helper".to_string()).to_rust_type_str(),
            "std___detail___helper"
        );

        // std::__2:: (alternative version) should also be stripped
        assert_eq!(
            CppType::Named("std::__2::vector<int>".to_string()).to_rust_type_str(),
            "std_vector_int"
        );

        // Android NDK uses __ndk1
        assert_eq!(
            CppType::Named("std::__ndk1::vector<int>".to_string()).to_rust_type_str(),
            "std_vector_int"
        );
    }

    #[test]
    fn test_parse_template_args() {
        // Basic arguments
        assert_eq!(parse_template_args("int, double"), vec!["int", "double"]);

        // Single argument
        assert_eq!(parse_template_args("int"), vec!["int"]);

        // With nested templates
        assert_eq!(
            parse_template_args("int, std::vector<int>, double"),
            vec!["int", "std::vector<int>", "double"]
        );

        // Deeply nested
        assert_eq!(
            parse_template_args("std::map<int, std::vector<double>>, bool"),
            vec!["std::map<int, std::vector<double>>", "bool"]
        );

        // With whitespace
        assert_eq!(
            parse_template_args("  int  ,  double  "),
            vec!["int", "double"]
        );

        // Empty
        assert_eq!(parse_template_args(""), Vec::<String>::new());
    }
}
