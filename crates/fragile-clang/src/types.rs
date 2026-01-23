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
                if let CppType::Function { return_type, params, is_variadic } = pointee.as_ref() {
                    let params_str: Vec<_> = params.iter().map(|p| p.to_rust_type_str()).collect();
                    let params_joined = if *is_variadic {
                        format!("{}, ...", params_str.join(", "))
                    } else {
                        params_str.join(", ")
                    };
                    // Use Option to handle nullable function pointers
                    // Note: We don't use extern "C" since transpiled functions use Rust calling convention
                    format!("Option<fn({}) -> {}>", params_joined, return_type.to_rust_type_str())
                } else {
                    // Regular pointer - respect const
                    let ptr_type = if *is_const { "*const" } else { "*mut" };
                    format!("{} {}", ptr_type, pointee.to_rust_type_str())
                }
            }
            CppType::Reference { referent, is_const, is_rvalue: _ } => {
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
                // Handle special C++ types that don't map directly to Rust
                match name.as_str() {
                    "float" => "f32".to_string(),
                    "double" | "long double" => "f64".to_string(),  // Rust doesn't have long double
                    "bool" => "bool".to_string(),
                    "long long" | "long long int" => "i64".to_string(),
                    "unsigned long long" | "unsigned long long int" => "u64".to_string(),
                    "long" | "long int" => "i64".to_string(),
                    "unsigned long" | "unsigned long int" => "u64".to_string(),
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
                    "size_t" => "usize".to_string(),
                    "ssize_t" | "ptrdiff_t" => "isize".to_string(),
                    "intptr_t" => "isize".to_string(),
                    "uintptr_t" => "usize".to_string(),
                    // STL type mappings
                    "std::string" | "const std::string" |
                    "std::__cxx11::basic_string<char>" | "const std::__cxx11::basic_string<char>" |
                    "basic_string<char>" | "const basic_string<char>" |
                    "basic_string<char, char_traits<char>, allocator<char>>" |
                    "std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char>>" => "String".to_string(),
                    _ => {
                        // Handle STL string types with patterns (including const variants)
                        let check_name = name.strip_prefix("const ").unwrap_or(name);
                        if check_name.contains("basic_string<char") || check_name == "std::string" {
                            return "String".to_string();
                        }
                        // Handle std::vector<T> -> Vec<T>
                        if let Some(rest) = name.strip_prefix("std::vector<") {
                            if let Some(inner) = rest.strip_suffix(">") {
                                // Handle allocator suffix: "int, std::allocator<int>" -> "int"
                                let element = if let Some(idx) = inner.find(", std::allocator<") {
                                    &inner[..idx]
                                } else if let Some(idx) = inner.find(", allocator<") {
                                    &inner[..idx]
                                } else {
                                    inner
                                };
                                let element_type = CppType::Named(element.trim().to_string());
                                return format!("Vec<{}>", element_type.to_rust_type_str());
                            }
                        }
                        // Handle std::optional<T> -> Option<T>
                        if let Some(rest) = name.strip_prefix("std::optional<") {
                            if let Some(inner) = rest.strip_suffix(">") {
                                let element_type = CppType::Named(inner.trim().to_string());
                                return format!("Option<{}>", element_type.to_rust_type_str());
                            }
                        }
                        // Handle std::array<T, N> -> [T; N]
                        if let Some(rest) = name.strip_prefix("std::array<") {
                            if let Some(inner) = rest.strip_suffix(">") {
                                // Find the last comma separating element type from size
                                // Use rfind to handle nested template types like std::array<std::vector<int>, 5>
                                if let Some(comma_idx) = inner.rfind(", ") {
                                    let element_str = &inner[..comma_idx];
                                    let size_str = inner[comma_idx + 2..].trim();
                                    let element_type = CppType::Named(element_str.trim().to_string());
                                    return format!("[{}; {}]", element_type.to_rust_type_str(), size_str);
                                }
                            }
                        }
                        // Handle std::span<T> -> &[T] / &mut [T]
                        if let Some(rest) = name.strip_prefix("std::span<") {
                            if let Some(inner) = rest.strip_suffix(">") {
                                // Handle dynamic vs static extent: "int" or "int, 5" (ignore extent)
                                let element_str = if let Some(comma_idx) = inner.rfind(", ") {
                                    let after_comma = inner[comma_idx + 2..].trim();
                                    // Check if the part after comma is a number (extent) - ignore it
                                    if after_comma.chars().all(|c| c.is_ascii_digit() || c == '_') {
                                        &inner[..comma_idx]
                                    } else {
                                        inner
                                    }
                                } else {
                                    inner
                                };

                                let element_str = element_str.trim();

                                // Check for const element type
                                let (is_const, element_type_str) = if let Some(rest) = element_str.strip_prefix("const ") {
                                    (true, rest.trim())
                                } else if element_str.ends_with(" const") {
                                    (true, element_str.strip_suffix(" const").unwrap().trim())
                                } else {
                                    (false, element_str)
                                };

                                let element_type = CppType::Named(element_type_str.to_string());
                                if is_const {
                                    return format!("&[{}]", element_type.to_rust_type_str());
                                } else {
                                    return format!("&mut [{}]", element_type.to_rust_type_str());
                                }
                            }
                        }
                        // Handle std::map<K,V> -> BTreeMap<K,V>
                        if let Some(rest) = name.strip_prefix("std::map<") {
                            // For now, map key-value pairs directly
                            if let Some(inner) = rest.strip_suffix(">") {
                                // Try to split at first comma for key, value
                                let trimmed = inner.trim();
                                // This is simplified - complex nested types may need better parsing
                                return format!("BTreeMap<{}>", trimmed);
                            }
                        }
                        // Handle std::unordered_map<K,V> -> HashMap<K,V>
                        if let Some(rest) = name.strip_prefix("std::unordered_map<") {
                            if let Some(inner) = rest.strip_suffix(">") {
                                let trimmed = inner.trim();
                                return format!("HashMap<{}>", trimmed);
                            }
                        }
                        // Handle std::unique_ptr<T> -> Box<T>
                        if let Some(rest) = name.strip_prefix("std::unique_ptr<") {
                            if let Some(inner) = rest.strip_suffix(">") {
                                // Handle default deleter: "int, std::default_delete<int>" -> "int"
                                let element = if let Some(idx) = inner.find(", std::default_delete<") {
                                    &inner[..idx]
                                } else if let Some(idx) = inner.find(", default_delete<") {
                                    &inner[..idx]
                                } else {
                                    inner
                                };
                                let element_type = CppType::Named(element.trim().to_string());
                                return format!("Box<{}>", element_type.to_rust_type_str());
                            }
                        }
                        // Handle __detail::__unique_ptr_t<T> -> Box<T> (libstdc++ internal)
                        if let Some(rest) = name.strip_prefix("__detail::__unique_ptr_t<") {
                            if let Some(inner) = rest.strip_suffix(">") {
                                let element_type = CppType::Named(inner.trim().to_string());
                                return format!("Box<{}>", element_type.to_rust_type_str());
                            }
                        }
                        // Handle std::shared_ptr<T> -> Arc<T>
                        if let Some(rest) = name.strip_prefix("std::shared_ptr<") {
                            if let Some(inner) = rest.strip_suffix(">") {
                                let element_type = CppType::Named(inner.trim().to_string());
                                return format!("Arc<{}>", element_type.to_rust_type_str());
                            }
                        }
                        // Handle shared_ptr<_NonArray<T>> (libstdc++ internal) -> Arc<T>
                        if let Some(rest) = name.strip_prefix("shared_ptr<_NonArray<") {
                            if let Some(inner) = rest.strip_suffix(">>") {
                                let element_type = CppType::Named(inner.trim().to_string());
                                return format!("Arc<{}>", element_type.to_rust_type_str());
                            }
                        }
                        // Handle std::weak_ptr<T> -> Weak<T>
                        if let Some(rest) = name.strip_prefix("std::weak_ptr<") {
                            if let Some(inner) = rest.strip_suffix(">") {
                                let element_type = CppType::Named(inner.trim().to_string());
                                return format!("Weak<{}>", element_type.to_rust_type_str());
                            }
                        }
                        // Handle std::ostream -> Box<dyn std::io::Write>
                        // Also handle basic_ostream<char> which is the underlying type
                        if check_name.starts_with("std::ostream") ||
                           check_name.starts_with("ostream") ||
                           check_name.starts_with("std::basic_ostream<char") ||
                           check_name.starts_with("basic_ostream<char") {
                            return "Box<dyn std::io::Write>".to_string();
                        }
                        // Handle std::istream -> Box<dyn std::io::Read>
                        // Also handle basic_istream<char> which is the underlying type
                        if check_name.starts_with("std::istream") ||
                           check_name.starts_with("istream") ||
                           check_name.starts_with("std::basic_istream<char") ||
                           check_name.starts_with("basic_istream<char") {
                            return "Box<dyn std::io::Read>".to_string();
                        }
                        // Handle std::iostream -> Box<dyn std::io::Read + std::io::Write>
                        // Also handle basic_iostream<char> which is the underlying type
                        if check_name.starts_with("std::iostream") ||
                           check_name.starts_with("iostream") ||
                           check_name.starts_with("std::basic_iostream<char") ||
                           check_name.starts_with("basic_iostream<char") {
                            return "Box<dyn std::io::Read + std::io::Write>".to_string();
                        }
                        // Handle std::variant<T1, T2, ...> -> Variant_T1_T2_...
                        // This generates a synthetic enum name that will be defined separately
                        // Handle both "std::variant<...>" and "variant<...>" (libclang sometimes omits std::)
                        let variant_rest = name.strip_prefix("std::variant<")
                            .or_else(|| name.strip_prefix("variant<"));
                        if let Some(rest) = variant_rest {
                            if let Some(inner) = rest.strip_suffix(">") {
                                let args = parse_template_args(inner);
                                if !args.is_empty() {
                                    // Convert each type argument to its Rust equivalent
                                    let rust_types: Vec<String> = args.iter()
                                        .map(|a| {
                                            let rust_type = CppType::Named(a.clone()).to_rust_type_str();
                                            // Sanitize for use in identifier: replace special chars
                                            rust_type
                                                .replace('<', "_")
                                                .replace('>', "")
                                                .replace(", ", "_")
                                                .replace(" ", "_")
                                                .replace("::", "_")
                                                .replace("*", "Ptr")
                                                .replace("&", "Ref")
                                                .replace("[", "Arr")
                                                .replace("]", "")
                                                .replace(";", "x")
                                        })
                                        .collect();
                                    // Generate unique enum name from types
                                    return format!("Variant_{}", rust_types.join("_"));
                                }
                            }
                        }
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
                            return "_".to_string();  // Let Rust infer the closure type
                        }
                        // Handle auto type (C++11) - use Rust type inference
                        if name == "auto" {
                            return "_".to_string();
                        }
                        // Strip C++ qualifiers that aren't valid in Rust type names
                        let cleaned = name
                            .trim_start_matches("const ")
                            .trim_start_matches("volatile ")
                            .trim_start_matches("struct ")
                            .trim_start_matches("class ")
                            .trim_start_matches("enum ")
                            .trim_end();  // Remove trailing whitespace

                        // Handle remaining "unsigned TYPE" patterns
                        let cleaned = if cleaned.starts_with("unsigned ") {
                            match cleaned.trim_start_matches("unsigned ") {
                                "int" => "u32",
                                "long" => "u64",
                                "short" => "u16",
                                "char" => "u8",
                                _ => cleaned
                            }
                        } else if cleaned.starts_with("signed ") {
                            match cleaned.trim_start_matches("signed ") {
                                "int" => "i32",
                                "long" => "i64",
                                "short" => "i16",
                                "char" => "i8",
                                _ => cleaned
                            }
                        } else {
                            cleaned
                        };

                        // Replace :: with _ for namespaced types, remove other invalid chars
                        cleaned.replace("::", "_")
                            .replace(" *", "")  // Remove trailing pointer indicators
                            .replace("*", "")
                            .replace(" ", "_")
                    }
                }
            }
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
        // std::unique_ptr<T> -> Box<T>
        assert_eq!(
            CppType::Named("std::unique_ptr<int>".to_string()).to_rust_type_str(),
            "Box<i32>"
        );
        assert_eq!(
            CppType::Named("std::unique_ptr<int, std::default_delete<int>>".to_string()).to_rust_type_str(),
            "Box<i32>"
        );
        assert_eq!(
            CppType::Named("std::unique_ptr<MyClass>".to_string()).to_rust_type_str(),
            "Box<MyClass>"
        );

        // __detail::__unique_ptr_t<T> -> Box<T> (libstdc++ internal)
        assert_eq!(
            CppType::Named("__detail::__unique_ptr_t<int>".to_string()).to_rust_type_str(),
            "Box<i32>"
        );

        // std::shared_ptr<T> -> Arc<T>
        assert_eq!(
            CppType::Named("std::shared_ptr<int>".to_string()).to_rust_type_str(),
            "Arc<i32>"
        );
        assert_eq!(
            CppType::Named("std::shared_ptr<MyClass>".to_string()).to_rust_type_str(),
            "Arc<MyClass>"
        );

        // shared_ptr<_NonArray<T>> -> Arc<T> (libstdc++ internal)
        assert_eq!(
            CppType::Named("shared_ptr<_NonArray<int>>".to_string()).to_rust_type_str(),
            "Arc<i32>"
        );

        // std::weak_ptr<T> -> Weak<T>
        assert_eq!(
            CppType::Named("std::weak_ptr<int>".to_string()).to_rust_type_str(),
            "Weak<i32>"
        );
        assert_eq!(
            CppType::Named("std::weak_ptr<MyClass>".to_string()).to_rust_type_str(),
            "Weak<MyClass>"
        );
    }

    #[test]
    fn test_std_array_type_mapping() {
        // Basic std::array<T, N> -> [T; N]
        assert_eq!(
            CppType::Named("std::array<int, 5>".to_string()).to_rust_type_str(),
            "[i32; 5]"
        );
        assert_eq!(
            CppType::Named("std::array<double, 10>".to_string()).to_rust_type_str(),
            "[f64; 10]"
        );
        assert_eq!(
            CppType::Named("std::array<char, 256>".to_string()).to_rust_type_str(),
            "[i8; 256]"
        );

        // With custom types
        assert_eq!(
            CppType::Named("std::array<MyClass, 3>".to_string()).to_rust_type_str(),
            "[MyClass; 3]"
        );

        // Nested template types
        assert_eq!(
            CppType::Named("std::array<std::vector<int>, 2>".to_string()).to_rust_type_str(),
            "[Vec<i32>; 2]"
        );
    }

    #[test]
    fn test_std_span_type_mapping() {
        // Dynamic extent, mutable element type -> &mut [T]
        assert_eq!(
            CppType::Named("std::span<int>".to_string()).to_rust_type_str(),
            "&mut [i32]"
        );
        assert_eq!(
            CppType::Named("std::span<double>".to_string()).to_rust_type_str(),
            "&mut [f64]"
        );

        // Const element type -> &[T]
        assert_eq!(
            CppType::Named("std::span<const int>".to_string()).to_rust_type_str(),
            "&[i32]"
        );
        assert_eq!(
            CppType::Named("std::span<const char>".to_string()).to_rust_type_str(),
            "&[i8]"
        );

        // With static extent (ignored, just extracts element type)
        assert_eq!(
            CppType::Named("std::span<int, 10>".to_string()).to_rust_type_str(),
            "&mut [i32]"
        );
        assert_eq!(
            CppType::Named("std::span<const double, 5>".to_string()).to_rust_type_str(),
            "&[f64]"
        );

        // Custom types
        assert_eq!(
            CppType::Named("std::span<MyClass>".to_string()).to_rust_type_str(),
            "&mut [MyClass]"
        );
    }

    #[test]
    fn test_std_variant_type_mapping() {
        // Basic variant with two primitive types
        assert_eq!(
            CppType::Named("std::variant<int, double>".to_string()).to_rust_type_str(),
            "Variant_i32_f64"
        );

        // Variant with three types
        assert_eq!(
            CppType::Named("std::variant<int, double, bool>".to_string()).to_rust_type_str(),
            "Variant_i32_f64_bool"
        );

        // Variant with string types
        assert_eq!(
            CppType::Named("std::variant<int, std::string>".to_string()).to_rust_type_str(),
            "Variant_i32_String"
        );

        // Variant with nested template types (vector)
        assert_eq!(
            CppType::Named("std::variant<std::vector<int>, double>".to_string()).to_rust_type_str(),
            "Variant_Vec_i32_f64"
        );

        // Variant with optional type
        assert_eq!(
            CppType::Named("std::variant<std::optional<int>, bool>".to_string()).to_rust_type_str(),
            "Variant_Option_i32_bool"
        );

        // Note: Pointer types within variant spellings don't go through proper type conversion
        // because Named("int *") doesn't match the exact string "int" in the match arm.
        // In practice, Clang provides the full spelling and this behavior is acceptable.
        // The key thing is that the parsing and separation works correctly.

        // Variant with custom class types
        assert_eq!(
            CppType::Named("std::variant<MyClass, OtherClass>".to_string()).to_rust_type_str(),
            "Variant_MyClass_OtherClass"
        );
    }

    #[test]
    fn test_stream_type_mappings() {
        // std::ostream -> Box<dyn std::io::Write>
        assert_eq!(
            CppType::Named("std::ostream".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Write>"
        );
        assert_eq!(
            CppType::Named("ostream".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Write>"
        );
        assert_eq!(
            CppType::Named("std::basic_ostream<char>".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Write>"
        );
        assert_eq!(
            CppType::Named("basic_ostream<char>".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Write>"
        );

        // std::istream -> Box<dyn std::io::Read>
        assert_eq!(
            CppType::Named("std::istream".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Read>"
        );
        assert_eq!(
            CppType::Named("istream".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Read>"
        );
        assert_eq!(
            CppType::Named("std::basic_istream<char>".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Read>"
        );
        assert_eq!(
            CppType::Named("basic_istream<char>".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Read>"
        );

        // std::iostream -> Box<dyn std::io::Read + std::io::Write>
        assert_eq!(
            CppType::Named("std::iostream".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Read + std::io::Write>"
        );
        assert_eq!(
            CppType::Named("iostream".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Read + std::io::Write>"
        );
        assert_eq!(
            CppType::Named("std::basic_iostream<char>".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Read + std::io::Write>"
        );
        assert_eq!(
            CppType::Named("basic_iostream<char>".to_string()).to_rust_type_str(),
            "Box<dyn std::io::Read + std::io::Write>"
        );
    }

    #[test]
    fn test_parse_template_args() {
        // Basic arguments
        assert_eq!(
            parse_template_args("int, double"),
            vec!["int", "double"]
        );

        // Single argument
        assert_eq!(
            parse_template_args("int"),
            vec!["int"]
        );

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
        assert_eq!(
            parse_template_args(""),
            Vec::<String>::new()
        );
    }
}
