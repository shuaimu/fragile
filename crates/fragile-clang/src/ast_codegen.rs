//! Direct AST to Rust source code generation.
//!
//! This module generates Rust source code directly from the Clang AST,
//! without going through an intermediate MIR representation.
//! This produces cleaner, more idiomatic Rust code.

use crate::ast::{ClangNode, ClangNodeKind, BinaryOp, UnaryOp, CastKind, ConstructorKind, CoroutineInfo, CoroutineKind, AccessSpecifier};
use crate::types::{CppType, parse_template_args};
use std::collections::{HashSet, HashMap};

/// Convert C++ access specifier to Rust visibility prefix.
/// - Public → "pub "
/// - Protected → "pub(crate) " (accessible within crate, roughly matches protected semantics)
/// - Private → "" (no visibility prefix, private by default in Rust)
fn access_to_visibility(access: AccessSpecifier) -> &'static str {
    match access {
        AccessSpecifier::Public => "pub ",
        AccessSpecifier::Protected => "pub(crate) ",
        AccessSpecifier::Private => "",
    }
}

/// Rust reserved keywords that need raw identifier syntax.
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn",
    "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
    "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
    "abstract", "become", "box", "do", "final", "macro", "override",
    "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Information about a virtual method for trait generation.
#[derive(Clone)]
struct VirtualMethodInfo {
    name: String,
    return_type: CppType,
    params: Vec<(String, CppType)>,
    #[allow(dead_code)] // Reserved for future use (const vs mutable self)
    is_const: bool,
}

#[derive(Clone)]
struct BaseInfo {
    name: String,
    is_virtual: bool,
}

enum BaseAccess {
    DirectField(String),
    FieldChain(String),
    VirtualPtr(String),
}

/// Information about a single bit field within a packed group.
#[derive(Clone, Debug)]
struct BitFieldInfo {
    /// Name of the original field
    field_name: String,
    /// Original type (for return type in accessor)
    original_type: CppType,
    /// Width in bits
    width: u32,
    /// Offset within the storage unit (in bits)
    offset: u32,
    /// Access specifier
    access: AccessSpecifier,
}

/// A group of consecutive bit fields packed into a single storage unit.
#[derive(Clone, Debug)]
struct BitFieldGroup {
    /// Fields in this group
    fields: Vec<BitFieldInfo>,
    /// Total bits used
    total_bits: u32,
    /// Index of this group (for generating _bitfield_0, _bitfield_1, etc.)
    group_index: usize,
}

impl BitFieldGroup {
    /// Get the smallest Rust unsigned integer type that can hold all bits.
    fn storage_type(&self) -> &'static str {
        match self.total_bits {
            0..=8 => "u8",
            9..=16 => "u16",
            17..=32 => "u32",
            33..=64 => "u64",
            _ => "u128",
        }
    }
}

/// Rust code generator that works directly with Clang AST.
pub struct AstCodeGen {
    output: String,
    indent: usize,
    /// Track variable names that are declared as reference types
    ref_vars: HashSet<String>,
    /// Track variable names that are declared as pointer types
    ptr_vars: HashSet<String>,
    /// Track variable names that are declared as array types
    arr_vars: HashSet<String>,
    /// When true, skip type suffixes for numeric literals (e.g., 5 instead of 5i32)
    skip_literal_suffix: bool,
    /// Current class being generated (for inherited member access)
    current_class: Option<String>,
    /// Classes that have virtual methods (need trait generation)
    polymorphic_classes: HashSet<String>,
    /// Map from class name to its base class names (supports multiple inheritance)
    class_bases: HashMap<String, Vec<BaseInfo>>,
    /// Map from class name to its transitive virtual bases
    virtual_bases: HashMap<String, Vec<String>>,
    /// Map from class name to its virtual methods
    virtual_methods: HashMap<String, Vec<VirtualMethodInfo>>,
    /// Map from (class_name, member_name) to global variable name for static members
    static_members: HashMap<(String, String), String>,
    /// Track global variable names (require unsafe access)
    global_vars: HashSet<String>,
    /// Current namespace path during code generation (for relative path computation)
    current_namespace: Vec<String>,
    /// When true, use __self instead of self for this expressions
    use_ctor_self: bool,
    /// Current method return type (for reference return handling)
    current_return_type: Option<CppType>,
    /// Map from class name to its field names (for constructor generation)
    class_fields: HashMap<String, Vec<(String, CppType)>>,
    /// Collected std::variant types: maps enum name (e.g., "Variant_i32_f64") to its Rust type arguments (e.g., ["i32", "f64"])
    variant_types: HashMap<String, Vec<String>>,
    /// Counter for generating unique anonymous namespace names
    anon_namespace_counter: usize,
    /// Track already generated struct names to avoid duplicates from template instantiation
    generated_structs: HashSet<String>,
    /// Map from class/struct name to its bit field groups
    bit_field_groups: HashMap<String, Vec<BitFieldGroup>>,
}

impl AstCodeGen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            ref_vars: HashSet::new(),
            ptr_vars: HashSet::new(),
            arr_vars: HashSet::new(),
            skip_literal_suffix: false,
            current_class: None,
            polymorphic_classes: HashSet::new(),
            class_bases: HashMap::new(),
            virtual_bases: HashMap::new(),
            virtual_methods: HashMap::new(),
            static_members: HashMap::new(),
            global_vars: HashSet::new(),
            current_namespace: Vec::new(),
            use_ctor_self: false,
            current_return_type: None,
            class_fields: HashMap::new(),
            variant_types: HashMap::new(),
            anon_namespace_counter: 0,
            generated_structs: HashSet::new(),
            bit_field_groups: HashMap::new(),
        }
    }

    /// Generate Rust source code from a Clang AST.
    pub fn generate(mut self, ast: &ClangNode) -> String {
        // First pass: collect polymorphic class information
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            self.collect_polymorphic_info(&ast.children);
        }
        self.compute_virtual_bases();

        // Collect std::variant types used in the code
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            self.collect_variant_types(&ast.children);
        }

        // File header
        self.writeln("#![allow(dead_code)]");
        self.writeln("#![allow(unused_variables)]");
        self.writeln("#![allow(unused_mut)]");
        self.writeln("#![allow(non_camel_case_types)]");
        self.writeln("#![allow(non_snake_case)]");
        self.writeln("");
        self.write_array_helpers();

        // Generate synthetic enum definitions for std::variant types
        self.generate_variant_enums();

        // Second pass: generate code
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            for child in &ast.children {
                self.generate_top_level(child);
            }
        }

        self.output
    }

    /// First pass: collect information about polymorphic classes.
    fn collect_polymorphic_info(&mut self, children: &[ClangNode]) {
        for child in children {
            match &child.kind {
                ClangNodeKind::RecordDecl { name, .. } => {
                    self.analyze_class(name, &child.children);
                }
                ClangNodeKind::NamespaceDecl { .. } => {
                    // Recurse into namespaces
                    self.collect_polymorphic_info(&child.children);
                }
                _ => {}
            }
        }
    }

    /// Analyze a class for virtual methods and inheritance.
    fn analyze_class(&mut self, class_name: &str, children: &[ClangNode]) {
        let mut virtual_methods = Vec::new();
        let mut base_classes: Vec<BaseInfo> = Vec::new();

        for child in children {
            match &child.kind {
                ClangNodeKind::CXXMethodDecl {
                    name, return_type, params, is_virtual, ..
                } => {
                    if *is_virtual {
                        virtual_methods.push(VirtualMethodInfo {
                            name: name.clone(),
                            return_type: return_type.clone(),
                            params: params.clone(),
                            is_const: false, // TODO: track const-ness
                        });
                    }
                }
                ClangNodeKind::CXXBaseSpecifier { base_type, is_virtual, .. } => {
                    // Extract base class name - collect ALL bases for MI
                    if let CppType::Named(base_name) = base_type {
                        let base_name = base_name.strip_prefix("const ").unwrap_or(base_name).to_string();
                        base_classes.push(BaseInfo { name: base_name, is_virtual: *is_virtual });
                    }
                }
                _ => {}
            }
        }

        // If this class has virtual methods, mark it as polymorphic
        if !virtual_methods.is_empty() {
            self.polymorphic_classes.insert(class_name.to_string());
            self.virtual_methods.insert(class_name.to_string(), virtual_methods);
        }

        // Record inheritance relationships (supports multiple bases)
        if !base_classes.is_empty() {
            // If any base class is polymorphic, this class is too
            for base in &base_classes {
                if self.polymorphic_classes.contains(&base.name) {
                    self.polymorphic_classes.insert(class_name.to_string());
                    break;
                }
            }
            self.class_bases.insert(class_name.to_string(), base_classes);
        }
    }

    fn compute_virtual_bases(&mut self) {
        let class_names: Vec<String> = self.class_bases.keys().cloned().collect();
        for class_name in class_names {
            let mut set = HashSet::new();
            let mut visiting = HashSet::new();
            self.collect_virtual_bases(&class_name, &mut set, &mut visiting);
            if !set.is_empty() {
                let mut list: Vec<String> = set.into_iter().collect();
                list.sort();
                self.virtual_bases.insert(class_name, list);
            }
        }
    }

    fn collect_virtual_bases(&self, class_name: &str, out: &mut HashSet<String>, visiting: &mut HashSet<String>) {
        if visiting.contains(class_name) {
            return;
        }
        visiting.insert(class_name.to_string());
        if let Some(bases) = self.class_bases.get(class_name) {
            for base in bases {
                if base.is_virtual {
                    out.insert(base.name.clone());
                }
                self.collect_virtual_bases(&base.name, out, visiting);
                if let Some(vb) = self.virtual_bases.get(&base.name) {
                    out.extend(vb.iter().cloned());
                }
            }
        }
        visiting.remove(class_name);
    }

    fn virtual_base_field_name(&self, base_name: &str) -> String {
        let sanitized = base_name.replace("::", "_");
        format!("__vbase_{}", sanitize_identifier(&sanitized))
    }

    fn virtual_base_storage_field_name(&self, base_name: &str) -> String {
        let sanitized = base_name.replace("::", "_");
        format!("__vbase_storage_{}", sanitize_identifier(&sanitized))
    }

    fn class_has_virtual_bases(&self, class_name: &str) -> bool {
        self.virtual_bases.get(class_name).map_or(false, |v| !v.is_empty())
    }

    /// Collect all std::variant types used in the code.
    fn collect_variant_types(&mut self, children: &[ClangNode]) {
        for child in children {
            match &child.kind {
                ClangNodeKind::VarDecl { ty, .. } => {
                    self.collect_variant_from_type(ty);
                }
                ClangNodeKind::FieldDecl { ty, .. } => {
                    self.collect_variant_from_type(ty);
                }
                ClangNodeKind::FunctionDecl { return_type, params, .. } => {
                    self.collect_variant_from_type(return_type);
                    for (_, param_ty) in params {
                        self.collect_variant_from_type(param_ty);
                    }
                    // Recurse into function body
                    self.collect_variant_types(&child.children);
                }
                ClangNodeKind::CXXMethodDecl { return_type, params, .. } => {
                    self.collect_variant_from_type(return_type);
                    for (_, param_ty) in params {
                        self.collect_variant_from_type(param_ty);
                    }
                    // Recurse into method body
                    self.collect_variant_types(&child.children);
                }
                ClangNodeKind::RecordDecl { .. } |
                ClangNodeKind::NamespaceDecl { .. } => {
                    self.collect_variant_types(&child.children);
                }
                ClangNodeKind::CompoundStmt => {
                    self.collect_variant_types(&child.children);
                }
                _ => {
                    // Recurse into other nodes that might contain declarations
                    self.collect_variant_types(&child.children);
                }
            }
        }
    }

    /// Check if a type is std::variant and if so, record it.
    fn collect_variant_from_type(&mut self, ty: &CppType) {
        if let CppType::Named(name) = ty {
            if let Some(rest) = name.strip_prefix("std::variant<") {
                if let Some(inner) = rest.strip_suffix(">") {
                    // Parse the template arguments
                    let args = parse_template_args(inner);
                    if !args.is_empty() {
                        // Convert each C++ type to its Rust equivalent
                        let rust_types: Vec<String> = args.iter()
                            .map(|a| CppType::Named(a.clone()).to_rust_type_str())
                            .collect();

                        // Generate the enum name (same logic as in types.rs)
                        let sanitized_types: Vec<String> = rust_types.iter()
                            .map(|t| {
                                t.replace('<', "_")
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
                        let enum_name = format!("Variant_{}", sanitized_types.join("_"));

                        // Store if not already recorded
                        if !self.variant_types.contains_key(&enum_name) {
                            self.variant_types.insert(enum_name, rust_types);
                        }
                    }
                }
            }
        }
        // Also check inside pointer/reference/array types
        match ty {
            CppType::Pointer { pointee, .. } => self.collect_variant_from_type(pointee),
            CppType::Reference { referent, .. } => self.collect_variant_from_type(referent),
            CppType::Array { element, .. } => self.collect_variant_from_type(element),
            _ => {}
        }
    }

    /// Map C++ compiler builtin functions to Rust equivalents.
    /// Returns Some((rust_code, needs_unsafe)) if the function is a builtin,
    /// where rust_code is the generated Rust code and needs_unsafe indicates if
    /// it should be wrapped in `unsafe {}`.
    fn map_builtin_function(func_name: &str, args: &[String]) -> Option<(String, bool)> {
        match func_name {
            // __builtin_is_constant_evaluated() is always false at runtime
            // (Clang evaluates constexpr at compile time, so runtime code sees false)
            "__builtin_is_constant_evaluated" => Some(("false".to_string(), false)),

            // Memory operations - map to std::ptr functions
            "__builtin_memcpy" => {
                // __builtin_memcpy(dst, src, n) -> std::ptr::copy_nonoverlapping(src, dst, n)
                if args.len() >= 3 {
                    // Note: memcpy copies n bytes, copy_nonoverlapping copies n elements
                    // We cast to u8 pointers to copy bytes
                    Some((format!(
                        "std::ptr::copy_nonoverlapping({} as *const u8, {} as *mut u8, {})",
                        args[1], args[0], args[2]
                    ), true))
                } else {
                    None
                }
            }
            "__builtin_memmove" => {
                // __builtin_memmove(dst, src, n) -> std::ptr::copy(src, dst, n)
                if args.len() >= 3 {
                    Some((format!(
                        "std::ptr::copy({} as *const u8, {} as *mut u8, {})",
                        args[1], args[0], args[2]
                    ), true))
                } else {
                    None
                }
            }
            "__builtin_memset" => {
                // __builtin_memset(dst, val, n) -> std::ptr::write_bytes(dst, val, n)
                if args.len() >= 3 {
                    Some((format!(
                        "std::ptr::write_bytes({} as *mut u8, {} as u8, {})",
                        args[0], args[1], args[2]
                    ), true))
                } else {
                    None
                }
            }
            "__builtin_memcmp" => {
                // __builtin_memcmp(s1, s2, n) -> compare n bytes
                // Rust doesn't have a direct equivalent, use libc or slice comparison
                if args.len() >= 3 {
                    Some((format!(
                        "{{ let s1 = std::slice::from_raw_parts({} as *const u8, {}); \
                         let s2 = std::slice::from_raw_parts({} as *const u8, {}); \
                         s1.cmp(s2) as i32 }}",
                        args[0], args[2], args[1], args[2]
                    ), true))
                } else {
                    None
                }
            }
            "__builtin_strlen" => {
                // __builtin_strlen(s) -> strlen equivalent
                if args.len() >= 1 {
                    Some((format!(
                        "{{ let mut __len = 0usize; let mut __p = {} as *const u8; \
                         while *__p != 0 {{ __len += 1; __p = __p.add(1); }} __len }}",
                        args[0]
                    ), true))
                } else {
                    None
                }
            }
            "__builtin_expect" => {
                // __builtin_expect(exp, c) -> exp (hint for branch prediction, just return exp)
                if args.len() >= 1 {
                    Some((args[0].clone(), false))
                } else {
                    None
                }
            }
            "__builtin_unreachable" => {
                // __builtin_unreachable() -> std::hint::unreachable_unchecked()
                Some(("std::hint::unreachable_unchecked()".to_string(), true))
            }
            "__builtin_trap" => {
                // __builtin_trap() -> std::intrinsics::abort() or panic
                Some(("std::process::abort()".to_string(), false))
            }
            "__builtin_abort" => {
                Some(("std::process::abort()".to_string(), false))
            }
            "__builtin_clz" | "__builtin_clzl" | "__builtin_clzll" => {
                // Count leading zeros
                if args.len() >= 1 {
                    Some((format!("({}).leading_zeros() as i32", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_ctz" | "__builtin_ctzl" | "__builtin_ctzll" => {
                // Count trailing zeros
                if args.len() >= 1 {
                    Some((format!("({}).trailing_zeros() as i32", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_popcount" | "__builtin_popcountl" | "__builtin_popcountll" => {
                // Population count (number of 1 bits)
                if args.len() >= 1 {
                    Some((format!("({}).count_ones() as i32", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_bswap16" => {
                if args.len() >= 1 {
                    Some((format!("({}).swap_bytes()", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_bswap32" => {
                if args.len() >= 1 {
                    Some((format!("({}).swap_bytes()", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_bswap64" => {
                if args.len() >= 1 {
                    Some((format!("({}).swap_bytes()", args[0]), false))
                } else {
                    None
                }
            }
            // Atomic builtins - common patterns
            "__atomic_load_n" => {
                if args.len() >= 2 {
                    Some((format!(
                        "std::sync::atomic::AtomicPtr::new({} as *mut _).load(std::sync::atomic::Ordering::SeqCst)",
                        args[0]
                    ), false))
                } else {
                    None
                }
            }
            "__atomic_store_n" => {
                if args.len() >= 3 {
                    Some((format!(
                        "std::sync::atomic::AtomicPtr::new({} as *mut _).store({}, std::sync::atomic::Ordering::SeqCst)",
                        args[0], args[1]
                    ), false))
                } else {
                    None
                }
            }
            // Variadic function builtins
            // Note: These are simplified implementations. Rust's VaList is unstable,
            // so we generate inline code that works with the transpiled va_list type.
            "__builtin_va_start" => {
                // va_start(ap, param) - Initialize va_list
                // In Rust, we treat this as a no-op since VaList comes pre-initialized
                // when passed as a function parameter
                Some(("{ /* va_start: va_list already initialized */ }".to_string(), false))
            }
            "__builtin_va_end" => {
                // va_end(ap) - Clean up va_list
                // In Rust, this is typically a no-op (cleanup happens automatically)
                Some(("{ /* va_end: no cleanup needed */ }".to_string(), false))
            }
            "__builtin_va_copy" => {
                // va_copy(dest, src) - Copy va_list
                if args.len() >= 2 {
                    Some((format!("{} = {}.clone()", args[0], args[1]), false))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if a type is std::variant (or variant without std:: prefix) and return its C++ template arguments if so.
    fn get_variant_args(ty: &CppType) -> Option<Vec<String>> {
        if let CppType::Named(name) = ty {
            // Handle both "std::variant<...>" and "variant<...>" (libclang sometimes omits std::)
            let rest = name.strip_prefix("std::variant<")
                .or_else(|| name.strip_prefix("variant<"))?;
            let inner = rest.strip_suffix(">")?;
            return Some(parse_template_args(inner));
        }
        None
    }

    /// Get the generated Rust enum name for a variant type.
    fn get_variant_enum_name(ty: &CppType) -> Option<String> {
        if let CppType::Named(name) = ty {
            // Handle both "std::variant<...>" and "variant<...>"
            if name.starts_with("std::variant<") || name.starts_with("variant<") {
                return Some(ty.to_rust_type_str());
            }
        }
        None
    }

    /// Find the variant index for a given C++ type in the variant's template arguments.
    /// Returns the index (0-based) if found.
    fn find_variant_index(variant_args: &[String], init_type: &CppType) -> Option<usize> {
        let init_rust_type = init_type.to_rust_type_str();
        for (idx, arg) in variant_args.iter().enumerate() {
            let arg_rust_type = CppType::Named(arg.clone()).to_rust_type_str();
            if arg_rust_type == init_rust_type {
                return Some(idx);
            }
        }
        None
    }

    /// For variant initialization, find the innermost actual value expression.
    /// This navigates through Unknown("UnexposedExpr") and CallExpr wrappers
    /// to find the actual value being passed to the variant constructor.
    fn find_variant_init_value(node: &ClangNode) -> Option<&ClangNode> {
        match &node.kind {
            // If this is an EvaluatedExpr, it contains the value directly
            ClangNodeKind::EvaluatedExpr { .. } => Some(node),
            // If this is an IntegerLiteral, FloatingLiteral, etc., use it
            ClangNodeKind::IntegerLiteral { .. } |
            ClangNodeKind::FloatingLiteral { .. } |
            ClangNodeKind::StringLiteral(_) |
            ClangNodeKind::BoolLiteral(_) => Some(node),
            // If this is a DeclRefExpr (variable reference), use it
            ClangNodeKind::DeclRefExpr { .. } => Some(node),
            // For CallExpr to variant constructor, look for the argument
            ClangNodeKind::CallExpr { ty } => {
                if let CppType::Named(name) = ty {
                    if name.starts_with("std::variant<") {
                        // This is a call to variant constructor, look for the argument
                        for child in &node.children {
                            if let Some(val) = Self::find_variant_init_value(child) {
                                return Some(val);
                            }
                        }
                    }
                }
                // For non-variant CallExpr, just return it
                Some(node)
            }
            // For Unknown wrappers, recurse into children
            ClangNodeKind::Unknown(_) => {
                for child in &node.children {
                    if let Some(val) = Self::find_variant_init_value(child) {
                        return Some(val);
                    }
                }
                None
            }
            // For ImplicitCastExpr, look through to child
            ClangNodeKind::ImplicitCastExpr { .. } => {
                for child in &node.children {
                    if let Some(val) = Self::find_variant_init_value(child) {
                        return Some(val);
                    }
                }
                None
            }
            // Default: return the node itself
            _ => Some(node),
        }
    }

    /// Check if this is a std::get call on a variant.
    /// Returns (variant_arg_node, variant_type, return_type) if it is.
    fn is_std_get_call<'a>(node: &'a ClangNode) -> Option<(&'a ClangNode, CppType, &'a CppType)> {
        if let ClangNodeKind::CallExpr { ty } = &node.kind {
            // Look for the callee - it may be directly a DeclRefExpr or wrapped in ImplicitCastExpr
            let callee = node.children.first()?;
            let decl_ref = match &callee.kind {
                ClangNodeKind::DeclRefExpr { .. } => callee,
                ClangNodeKind::ImplicitCastExpr { .. } => {
                    // Look inside ImplicitCastExpr for DeclRefExpr
                    callee.children.first()?
                }
                _ => return None,
            };

            if let ClangNodeKind::DeclRefExpr { name, ty: func_ty, .. } = &decl_ref.kind {
                if name == "get" {
                    // Check if first parameter is a reference to variant type
                    if let CppType::Function { params, .. } = func_ty {
                        if let Some(first_param) = params.first() {
                            // Parameter is Reference { referent: Named("variant<...>"), ... }
                            let param_type = match first_param {
                                CppType::Reference { referent, .. } => referent.as_ref(),
                                _ => first_param,
                            };
                            if Self::get_variant_args(param_type).is_some() {
                                // Find the variant argument in children
                                // It's typically the second child (after callee or ImplicitCastExpr)
                                let variant_arg = node.children.get(1)?;
                                let variant_type = Self::get_expr_type(variant_arg)?;
                                if Self::get_variant_args(&variant_type).is_some() {
                                    return Some((variant_arg, variant_type, ty));
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Check if this is a std::visit call on variant(s).
    /// Returns (visitor_node, variant_nodes_with_types) if it is.
    /// visitor_node is the first argument (the callable).
    /// variant_nodes_with_types is a vec of (node, variant_type) for each variant argument.
    fn is_std_visit_call<'a>(node: &'a ClangNode) -> Option<(&'a ClangNode, Vec<(&'a ClangNode, CppType)>)> {
        if let ClangNodeKind::CallExpr { .. } = &node.kind {
            // Look for the callee - it may be directly a DeclRefExpr or wrapped in ImplicitCastExpr
            let callee = node.children.first()?;
            let decl_ref = match &callee.kind {
                ClangNodeKind::DeclRefExpr { .. } => callee,
                ClangNodeKind::ImplicitCastExpr { .. } => {
                    // Look inside ImplicitCastExpr for DeclRefExpr
                    callee.children.first()?
                }
                _ => return None,
            };

            if let ClangNodeKind::DeclRefExpr { name, ty: func_ty, .. } = &decl_ref.kind {
                if name == "visit" {
                    // std::visit signature: visit(Visitor&& vis, Variants&&... vars)
                    // So we expect at least 2 children: callee + visitor + at least one variant
                    if node.children.len() < 3 {
                        return None;
                    }

                    // Check if function type params contain variant references
                    if let CppType::Function { params, .. } = func_ty {
                        // First param is the visitor, remaining are variants
                        if params.len() < 2 {
                            return None;
                        }

                        // Check that at least one param (after visitor) is a variant
                        let mut has_variant = false;
                        for param in params.iter().skip(1) {
                            let param_type = match param {
                                CppType::Reference { referent, .. } => referent.as_ref(),
                                _ => param,
                            };
                            if Self::get_variant_args(param_type).is_some() {
                                has_variant = true;
                                break;
                            }
                        }

                        if !has_variant {
                            return None;
                        }

                        // Get the visitor node (first argument after callee)
                        let visitor_node = node.children.get(1)?;

                        // Collect variant nodes and their types
                        let mut variant_nodes = Vec::new();
                        for arg in node.children.iter().skip(2) {
                            if let Some(var_type) = Self::get_expr_type(arg) {
                                // Unwrap reference types to get the actual variant type
                                let inner_type = match &var_type {
                                    CppType::Reference { referent, .. } => referent.as_ref().clone(),
                                    _ => var_type.clone(),
                                };
                                if Self::get_variant_args(&inner_type).is_some() {
                                    variant_nodes.push((arg, inner_type));
                                }
                            }
                        }

                        if !variant_nodes.is_empty() {
                            return Some((visitor_node, variant_nodes));
                        }
                    }
                }
            }
        }
        None
    }

    /// Check if this is a std::views range adaptor call.
    /// Returns (adaptor_name, range_node, optional_arg_node) if it is.
    /// adaptor_name is one of: "filter", "transform", "take", "drop", "reverse"
    fn is_std_views_adaptor_call<'a>(node: &'a ClangNode) -> Option<(&'static str, &'a ClangNode, Option<&'a ClangNode>)> {
        if let ClangNodeKind::CallExpr { .. } = &node.kind {
            // Look for the callee - it may be directly a DeclRefExpr or wrapped in ImplicitCastExpr
            let callee = node.children.first()?;
            let decl_ref = match &callee.kind {
                ClangNodeKind::DeclRefExpr { .. } => callee,
                ClangNodeKind::ImplicitCastExpr { .. } => callee.children.first()?,
                _ => return None,
            };

            if let ClangNodeKind::DeclRefExpr { name, .. } = &decl_ref.kind {
                // Map std::views adaptor names to Rust iterator methods
                let adaptor_name = match name.as_str() {
                    "filter" => Some("filter"),
                    "transform" => Some("map"),
                    "take" => Some("take"),
                    "drop" => Some("skip"),
                    "reverse" => Some("rev"),
                    "take_while" => Some("take_while"),
                    "drop_while" => Some("skip_while"),
                    _ => None,
                };

                if let Some(adaptor) = adaptor_name {
                    // Get the range argument (first arg after callee)
                    let range_node = node.children.get(1)?;

                    // Get the optional second argument (predicate/count for filter/take/drop, etc.)
                    let arg_node = node.children.get(2);

                    return Some((adaptor, range_node, arg_node));
                }
            }
        }
        None
    }

    /// Check if this is a std::ranges algorithm call.
    /// Returns (algorithm_name, range_node, optional_arg_node) if it is.
    fn is_std_ranges_algorithm_call<'a>(node: &'a ClangNode) -> Option<(&'static str, &'a ClangNode, Option<&'a ClangNode>)> {
        if let ClangNodeKind::CallExpr { .. } = &node.kind {
            let callee = node.children.first()?;
            let decl_ref = match &callee.kind {
                ClangNodeKind::DeclRefExpr { .. } => callee,
                ClangNodeKind::ImplicitCastExpr { .. } => callee.children.first()?,
                _ => return None,
            };

            if let ClangNodeKind::DeclRefExpr { name, .. } = &decl_ref.kind {
                // Map std::ranges algorithm names to Rust iterator methods
                let algo_name = match name.as_str() {
                    "for_each" => Some("for_each"),
                    "find" => Some("find"),
                    "find_if" => Some("find"),
                    "sort" => Some("sort"),
                    "copy" => Some("collect"),
                    "any_of" => Some("any"),
                    "all_of" => Some("all"),
                    "none_of" => Some("all"), // Handled specially: none_of(f) => !all(f)
                    "count" => Some("count"),
                    "count_if" => Some("count"),
                    _ => None,
                };

                if let Some(algo) = algo_name {
                    let range_node = node.children.get(1)?;
                    let arg_node = node.children.get(2);
                    return Some((algo, range_node, arg_node));
                }
            }
        }
        None
    }

    /// Get the variant index by matching the return type to variant template arguments.
    /// The return type from std::get is T& where T is one of the variant types.
    /// For std::get<I>, the return type may be variant_alternative_t<I, variant<...>>.
    fn get_variant_index_from_return_type(variant_type: &CppType, return_type: &CppType) -> Option<usize> {
        let variant_args = Self::get_variant_args(variant_type)?;

        // Extract the referent type if return_type is a reference (std::get returns T&)
        let target_type = match return_type {
            CppType::Reference { referent, .. } => referent.as_ref(),
            _ => return_type,
        };

        // Check if the return type is variant_alternative_t<Index, variant<...>>
        // This happens with std::get<I>(v) where I is an index
        if let CppType::Named(name) = target_type {
            if let Some(rest) = name.strip_prefix("variant_alternative_t<") {
                // Parse "0UL, variant<int, double, bool>>" to extract the index
                if let Some(comma_pos) = rest.find(',') {
                    let idx_str = rest[..comma_pos].trim();
                    // Remove suffix like "UL" or "u" from the index
                    let idx_num: String = idx_str.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if let Ok(idx) = idx_num.parse::<usize>() {
                        return Some(idx);
                    }
                }
            }
        }

        // Otherwise, find matching index using Rust type string comparison
        Self::find_variant_index(&variant_args, target_type)
    }

    /// Determine how to call the visitor in std::visit.
    /// Returns a format string where {} is the args placeholder.
    /// - For lambdas: "(visitor)({})"
    /// - For functors: "visitor.op_call({})"
    /// - For function pointers: "(visitor)({})" or "visitor.unwrap()({})"
    fn get_visitor_call_format(&self, visitor_node: &ClangNode, visitor_expr: &str) -> String {
        // Check if visitor is a lambda (type contains "lambda at")
        if let Some(visitor_type) = Self::get_expr_type(visitor_node) {
            if let CppType::Named(name) = &visitor_type {
                if name.contains("lambda at ") {
                    // Lambda - callable directly
                    return format!("({})({{}})", visitor_expr);
                }
            }
            // Check if it's a function pointer (Option<fn(...)>)
            if let CppType::Pointer { pointee, .. } = &visitor_type {
                if matches!(pointee.as_ref(), CppType::Function { .. }) {
                    // Function pointer wrapped in Option - use unwrap
                    return format!("{}.unwrap()({{}})", visitor_expr);
                }
            }
            if matches!(visitor_type, CppType::Function { .. }) {
                // Direct function reference - callable directly
                return format!("({})({{}})", visitor_expr);
            }
            // For struct/class types (functors), use op_call
            if let CppType::Named(_) = &visitor_type {
                // Functor - use op_call method
                return format!("{}.op_call({{}})", visitor_expr);
            }
        }
        // Default to direct call for lambdas and other callables
        format!("({})({{}})", visitor_expr)
    }

    /// Generate a match expression for std::visit on one or more variants.
    /// visitor_node is the visitor (lambda, functor, or function).
    /// variants is a list of (node, type) pairs for each variant argument.
    fn generate_visit_match(&self, visitor_node: &ClangNode, variants: &[(&ClangNode, CppType)], _return_type: &CppType) -> String {
        if variants.is_empty() {
            return "/* std::visit error: no variants */".to_string();
        }

        // Generate the visitor expression
        let visitor_expr = self.expr_to_string(visitor_node);

        // Determine how to call the visitor (lambda, functor, or function)
        let call_format = self.get_visitor_call_format(visitor_node, &visitor_expr);

        // For single variant, generate a simple match
        if variants.len() == 1 {
            let (var_node, var_type) = &variants[0];
            let var_expr = self.expr_to_string(var_node);
            if let Some(enum_name) = Self::get_variant_enum_name(var_type) {
                if let Some(args) = Self::get_variant_args(var_type) {
                    let arms: Vec<String> = (0..args.len())
                        .map(|i| format!("{}::V{}(__v) => {}", enum_name, i, call_format.replace("{}", "__v")))
                        .collect();
                    return format!("match &{} {{ {} }}", var_expr, arms.join(", "));
                }
            }
            return format!("/* std::visit error: cannot process variant type {:?} */", var_type);
        }

        // For multiple variants, generate cartesian product of match arms
        // Collect variant info
        let mut var_info: Vec<(String, String, usize)> = Vec::new(); // (expr, enum_name, num_variants)
        for (var_node, var_type) in variants {
            let var_expr = self.expr_to_string(var_node);
            if let Some(enum_name) = Self::get_variant_enum_name(var_type) {
                if let Some(args) = Self::get_variant_args(var_type) {
                    var_info.push((var_expr, enum_name, args.len()));
                }
            }
        }

        if var_info.is_empty() {
            return "/* std::visit error: no valid variants */".to_string();
        }

        // Generate match expression on tuple of variants
        let tuple_expr: Vec<String> = var_info.iter().map(|(e, _, _)| format!("&{}", e)).collect();

        // Generate all combinations (cartesian product)
        let mut arms: Vec<String> = Vec::new();
        let mut indices: Vec<usize> = vec![0; var_info.len()];
        loop {
            // Build pattern for this combination: (Enum1::V0(__v0), Enum2::V1(__v1), ...)
            let patterns: Vec<String> = var_info.iter()
                .enumerate()
                .map(|(i, (_, enum_name, _))| format!("{}::V{}(__v{})", enum_name, indices[i], i))
                .collect();
            // Build visitor call with appropriate call format
            let args: Vec<String> = (0..var_info.len()).map(|i| format!("__v{}", i)).collect();
            let args_str = args.join(", ");
            arms.push(format!("({}) => {}", patterns.join(", "), call_format.replace("{}", &args_str)));

            // Increment indices (like counting in mixed-radix)
            let mut carry = true;
            for i in (0..var_info.len()).rev() {
                if carry {
                    indices[i] += 1;
                    if indices[i] >= var_info[i].2 {
                        indices[i] = 0;
                        carry = true;
                    } else {
                        carry = false;
                    }
                }
            }
            if carry {
                break; // All combinations exhausted
            }
        }

        format!("match ({}) {{ {} }}", tuple_expr.join(", "), arms.join(", "))
    }

    /// Generate Rust enum definitions for all collected std::variant types.
    fn generate_variant_enums(&mut self) {
        if self.variant_types.is_empty() {
            return;
        }

        // Clone and sort by enum name for deterministic output
        let mut variants: Vec<_> = self.variant_types.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        variants.sort_by_key(|(name, _)| name.clone());

        for (enum_name, rust_types) in variants {
            self.writeln(&format!("/// Generated Rust enum for std::variant type"));
            self.writeln("#[derive(Clone, Debug)]");
            self.writeln(&format!("pub enum {} {{", enum_name));
            self.indent += 1;

            for (idx, rust_type) in rust_types.iter().enumerate() {
                self.writeln(&format!("V{}({}),", idx, rust_type));
            }

            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
        }
    }

    /// Compute the relative Rust path from current namespace to target namespace.
    /// Returns the path string to use for referring to an item in target_ns from current_namespace.
    fn compute_relative_path(&self, target_ns: &[String], ident: &str) -> String {
        // If target namespace matches current namespace, just use the identifier
        if target_ns == self.current_namespace.as_slice() {
            return ident.to_string();
        }

        // Find the common prefix length
        let common_len = target_ns.iter()
            .zip(self.current_namespace.iter())
            .take_while(|(a, b)| a == b)
            .count();

        // Calculate how many levels to go up
        let levels_up = self.current_namespace.len() - common_len;

        // Build the path: super:: for going up, then the remaining target path
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..levels_up {
            parts.push("super".to_string());
        }

        // Add the remaining path segments from target_ns (after common prefix)
        for ns in target_ns.iter().skip(common_len) {
            parts.push(sanitize_identifier(ns));
        }

        // Add the identifier at the end
        parts.push(ident.to_string());

        parts.join("::")
    }

    /// Generate Rust stubs (signatures only, no bodies) from a Clang AST.
    /// This is useful for FFI declarations and header generation.
    pub fn generate_stubs(mut self, ast: &ClangNode) -> String {
        // File header
        self.writeln("// Auto-generated Rust stubs from C++ code");
        self.writeln("#![allow(dead_code)]");
        self.writeln("#![allow(unused_variables)]");
        self.writeln("");

        // Process translation unit
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            for child in &ast.children {
                self.generate_stub_top_level(child);
            }
        }

        self.output
    }

    fn write_array_helpers(&mut self) {
        self.writeln("// Helper for C++ new[] / delete[] with size tracking");
        self.writeln("#[inline]");
        self.writeln("unsafe fn fragile_new_array<T: Clone>(len: usize, init: T) -> *mut T {");
        self.indent += 1;
        self.writeln("let align = std::mem::align_of::<T>().max(std::mem::align_of::<usize>());");
        self.writeln("let header_size = std::mem::size_of::<usize>();");
        self.writeln("let padding = (align - (header_size % align)) % align;");
        self.writeln("let offset = header_size + padding;");
        self.writeln("let elem_size = std::mem::size_of::<T>();");
        self.writeln("let total_size = offset + elem_size.saturating_mul(len);");
        self.writeln("let layout = std::alloc::Layout::from_size_align(total_size, align).unwrap();");
        self.writeln("let base = std::alloc::alloc(layout);");
        self.writeln("if base.is_null() { std::alloc::handle_alloc_error(layout); }");
        self.writeln("let header = base as *mut usize;");
        self.writeln("*header = len;");
        self.writeln("let data = base.add(offset) as *mut T;");
        self.writeln("for i in 0..len {");
        self.indent += 1;
        self.writeln("std::ptr::write(data.add(i), init.clone());");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("data");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("#[inline]");
        self.writeln("unsafe fn fragile_delete_array<T>(ptr: *mut T) {");
        self.indent += 1;
        self.writeln("if ptr.is_null() { return; }");
        self.writeln("let align = std::mem::align_of::<T>().max(std::mem::align_of::<usize>());");
        self.writeln("let header_size = std::mem::size_of::<usize>();");
        self.writeln("let padding = (align - (header_size % align)) % align;");
        self.writeln("let offset = header_size + padding;");
        self.writeln("let base = (ptr as *mut u8).sub(offset);");
        self.writeln("let len = *(base as *mut usize);");
        self.writeln("for i in 0..len {");
        self.indent += 1;
        self.writeln("std::ptr::drop_in_place(ptr.add(i));");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("let elem_size = std::mem::size_of::<T>();");
        self.writeln("let total_size = offset + elem_size.saturating_mul(len);");
        self.writeln("let layout = std::alloc::Layout::from_size_align(total_size, align).unwrap();");
        self.writeln("std::alloc::dealloc(base, layout);");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a top-level stub declaration (signatures only).
    fn generate_stub_top_level(&mut self, node: &ClangNode) {
        match &node.kind {
            ClangNodeKind::FunctionDecl { name, mangled_name, return_type, params, is_definition, is_variadic, .. } => {
                if *is_definition {
                    self.generate_function_stub(name, mangled_name, return_type, params, *is_variadic);
                }
            }
            ClangNodeKind::RecordDecl { name, is_class, .. } => {
                self.generate_struct_stub(name, *is_class, &node.children);
            }
            ClangNodeKind::EnumDecl { name, is_scoped, underlying_type } => {
                self.generate_enum_stub(name, *is_scoped, underlying_type, &node.children);
            }
            ClangNodeKind::UnionDecl { name, .. } => {
                self.generate_union_stub(name, &node.children);
            }
            ClangNodeKind::NamespaceDecl { name } => {
                // Generate Rust module for namespace stubs
                if let Some(ns_name) = name {
                    if ns_name.starts_with("__") || ns_name == "std" {
                        for child in &node.children {
                            self.generate_stub_top_level(child);
                        }
                    } else {
                        self.writeln(&format!("pub mod {} {{", sanitize_identifier(ns_name)));
                        self.indent += 1;
                        for child in &node.children {
                            self.generate_stub_top_level(child);
                        }
                        self.indent -= 1;
                        self.writeln("}");
                        self.writeln("");
                    }
                } else {
                    for child in &node.children {
                        self.generate_stub_top_level(child);
                    }
                }
            }
            _ => {}
        }
    }

    /// Generate a function stub (signature with placeholder body).
    fn generate_function_stub(&mut self, name: &str, mangled_name: &str, return_type: &CppType,
                              params: &[(String, CppType)], is_variadic: bool) {
        self.writeln(&format!("/// @fragile_cpp_mangled: {}", mangled_name));
        self.writeln(&format!("#[export_name = \"{}\"]", mangled_name));

        let params_str = params.iter()
            .map(|(n, t)| format!("{}: {}", sanitize_identifier(n), t.to_rust_type_str()))
            .collect::<Vec<_>>()
            .join(", ");

        // Add variadic indicator for C variadic functions
        let params_with_variadic = if is_variadic {
            if params_str.is_empty() {
                "...".to_string()
            } else {
                format!("{}, ...", params_str)
            }
        } else {
            params_str
        };

        let ret_str = if *return_type == CppType::Void {
            String::new()
        } else {
            format!(" -> {}", return_type.to_rust_type_str())
        };

        self.writeln(&format!("pub extern \"C\" fn {}({}){} {{", sanitize_identifier(name), params_with_variadic, ret_str));
        self.indent += 1;
        self.writeln("// Stub body - replaced by MIR injection at compile time");
        self.writeln("unreachable!(\"Fragile: C++ MIR should be injected\")");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a struct stub (fields only).
    fn generate_struct_stub(&mut self, name: &str, is_class: bool, children: &[ClangNode]) {
        // Convert C++ struct name to valid Rust identifier (handles template types)
        let rust_name = CppType::Named(name.to_string()).to_rust_type_str();

        // Skip if already generated (handles duplicate template instantiations)
        if self.generated_structs.contains(&rust_name) {
            return;
        }
        self.generated_structs.insert(rust_name.clone());

        let kind = if is_class { "class" } else { "struct" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));
        self.writeln("#[repr(C)]");
        self.writeln(&format!("pub struct {} {{", rust_name));
        self.indent += 1;

        // First, embed non-virtual base classes as fields (supports multiple inheritance)
        // Also collect base fields for class_fields tracking
        let mut base_fields = Vec::new();
        let mut base_idx = 0;
        for child in children {
            if let ClangNodeKind::CXXBaseSpecifier { base_type, access, is_virtual, .. } = &child.kind {
                if !matches!(access, crate::ast::AccessSpecifier::Private) {
                    if *is_virtual {
                        continue;
                    }
                    let base_name = base_type.to_rust_type_str();
                    // Use __base for single inheritance, __base0/__base1/etc for MI
                    let field_name = if base_idx == 0 { "__base".to_string() } else { format!("__base{}", base_idx) };
                    self.writeln(&format!("pub {}: {},", field_name, base_name));
                    base_fields.push((field_name, base_type.clone()));
                    base_idx += 1;
                }
            }
        }

        // Add virtual base pointers and storage if needed
        let vbases_to_add = self.virtual_bases.get(name).cloned().unwrap_or_default();
        for vb in &vbases_to_add {
            let field = self.virtual_base_field_name(vb);
            let storage = self.virtual_base_storage_field_name(vb);
            self.writeln(&format!("pub {}: *mut {},", field, vb));
            self.writeln(&format!("pub {}: Option<Box<{}>>,", storage, vb));
        }

        // Then add derived class fields (including flattened anonymous struct fields)
        let mut fields = Vec::new();
        for child in children {
            if let ClangNodeKind::FieldDecl { name: field_name, ty, access, .. } = &child.kind {
                let sanitized_name = if field_name.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(field_name)
                };
                let vis = access_to_visibility(*access);
                self.writeln(&format!("{}{}: {},", vis, sanitized_name, ty.to_rust_type_str()));
                fields.push((sanitized_name, ty.clone()));
            } else if let ClangNodeKind::RecordDecl { name: anon_name, .. } = &child.kind {
                // Flatten anonymous struct fields into parent
                if anon_name.starts_with("(anonymous") || anon_name.starts_with("__anon_") {
                    for anon_child in &child.children {
                        if let ClangNodeKind::FieldDecl { name: field_name, ty, access, .. } = &anon_child.kind {
                            let sanitized_name = if field_name.is_empty() {
                                "_field".to_string()
                            } else {
                                sanitize_identifier(field_name)
                            };
                            let vis = access_to_visibility(*access);
                            self.writeln(&format!("{}{}: {},", vis, sanitized_name, ty.to_rust_type_str()));
                            fields.push((sanitized_name, ty.clone()));
                        }
                    }
                }
            } else if let ClangNodeKind::UnionDecl { name: anon_name, .. } = &child.kind {
                // Flatten anonymous union fields into parent
                // In C++, anonymous unions allow direct access to their members from the parent
                if anon_name.starts_with("(anonymous") || anon_name.starts_with("__anon_union_") {
                    for anon_child in &child.children {
                        if let ClangNodeKind::FieldDecl { name: field_name, ty, access, .. } = &anon_child.kind {
                            let sanitized_name = if field_name.is_empty() {
                                "_field".to_string()
                            } else {
                                sanitize_identifier(field_name)
                            };
                            let vis = access_to_visibility(*access);
                            self.writeln(&format!("{}{}: {},", vis, sanitized_name, ty.to_rust_type_str()));
                            fields.push((sanitized_name, ty.clone()));
                        }
                    }
                }
            }
        }
        // Store field info for constructor generation (including base fields)
        let mut all_fields = base_fields;
        all_fields.extend(fields);
        self.class_fields.insert(name.to_string(), all_fields);

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate an enum stub.
    fn generate_enum_stub(&mut self, name: &str, is_scoped: bool, underlying_type: &CppType, children: &[ClangNode]) {
        let kind = if is_scoped { "enum class" } else { "enum" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));

        // Generate as Rust enum
        let repr_type = underlying_type.to_rust_type_str();
        self.writeln(&format!("#[repr({})]", repr_type));
        self.writeln("#[derive(Clone, Copy, PartialEq, Eq, Debug)]");
        self.writeln(&format!("pub enum {} {{", name));
        self.indent += 1;

        for child in children {
            if let ClangNodeKind::EnumConstantDecl { name: const_name, value } = &child.kind {
                if let Some(v) = value {
                    self.writeln(&format!("{} = {},", const_name, v));
                } else {
                    self.writeln(&format!("{},", const_name));
                }
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a union stub (fields only).
    fn generate_union_stub(&mut self, name: &str, children: &[ClangNode]) {
        // Convert C++ union name to valid Rust identifier
        let rust_name = CppType::Named(name.to_string()).to_rust_type_str();

        // Skip if already generated
        if self.generated_structs.contains(&rust_name) {
            return;
        }
        self.generated_structs.insert(rust_name.clone());

        self.writeln(&format!("/// C++ union `{}`", name));
        self.writeln("#[repr(C)]");
        self.writeln(&format!("pub union {} {{", rust_name));
        self.indent += 1;

        for child in children {
            if let ClangNodeKind::FieldDecl { name: field_name, ty, access, .. } = &child.kind {
                let sanitized_name = if field_name.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(field_name)
                };
                let vis = access_to_visibility(*access);
                self.writeln(&format!("{}{}: {},", vis, sanitized_name, ty.to_rust_type_str()));
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a top-level declaration.
    fn generate_top_level(&mut self, node: &ClangNode) {
        match &node.kind {
            ClangNodeKind::FunctionDecl { name, mangled_name, return_type, params, is_definition, is_variadic, is_coroutine, coroutine_info, .. } => {
                if *is_definition {
                    self.generate_function(name, mangled_name, return_type, params, *is_variadic, *is_coroutine, coroutine_info, &node.children);
                }
            }
            ClangNodeKind::RecordDecl { name, is_class, .. } => {
                self.generate_struct(name, *is_class, &node.children);
            }
            ClangNodeKind::EnumDecl { name, is_scoped, underlying_type } => {
                self.generate_enum(name, *is_scoped, underlying_type, &node.children);
            }
            ClangNodeKind::UnionDecl { name, .. } => {
                self.generate_union(name, &node.children);
            }
            ClangNodeKind::TypedefDecl { name, underlying_type } => {
                self.generate_type_alias(name, underlying_type);
            }
            ClangNodeKind::TypeAliasDecl { name, underlying_type } => {
                self.generate_type_alias(name, underlying_type);
            }
            ClangNodeKind::VarDecl { name, ty, has_init } => {
                // Skip out-of-class static member definitions (TypeRef child indicates qualified name)
                // These are already handled in the class generation
                let is_static_member_def = node.children.iter().any(|c| {
                    matches!(&c.kind, ClangNodeKind::Unknown(s) if s.starts_with("TypeRef:"))
                });
                if !is_static_member_def {
                    self.generate_global_var(name, ty, *has_init, &node.children);
                }
            }
            ClangNodeKind::ModuleImportDecl { module_name, is_header_unit } => {
                // C++20 module import → comment for now (pending full module support)
                // In the future, this could map to:
                // - `use module_name::*;` for regular modules
                // - `include!("header.rs");` for header units
                if *is_header_unit {
                    self.writeln(&format!("// C++20 header unit import: import <{}>", module_name));
                } else {
                    // Convert module path separators (. or ::) to Rust path
                    let rust_path = module_name.replace('.', "::");
                    self.writeln(&format!("// C++20 module import: import {}", module_name));
                    // Generate a use statement as a placeholder
                    // When modules are fully implemented, this will become functional
                    if !rust_path.is_empty() {
                        self.writeln(&format!("// use {}::*; // (pending module implementation)", sanitize_identifier(&rust_path)));
                    }
                }
            }
            ClangNodeKind::NamespaceDecl { name } => {
                // Generate Rust module for namespace
                if let Some(ns_name) = name {
                    // Skip anonymous namespaces or standard library namespaces
                    if ns_name.starts_with("__") || ns_name == "std" {
                        // Just process children without module wrapper for internal namespaces
                        for child in &node.children {
                            self.generate_top_level(child);
                        }
                    } else {
                        // Create a module for the namespace
                        self.writeln(&format!("pub mod {} {{", sanitize_identifier(ns_name)));
                        self.indent += 1;
                        // Track current namespace for relative path computation
                        self.current_namespace.push(ns_name.clone());
                        for child in &node.children {
                            self.generate_top_level(child);
                        }
                        self.current_namespace.pop();
                        self.indent -= 1;
                        self.writeln("}");
                        self.writeln("");
                    }
                } else {
                    // Anonymous namespace - generate private module with synthetic name
                    // This mirrors C++ semantics where anonymous namespaces have internal linkage
                    let anon_name = format!("__anon_{}", self.anon_namespace_counter);
                    self.anon_namespace_counter += 1;

                    self.writeln(&format!("/// Anonymous namespace (internal linkage)"));
                    self.writeln(&format!("mod {} {{", anon_name));
                    self.indent += 1;

                    // Track the synthetic namespace name for path resolution
                    self.current_namespace.push(anon_name.clone());
                    for child in &node.children {
                        self.generate_top_level(child);
                    }
                    self.current_namespace.pop();

                    self.indent -= 1;
                    self.writeln("}");

                    // Auto-use the contents so they're accessible in parent scope
                    self.writeln(&format!("use {}::*;", anon_name));
                    self.writeln("");
                }
            }
            _ => {}
        }
    }

    /// Get the appropriate return type string for a function, considering coroutine info.
    /// For async coroutines with value type, uses the extracted type.
    /// For generators, could use impl Iterator<Item=T> (future enhancement).
    fn get_coroutine_return_type(&self, return_type: &CppType, coroutine_info: &Option<CoroutineInfo>) -> String {
        if let Some(info) = coroutine_info {
            // If we extracted a value type from the coroutine return type, use it
            if let Some(ref value_type) = info.value_type {
                match info.kind {
                    CoroutineKind::Async | CoroutineKind::Task => {
                        // async fn returns the inner type directly
                        if *value_type == CppType::Void {
                            return String::new();
                        }
                        return format!(" -> {}", value_type.to_rust_type_str());
                    }
                    CoroutineKind::Generator => {
                        // Generators should return impl Iterator<Item=T>
                        // Note: Rust generators are unstable, so this is forward-looking
                        return format!(" -> impl Iterator<Item={}>", value_type.to_rust_type_str());
                    }
                    CoroutineKind::Custom => {
                        // Fall through to default handling
                    }
                }
            }
        }

        // Default: use the original return type
        if *return_type == CppType::Void {
            String::new()
        } else {
            format!(" -> {}", return_type.to_rust_type_str())
        }
    }

    /// Collect co_yield expressions from a generator function body.
    /// Returns a list of yield value strings.
    fn collect_generator_yields(&mut self, children: &[ClangNode]) -> Vec<String> {
        let mut yields = Vec::new();
        self.collect_yields_recursive(children, &mut yields);
        yields
    }

    fn collect_yields_recursive(&mut self, children: &[ClangNode], yields: &mut Vec<String>) {
        for child in children {
            if let ClangNodeKind::CoyieldExpr { .. } = &child.kind {
                // Extract the yield value
                if !child.children.is_empty() {
                    let value = self.expr_to_string(&child.children[0]);
                    yields.push(value);
                } else {
                    yields.push("()".to_string());
                }
            }
            // Recursively search in children
            self.collect_yields_recursive(&child.children, yields);
        }
    }

    /// Generate a state machine struct and Iterator implementation for a generator.
    fn generate_generator_struct(&mut self, func_name: &str, item_type: &str, yields: &[String]) {
        let struct_name = format!("{}Generator", to_pascal_case(func_name));

        // Generate the struct
        self.writeln(&format!("/// State machine struct for generator `{}`", func_name));
        self.writeln(&format!("pub struct {} {{", struct_name));
        self.indent += 1;
        self.writeln("__state: i32,");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // Generate Iterator implementation
        self.writeln(&format!("impl Iterator for {} {{", struct_name));
        self.indent += 1;
        self.writeln(&format!("type Item = {};", item_type));
        self.writeln("");
        self.writeln("fn next(&mut self) -> Option<Self::Item> {");
        self.indent += 1;
        self.writeln("match self.__state {");
        self.indent += 1;

        // Generate match arms for each yield
        for (i, yield_val) in yields.iter().enumerate() {
            self.writeln(&format!("{} => {{ self.__state = {}; Some({}) }}", i, i + 1, yield_val));
        }

        // Final state returns None
        self.writeln("_ => None,");

        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a function definition.
    fn generate_function(&mut self, name: &str, mangled_name: &str, return_type: &CppType,
                         params: &[(String, CppType)], is_variadic: bool, is_coroutine: bool,
                         coroutine_info: &Option<CoroutineInfo>, children: &[ClangNode]) {
        // Special handling for C++ main function
        let is_main = name == "main" && params.is_empty();
        let func_name = if is_main { "cpp_main" } else { name };

        // Doc comment
        self.writeln(&format!("/// C++ function `{}`", name));
        self.writeln(&format!("/// Mangled: `{}`", mangled_name));

        // Add coroutine info comment if present
        if let Some(info) = coroutine_info {
            let kind_str = match info.kind {
                CoroutineKind::Async => "async",
                CoroutineKind::Generator => "generator",
                CoroutineKind::Task => "task",
                CoroutineKind::Custom => "custom",
            };
            self.writeln(&format!("/// Coroutine: {} ({})", kind_str, info.return_type_spelling));
        }

        // Track reference, pointer, and array parameters - clear any from previous function
        self.ref_vars.clear();
        self.ptr_vars.clear();
        self.arr_vars.clear();
        for (param_name, param_type) in params {
            if matches!(param_type, CppType::Reference { .. }) {
                self.ref_vars.insert(param_name.clone());
            }
            // Unsized arrays in function parameters are actually pointers in C++
            // (int arr[] is equivalent to int* arr)
            if matches!(param_type, CppType::Pointer { .. })
                || matches!(param_type, CppType::Array { size: None, .. }) {
                self.ptr_vars.insert(param_name.clone());
            }
            // Only track sized arrays as arrays
            if matches!(param_type, CppType::Array { size: Some(_), .. }) {
                self.arr_vars.insert(param_name.clone());
            }
        }

        // Function signature - convert polymorphic pointers to trait objects
        let params_str = params.iter()
            .map(|(n, t)| {
                let type_str = self.convert_type_for_polymorphism(t);
                format!("{}: {}", sanitize_identifier(n), type_str)
            })
            .collect::<Vec<_>>()
            .join(", ");

        // Determine return type based on coroutine info
        let ret_str = self.get_coroutine_return_type(return_type, coroutine_info);

        // Check if this is a generator
        let is_generator = is_coroutine && matches!(
            coroutine_info.as_ref().map(|i| i.kind),
            Some(CoroutineKind::Generator)
        );

        // Determine if this should be an async function
        let is_async = is_coroutine && matches!(
            coroutine_info.as_ref().map(|i| i.kind),
            Some(CoroutineKind::Async) | Some(CoroutineKind::Task) | None
        );

        // Handle generators with state machine
        if is_generator {
            // Collect all yield expressions
            let yields = self.collect_generator_yields(children);

            // Get the item type for the iterator
            let item_type = if let Some(ref info) = coroutine_info {
                if let Some(ref vt) = info.value_type {
                    vt.to_rust_type_str()
                } else {
                    "()".to_string()
                }
            } else {
                "()".to_string()
            };

            // Generate the state machine struct and Iterator implementation
            self.generate_generator_struct(func_name, &item_type, &yields);

            // Generate the function that returns the generator
            let struct_name = format!("{}Generator", to_pascal_case(func_name));
            self.writeln(&format!("pub fn {}({}){} {{", sanitize_identifier(func_name), params_str, ret_str));
            self.indent += 1;
            self.writeln(&format!("{} {{ __state: 0 }}", struct_name));
            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
        } else {
            // Normal function handling
            // Add variadic indicator for C variadic functions
            let params_with_variadic = if is_variadic {
                if params_str.is_empty() {
                    "...".to_string()
                } else {
                    format!("{}, ...", params_str)
                }
            } else {
                params_str
            };

            // Variadic functions require extern "C" linkage
            let (async_keyword, extern_c) = if is_variadic {
                ("", "extern \"C\" ")
            } else if is_async {
                ("async ", "")
            } else {
                ("", "")
            };
            self.writeln(&format!("pub {}{}fn {}({}){} {{", async_keyword, extern_c, sanitize_identifier(func_name), params_with_variadic, ret_str));
            self.indent += 1;

            // Find the compound statement (function body)
            for child in children {
                if let ClangNodeKind::CompoundStmt = &child.kind {
                    self.generate_block_contents(&child.children, return_type);
                }
            }

            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
        }

        // Generate Rust main wrapper for C++ main
        if is_main {
            self.writeln("fn main() {");
            self.indent += 1;
            self.writeln("std::process::exit(cpp_main());");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
        }
    }

    /// Collect and group bit fields from a list of field declarations.
    /// Returns a tuple of (bit_field_groups, regular_field_indices).
    /// regular_field_indices contains indices into the original children array for non-bit-field entries.
    fn collect_bit_field_groups(&self, children: &[ClangNode]) -> (Vec<BitFieldGroup>, Vec<usize>) {
        let mut groups: Vec<BitFieldGroup> = Vec::new();
        let mut regular_indices: Vec<usize> = Vec::new();
        let mut current_group: Option<BitFieldGroup> = None;
        let mut group_index = 0;

        for (idx, child) in children.iter().enumerate() {
            if let ClangNodeKind::FieldDecl { name: field_name, ty, access, is_static, bit_field_width } = &child.kind {
                if *is_static {
                    continue; // Static fields handled separately
                }

                if let Some(width) = bit_field_width {
                    // This is a bit field
                    let bit_info = BitFieldInfo {
                        field_name: field_name.clone(),
                        original_type: ty.clone(),
                        width: *width,
                        offset: 0, // Will be set below
                        access: *access,
                    };

                    if let Some(ref mut group) = current_group {
                        // Check if we can add to current group (total bits <= 64 to fit in u64)
                        // Note: C++ allows up to storage unit size, we use 64 bits max for simplicity
                        if group.total_bits + width <= 64 {
                            // Add to existing group
                            let mut info = bit_info;
                            info.offset = group.total_bits;
                            group.total_bits += width;
                            group.fields.push(info);
                        } else {
                            // Start new group, finalize current one
                            groups.push(current_group.take().unwrap());
                            group_index += 1;

                            let mut info = bit_info;
                            info.offset = 0;
                            current_group = Some(BitFieldGroup {
                                fields: vec![info],
                                total_bits: *width,
                                group_index,
                            });
                        }
                    } else {
                        // Start new group
                        let mut info = bit_info;
                        info.offset = 0;
                        current_group = Some(BitFieldGroup {
                            fields: vec![info],
                            total_bits: *width,
                            group_index,
                        });
                    }
                } else {
                    // Regular field - finalize any current bit field group first
                    if let Some(group) = current_group.take() {
                        groups.push(group);
                        group_index += 1;
                    }
                    regular_indices.push(idx);
                }
            } else {
                // Non-field node - finalize any current bit field group
                if let Some(group) = current_group.take() {
                    groups.push(group);
                    group_index += 1;
                }
                // Pass through non-FieldDecl nodes (e.g., anonymous structs/unions)
                regular_indices.push(idx);
            }
        }

        // Finalize last group if any
        if let Some(group) = current_group.take() {
            groups.push(group);
        }

        (groups, regular_indices)
    }

    /// Generate getter and setter methods for bit fields.
    /// Must be called inside an impl block.
    fn generate_bit_field_accessors(&mut self, struct_name: &str) {
        let groups = match self.bit_field_groups.get(struct_name) {
            Some(g) => g.clone(),
            None => return,
        };

        for group in &groups {
            let storage_type = group.storage_type();
            let storage_field = format!("_bitfield_{}", group.group_index);

            for field in &group.fields {
                let vis = access_to_visibility(field.access);
                let field_name = sanitize_identifier(&field.field_name);
                let ret_type = field.original_type.to_rust_type_str();

                // Calculate mask for this field's width
                let mask = (1u64 << field.width) - 1;

                // Getter: extract bits and cast to original type
                self.writeln(&format!("/// Getter for bit field `{}`", field.field_name));
                self.writeln(&format!("{}fn {}(&self) -> {} {{", vis, field_name, ret_type));
                self.indent += 1;
                if field.offset == 0 {
                    self.writeln(&format!("(self.{} & 0x{:X}) as {}",
                        storage_field, mask, ret_type));
                } else {
                    self.writeln(&format!("((self.{} >> {}) & 0x{:X}) as {}",
                        storage_field, field.offset, mask, ret_type));
                }
                self.indent -= 1;
                self.writeln("}");
                self.writeln("");

                // Setter: clear bits and set new value
                self.writeln(&format!("/// Setter for bit field `{}`", field.field_name));
                self.writeln(&format!("{}fn set_{}(&mut self, v: {}) {{", vis, field_name, ret_type));
                self.indent += 1;
                if field.offset == 0 {
                    self.writeln(&format!("self.{} = (self.{} & !0x{:X}) | ((v as {}) & 0x{:X});",
                        storage_field, storage_field, mask, storage_type, mask));
                } else {
                    let shifted_mask = mask << field.offset;
                    self.writeln(&format!("self.{} = (self.{} & !0x{:X}) | (((v as {}) & 0x{:X}) << {});",
                        storage_field, storage_field, shifted_mask, storage_type, mask, field.offset));
                }
                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }
        }
    }

    /// Generate struct definition.
    fn generate_struct(&mut self, name: &str, is_class: bool, children: &[ClangNode]) {
        // Convert C++ struct name to valid Rust identifier (handles template types)
        let rust_name = CppType::Named(name.to_string()).to_rust_type_str();

        // Skip if already generated (handles duplicate template instantiations)
        if self.generated_structs.contains(&rust_name) {
            return;
        }
        self.generated_structs.insert(rust_name.clone());

        let kind = if is_class { "class" } else { "struct" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default)]");
        self.writeln(&format!("pub struct {} {{", rust_name));
        self.indent += 1;

        // First, embed non-virtual base classes as fields (supports multiple inheritance)
        // Base classes must come first to maintain C++ memory layout
        let mut base_fields = Vec::new();
        let mut base_idx = 0;
        for child in children {
            if let ClangNodeKind::CXXBaseSpecifier { base_type, access, is_virtual, .. } = &child.kind {
                // Only include public/protected bases (private inheritance is more complex)
                if !matches!(access, crate::ast::AccessSpecifier::Private) {
                    if *is_virtual {
                        continue;
                    }
                    let base_name = base_type.to_rust_type_str();
                    // Use __base for first base (backward compatible), __base1/__base2/etc for MI
                    let field_name = if base_idx == 0 { "__base".to_string() } else { format!("__base{}", base_idx) };
                    self.writeln(&format!("/// Inherited from `{}`", base_name));
                    self.writeln(&format!("pub {}: {},", field_name, base_name));
                    base_fields.push((field_name, base_type.clone()));
                    base_idx += 1;
                }
            }
        }

        // Add virtual base pointers and storage if needed
        let vbases_to_add = self.virtual_bases.get(name).cloned().unwrap_or_default();
        for vb in &vbases_to_add {
            let field = self.virtual_base_field_name(vb);
            let storage = self.virtual_base_storage_field_name(vb);
            self.writeln(&format!("/// Virtual base `{}`", vb));
            self.writeln(&format!("pub {}: *mut {},", field, vb));
            self.writeln(&format!("pub {}: Option<Box<{}>>,", storage, vb));
        }

        // Collect and group bit fields, separating regular fields
        let (bit_groups, regular_indices) = self.collect_bit_field_groups(children);

        // Store bit field groups for this struct (for accessor generation)
        if !bit_groups.is_empty() {
            self.bit_field_groups.insert(name.to_string(), bit_groups.clone());
        }

        // Generate bit field storage fields first
        for group in &bit_groups {
            let storage_type = group.storage_type();
            let field_name = format!("_bitfield_{}", group.group_index);
            // Bit field storage is always public for now (accessors control visibility)
            self.writeln(&format!("pub {}: {},", field_name, storage_type));
        }

        // Then collect derived class fields (skip static fields - they become globals)
        // Also flatten anonymous struct fields into parent
        let mut fields = Vec::new();
        for &idx in &regular_indices {
            let child = &children[idx];
            if let ClangNodeKind::FieldDecl { name: fname, ty, is_static, access, bit_field_width } = &child.kind {
                if *is_static || bit_field_width.is_some() {
                    continue; // Static fields handled separately, bit fields handled above
                }
                let sanitized_name = if fname.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(fname)
                };
                let vis = access_to_visibility(*access);
                self.writeln(&format!("{}{}: {},", vis, sanitized_name, ty.to_rust_type_str()));
                fields.push((sanitized_name, ty.clone()));
            } else if let ClangNodeKind::RecordDecl { name: anon_name, .. } = &child.kind {
                // Flatten anonymous struct fields into parent
                if anon_name.starts_with("(anonymous") || anon_name.starts_with("__anon_") {
                    for anon_child in &child.children {
                        if let ClangNodeKind::FieldDecl { name: fname, ty, is_static, access, bit_field_width } = &anon_child.kind {
                            if *is_static || bit_field_width.is_some() {
                                continue;
                            }
                            let sanitized_name = if fname.is_empty() {
                                "_field".to_string()
                            } else {
                                sanitize_identifier(fname)
                            };
                            let vis = access_to_visibility(*access);
                            self.writeln(&format!("{}{}: {},", vis, sanitized_name, ty.to_rust_type_str()));
                            fields.push((sanitized_name, ty.clone()));
                        }
                    }
                }
            } else if let ClangNodeKind::UnionDecl { name: anon_name, .. } = &child.kind {
                // Flatten anonymous union fields into parent
                // In C++, anonymous unions allow direct access to their members from the parent
                if anon_name.starts_with("(anonymous") || anon_name.starts_with("__anon_union_") {
                    for anon_child in &child.children {
                        if let ClangNodeKind::FieldDecl { name: fname, ty, is_static, access, bit_field_width } = &anon_child.kind {
                            if *is_static || bit_field_width.is_some() {
                                continue;
                            }
                            let sanitized_name = if fname.is_empty() {
                                "_field".to_string()
                            } else {
                                sanitize_identifier(fname)
                            };
                            let vis = access_to_visibility(*access);
                            self.writeln(&format!("{}{}: {},", vis, sanitized_name, ty.to_rust_type_str()));
                            fields.push((sanitized_name, ty.clone()));
                        }
                    }
                }
            }
        }

        // Add bit field storage to class fields (for constructor generation)
        // Use the storage type for the bitfield fields
        let mut all_fields = base_fields;
        for group in &bit_groups {
            let storage_type_str = group.storage_type();
            let field_name = format!("_bitfield_{}", group.group_index);
            // Create a CppType for the storage (unsigned integer)
            let storage_type = match storage_type_str {
                "u8" => CppType::Char { signed: false },
                "u16" => CppType::Short { signed: false },
                "u32" => CppType::Int { signed: false },
                _ => CppType::LongLong { signed: false }, // u64 or larger
            };
            all_fields.push((field_name, storage_type));
        }
        all_fields.extend(fields);
        self.class_fields.insert(name.to_string(), all_fields);

        self.indent -= 1;
        self.writeln("}");

        // Generate static member variables as globals
        for child in children {
            if let ClangNodeKind::FieldDecl { name: field_name, ty, is_static: true, .. } = &child.kind {
                let sanitized_field = sanitize_identifier(field_name);
                let rust_ty = ty.to_rust_type_str();
                let global_name = format!("{}_{}", name.to_uppercase(), sanitized_field.to_uppercase());
                self.writeln("");
                self.writeln(&format!("/// Static member `{}::{}`", name, field_name));
                self.writeln(&format!("static mut {}: {} = {};",
                    global_name, rust_ty,
                    Self::default_value_for_type(ty)));
                // Register the static member for later lookup
                self.static_members.insert((name.to_string(), field_name.clone()), global_name);
            }
        }

        // Check if there's an explicit default constructor (0 params)
        let has_default_ctor = children.iter().any(|c| {
            matches!(&c.kind, ClangNodeKind::ConstructorDecl { params, is_definition: true, .. } if params.is_empty())
        });

        // Generate impl block for methods
        let methods: Vec<_> = children.iter().filter(|c| {
            matches!(&c.kind, ClangNodeKind::CXXMethodDecl { is_definition: true, .. } |
                              ClangNodeKind::ConstructorDecl { is_definition: true, .. })
        }).collect();

        // Check if we have bit fields that need accessor methods
        let has_bit_fields = self.bit_field_groups.contains_key(name);

        // Always generate impl block if we need new_0, have other methods, or have bit fields
        if !methods.is_empty() || !has_default_ctor || has_bit_fields {
            self.writeln("");
            self.writeln(&format!("impl {} {{", rust_name));
            self.indent += 1;

            // Generate default new_0() if no explicit default constructor
            if !has_default_ctor {
                self.writeln("pub fn new_0() -> Self {");
                self.indent += 1;
                self.writeln("Default::default()");
                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }

            for method in methods {
                self.generate_method(method, name);
            }

            // Generate bit field accessor methods
            self.generate_bit_field_accessors(name);

            self.indent -= 1;
            self.writeln("}");
        }

        // Generate Drop impl if there's a destructor
        for child in children {
            if let ClangNodeKind::DestructorDecl { is_definition: true, .. } = &child.kind {
                self.writeln("");
                self.writeln(&format!("impl Drop for {} {{", rust_name));
                self.indent += 1;
                self.writeln("fn drop(&mut self) {");
                self.indent += 1;
                // Find the destructor body
                for dtor_child in &child.children {
                    if let ClangNodeKind::CompoundStmt = &dtor_child.kind {
                        self.generate_block_contents(&dtor_child.children, &CppType::Void);
                    }
                }
                self.indent -= 1;
                self.writeln("}");
                self.indent -= 1;
                self.writeln("}");
                break; // Only one destructor per class
            }
        }

        // Generate Clone impl if there's a copy constructor
        for child in children {
            if let ClangNodeKind::ConstructorDecl { ctor_kind: ConstructorKind::Copy, is_definition: true, .. } = &child.kind {
                self.writeln("");
                self.writeln(&format!("impl Clone for {} {{", rust_name));
                self.indent += 1;
                self.writeln("fn clone(&self) -> Self {");
                self.indent += 1;
                // Copy constructor is always new_1 (takes one argument: const T&)
                self.writeln("Self::new_1(self)");
                self.indent -= 1;
                self.writeln("}");
                self.indent -= 1;
                self.writeln("}");
                break; // Only one copy constructor per class
            }
        }

        // Generate trait for polymorphic base classes
        // Only generate trait if this class declares virtual methods AND is not a derived class
        let is_base_class = !self.class_bases.contains_key(name);
        if is_base_class && self.polymorphic_classes.contains(name) {
            if let Some(methods) = self.virtual_methods.get(name).cloned() {
                self.generate_trait_for_class(name, &methods);
                self.generate_trait_impl(name, name, &methods, children, None);
            }
        }

        // If this class derives from polymorphic bases, implement each base's trait
        if let Some(base_infos) = self.class_bases.get(name).cloned() {
            let mut non_virtual_idx = 0;
            for base in base_infos {
                if self.polymorphic_classes.contains(&base.name) {
                    if let Some(methods) = self.virtual_methods.get(&base.name).cloned() {
                        let base_access = if base.is_virtual {
                            BaseAccess::VirtualPtr(self.virtual_base_field_name(&base.name))
                        } else {
                            let field_name = if non_virtual_idx == 0 { "__base".to_string() } else { format!("__base{}", non_virtual_idx) };
                            BaseAccess::DirectField(field_name)
                        };
                        self.generate_trait_impl(name, &base.name, &methods, children, Some(base_access));
                    }
                }
                if !base.is_virtual {
                    non_virtual_idx += 1;
                }
            }
        }

        self.writeln("");
    }

    /// Generate an enum definition.
    fn generate_enum(&mut self, name: &str, is_scoped: bool, underlying_type: &CppType, children: &[ClangNode]) {
        let kind = if is_scoped { "enum class" } else { "enum" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));

        // Generate as Rust enum
        let repr_type = underlying_type.to_rust_type_str();
        self.writeln(&format!("#[repr({})]", repr_type));
        self.writeln("#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]");
        self.writeln(&format!("pub enum {} {{", name));
        self.indent += 1;

        let mut first = true;
        for child in children {
            if let ClangNodeKind::EnumConstantDecl { name: const_name, value } = &child.kind {
                if first {
                    // First variant is the default
                    self.writeln("#[default]");
                    first = false;
                }
                if let Some(v) = value {
                    self.writeln(&format!("{} = {},", const_name, v));
                } else {
                    self.writeln(&format!("{},", const_name));
                }
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a Rust union from a C++ union declaration.
    fn generate_union(&mut self, name: &str, children: &[ClangNode]) {
        // Convert C++ union name to valid Rust identifier
        let rust_name = CppType::Named(name.to_string()).to_rust_type_str();

        // Skip if already generated
        if self.generated_structs.contains(&rust_name) {
            return;
        }
        self.generated_structs.insert(rust_name.clone());

        self.writeln(&format!("/// C++ union `{}`", name));
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Copy, Clone)]");
        self.writeln(&format!("pub union {} {{", rust_name));
        self.indent += 1;

        let mut fields = Vec::new();
        for child in children {
            if let ClangNodeKind::FieldDecl { name: field_name, ty, is_static, access, .. } = &child.kind {
                if *is_static {
                    continue;
                }
                let sanitized_name = if field_name.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(field_name)
                };
                let vis = access_to_visibility(*access);
                self.writeln(&format!("{}{}: {},", vis, sanitized_name, ty.to_rust_type_str()));
                fields.push((sanitized_name, ty.clone()));
            }
        }

        self.indent -= 1;
        self.writeln("}");

        // Generate a Default impl that zeros the union
        self.writeln("");
        self.writeln(&format!("impl Default for {} {{", rust_name));
        self.indent += 1;
        self.writeln("fn default() -> Self {");
        self.indent += 1;
        self.writeln("unsafe { std::mem::zeroed() }");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a type alias for typedef or using declarations.
    fn generate_type_alias(&mut self, name: &str, underlying_type: &CppType) {
        // Convert the underlying C++ type to Rust
        let rust_type = underlying_type.to_rust_type_str();
        self.writeln(&format!("/// C++ typedef/using `{}`", name));
        self.writeln(&format!("pub type {} = {};", name, rust_type));
        self.writeln("");
    }

    /// Generate a global variable declaration.
    fn generate_global_var(&mut self, name: &str, ty: &CppType, _has_init: bool, children: &[ClangNode]) {
        // Track this as a global variable (needs unsafe access)
        self.global_vars.insert(name.to_string());

        let rust_type = ty.to_rust_type_str();
        self.writeln(&format!("/// C++ global variable `{}`", name));

        // Get initial value if present
        // Handle different cases:
        // - Arrays without initializers have IntegerLiteral (size) as first child
        // - Arrays with initializers have InitListExpr as first child
        // - Static member definitions have TypeRef as first child (skip it)
        // - Regular variables have their initializer as first child
        let init_value = if !children.is_empty() {
            // Find the actual initializer, skipping TypeRef for qualified definitions
            let init_idx = if matches!(&children[0].kind, ClangNodeKind::Unknown(s) if s.starts_with("TypeRef:")) {
                // Skip TypeRef child for qualified definitions like "int Counter::count = 0"
                if children.len() > 1 { Some(1) } else { None }
            } else {
                Some(0)
            };

            if let Some(idx) = init_idx {
                let init_node = &children[idx];
                // Check if this is an array type
                if matches!(ty, CppType::Array { .. }) {
                    // For arrays, only use children if the child is an InitListExpr
                    if matches!(&init_node.kind, ClangNodeKind::InitListExpr { .. }) {
                        self.expr_to_string(init_node)
                    } else {
                        // IntegerLiteral child is the array size, not initializer
                        Self::default_value_for_static(ty)
                    }
                } else {
                    // Non-array: the child is the initializer
                    self.expr_to_string(init_node)
                }
            } else {
                Self::default_value_for_static(ty)
            }
        } else {
            // No children: use default value
            Self::default_value_for_static(ty)
        };

        self.writeln(&format!("static mut {}: {} = {};", name, rust_type, init_value));
        self.writeln("");
    }

    /// Generate a const-safe default value for static variables.
    fn default_value_for_static(ty: &CppType) -> String {
        match ty {
            CppType::Int { .. } | CppType::Short { .. } | CppType::Long { .. } | CppType::LongLong { .. }
            | CppType::Char { .. } => "0".to_string(),
            CppType::Float => "0.0f32".to_string(),
            CppType::Double => "0.0f64".to_string(),
            CppType::Bool => "false".to_string(),
            CppType::Pointer { .. } => "std::ptr::null_mut()".to_string(),
            CppType::Array { element, size } => {
                let elem_default = Self::default_value_for_static(element);
                if let Some(n) = size {
                    format!("[{}; {}]", elem_default, n)
                } else {
                    // Unsized arrays shouldn't appear as globals, but fallback
                    "[]".to_string()
                }
            }
            _ => {
                // For named types (structs), try to generate a const default
                // This may fail for complex types, but works for simple cases
                "unsafe { std::mem::zeroed() }".to_string()
            }
        }
    }

    /// Generate a trait for a polymorphic class.
    fn generate_trait_for_class(&mut self, name: &str, methods: &[VirtualMethodInfo]) {
        self.writeln("");
        self.writeln(&format!("/// Trait for polymorphic dispatch of `{}`", name));
        self.writeln(&format!("pub trait {}Trait {{", name));
        self.indent += 1;

        for method in methods {
            let method_name = sanitize_identifier(&method.name);
            let return_type = method.return_type.to_rust_type_str();

            // Build parameter list (skip first param which is self)
            let params: Vec<String> = method.params.iter()
                .map(|(pname, ptype)| format!("{}: {}", sanitize_identifier(pname), ptype.to_rust_type_str()))
                .collect();

            let params_str = if params.is_empty() {
                "&self".to_string()
            } else {
                format!("&self, {}", params.join(", "))
            };

            if return_type == "()" {
                self.writeln(&format!("fn {}({});", method_name, params_str));
            } else {
                self.writeln(&format!("fn {}({}) -> {};", method_name, params_str, return_type));
            }
        }

        self.indent -= 1;
        self.writeln("}");
    }

    /// Generate trait implementation for a class.
    /// base_access is the access path to delegate to if this is a derived class.
    fn generate_trait_impl(&mut self, class_name: &str, trait_class: &str, methods: &[VirtualMethodInfo], children: &[ClangNode], base_access: Option<BaseAccess>) {
        self.writeln("");
        self.writeln(&format!("impl {}Trait for {} {{", trait_class, class_name));
        self.indent += 1;

        for method in methods {
            let method_name = sanitize_identifier(&method.name);
            let return_type = method.return_type.to_rust_type_str();

            // Build parameter list
            let params: Vec<String> = method.params.iter()
                .map(|(pname, ptype)| format!("{}: {}", sanitize_identifier(pname), ptype.to_rust_type_str()))
                .collect();

            let params_str = if params.is_empty() {
                "&self".to_string()
            } else {
                format!("&self, {}", params.join(", "))
            };

            // Check if this class has an override for this method
            let has_override = children.iter().any(|c| {
                matches!(&c.kind, ClangNodeKind::CXXMethodDecl { name, is_definition: true, .. }
                    if name == &method.name)
            });

            if return_type == "()" {
                self.writeln(&format!("fn {}({}) {{", method_name, params_str));
            } else {
                self.writeln(&format!("fn {}({}) -> {} {{", method_name, params_str, return_type));
            }
            self.indent += 1;

            if has_override || class_name == trait_class {
                // Call the actual method on self
                let args: Vec<String> = method.params.iter()
                    .map(|(pname, _)| sanitize_identifier(pname))
                    .collect();
                if args.is_empty() {
                    self.writeln(&format!("self.{}()", method_name));
                } else {
                    self.writeln(&format!("self.{}({})", method_name, args.join(", ")));
                }
            } else if let Some(ref base_access) = base_access {
                // Delegate to the correct base class field
                let args: Vec<String> = method.params.iter()
                    .map(|(pname, _)| sanitize_identifier(pname))
                    .collect();
                match base_access {
                    BaseAccess::VirtualPtr(field) => {
                        if args.is_empty() {
                            self.writeln(&format!("unsafe {{ (*self.{}).{}() }}", field, method_name));
                        } else {
                            self.writeln(&format!("unsafe {{ (*self.{}).{}({}) }}", field, method_name, args.join(", ")));
                        }
                    }
                    BaseAccess::DirectField(field) | BaseAccess::FieldChain(field) => {
                        if args.is_empty() {
                            self.writeln(&format!("self.{}.{}()", field, method_name));
                        } else {
                            self.writeln(&format!("self.{}.{}({})", field, method_name, args.join(", ")));
                        }
                    }
                }
            } else {
                // Fallback to __base (shouldn't happen with proper calls)
                let args: Vec<String> = method.params.iter()
                    .map(|(pname, _)| sanitize_identifier(pname))
                    .collect();
                if args.is_empty() {
                    self.writeln(&format!("self.__base.{}()", method_name));
                } else {
                    self.writeln(&format!("self.__base.{}({})", method_name, args.join(", ")));
                }
            }

            self.indent -= 1;
            self.writeln("}");
        }

        self.indent -= 1;
        self.writeln("}");
    }

    /// Convert a type to Rust, using trait objects for polymorphic pointers.
    fn convert_type_for_polymorphism(&self, ty: &CppType) -> String {
        match ty {
            CppType::Pointer { pointee, is_const } => {
                // Check if pointee is a polymorphic class
                if let CppType::Named(class_name) = pointee.as_ref() {
                    if self.polymorphic_classes.contains(class_name) {
                        // Convert to trait object reference
                        let trait_name = format!("{}Trait", class_name);
                        return if *is_const {
                            format!("&dyn {}", trait_name)
                        } else {
                            format!("&mut dyn {}", trait_name)
                        };
                    }
                }
                // Not polymorphic, use regular pointer type
                ty.to_rust_type_str()
            }
            _ => ty.to_rust_type_str(),
        }
    }

    /// Check if a method modifies self (has assignments to member fields).
    fn method_modifies_self(node: &ClangNode) -> bool {
        // Check if this node is an assignment to a member
        if let ClangNodeKind::BinaryOperator { op: BinaryOp::Assign, .. } = &node.kind {
            // Left side of assignment - check if it's a member expression
            if !node.children.is_empty() {
                if Self::is_member_access(&node.children[0]) {
                    return true;
                }
            }
        }
        // Also check compound assignment operators
        if let ClangNodeKind::BinaryOperator { op, .. } = &node.kind {
            match op {
                BinaryOp::AddAssign | BinaryOp::SubAssign | BinaryOp::MulAssign |
                BinaryOp::DivAssign | BinaryOp::RemAssign | BinaryOp::AndAssign |
                BinaryOp::OrAssign | BinaryOp::XorAssign | BinaryOp::ShlAssign |
                BinaryOp::ShrAssign => {
                    if !node.children.is_empty() {
                        if Self::is_member_access(&node.children[0]) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        // Check for increment/decrement operators on member fields
        if let ClangNodeKind::UnaryOperator { op, .. } = &node.kind {
            match op {
                UnaryOp::PreInc | UnaryOp::PostInc | UnaryOp::PreDec | UnaryOp::PostDec => {
                    if !node.children.is_empty() {
                        if Self::is_member_access(&node.children[0]) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        // Recursively check children
        for child in &node.children {
            if Self::method_modifies_self(child) {
                return true;
            }
        }
        false
    }

    /// Check if a node is a member access (directly or through implicit this).
    fn is_member_access(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::MemberExpr { .. } => true,
            ClangNodeKind::ImplicitCastExpr { .. } => {
                // Check through implicit casts
                !node.children.is_empty() && Self::is_member_access(&node.children[0])
            }
            _ => false,
        }
    }

    /// Extract member assignments from a constructor body.
    /// Looks for patterns like `this->field = value;` or `field = value;`
    fn extract_member_assignments(node: &ClangNode, initializers: &mut Vec<(String, String)>, codegen: &AstCodeGen) {
        for child in &node.children {
            // Look for ExprStmt containing BinaryOperator with Assign
            if let ClangNodeKind::ExprStmt = &child.kind {
                if !child.children.is_empty() {
                    Self::extract_assignment(&child.children[0], initializers, codegen);
                }
            } else if let ClangNodeKind::BinaryOperator { op: BinaryOp::Assign, .. } = &child.kind {
                Self::extract_assignment(child, initializers, codegen);
            }
            // Recursively check compound statements
            if let ClangNodeKind::CompoundStmt = &child.kind {
                Self::extract_member_assignments(child, initializers, codegen);
            }
        }
    }

    /// Extract a single member assignment from a BinaryOperator node.
    fn extract_assignment(node: &ClangNode, initializers: &mut Vec<(String, String)>, codegen: &AstCodeGen) {
        if let ClangNodeKind::BinaryOperator { op: BinaryOp::Assign, .. } = &node.kind {
            if node.children.len() >= 2 {
                // Get member name from left side
                if let Some(member_name) = Self::get_member_name(&node.children[0]) {
                    // Get value from right side
                    let value = codegen.expr_to_string(&node.children[1]);
                    initializers.push((member_name, value));
                }
            }
        }
    }

    /// Get member name from a member expression (possibly wrapped in casts).
    fn get_member_name(node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::MemberExpr { member_name, .. } => Some(member_name.clone()),
            ClangNodeKind::ImplicitCastExpr { .. } => {
                if !node.children.is_empty() {
                    Self::get_member_name(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if a statement is a member field assignment (for filtering in ctor body)
    fn is_member_assignment(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::ExprStmt => {
                if !node.children.is_empty() {
                    return Self::is_member_assignment(&node.children[0]);
                }
                false
            }
            ClangNodeKind::BinaryOperator { op: BinaryOp::Assign, .. } => {
                if node.children.len() >= 2 {
                    // Check if left side is a member access (instance field)
                    if let Some(_name) = Self::get_member_name(&node.children[0]) {
                        // Check if it's a non-static member (has implicit this)
                        // Static members use DeclRefExpr, not MemberExpr with implicit this
                        return Self::has_implicit_this_or_member(&node.children[0]);
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if a node is a member expression with implicit this (instance member)
    fn has_implicit_this_or_member(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::MemberExpr { is_static, .. } => {
                // Non-static member expressions with no children have implicit this
                !*is_static && node.children.is_empty()
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                if !node.children.is_empty() {
                    Self::has_implicit_this_or_member(&node.children[0])
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if a constructor compound statement has non-member statements
    fn has_non_member_ctor_stmts(compound_stmt: &ClangNode) -> bool {
        for child in &compound_stmt.children {
            // Skip member field assignments
            if Self::is_member_assignment(child) {
                continue;
            }
            // Any other statement means we have non-member statements
            match &child.kind {
                ClangNodeKind::CompoundStmt => {
                    if Self::has_non_member_ctor_stmts(child) {
                        return true;
                    }
                }
                _ => return true,
            }
        }
        false
    }

    /// Generate non-member statements from constructor body (like static member modifications)
    fn generate_non_member_ctor_stmts(&mut self, compound_stmt: &ClangNode) {
        for child in &compound_stmt.children {
            // Skip member field assignments - those are handled in struct initializer
            if Self::is_member_assignment(child) {
                continue;
            }

            // Generate the statement
            match &child.kind {
                ClangNodeKind::ExprStmt => {
                    if !child.children.is_empty() {
                        let expr = self.expr_to_string(&child.children[0]);
                        self.writeln(&format!("{};", expr));
                    }
                }
                ClangNodeKind::CompoundStmt => {
                    // Recursively handle nested compound statements
                    self.generate_non_member_ctor_stmts(child);
                }
                _ => {
                    // For other statement types, generate them
                    self.generate_stmt(child, false);
                }
            }
        }
    }

    /// Extract constructor arguments from a CallExpr or CXXConstructExpr node.
    fn extract_constructor_args(&self, node: &ClangNode) -> Vec<String> {
        let mut args = Vec::new();
        match &node.kind {
            ClangNodeKind::CallExpr { .. } => {
                // Arguments are children of the call expression
                for child in &node.children {
                    // Skip type references and function references
                    match &child.kind {
                        ClangNodeKind::Unknown(s) if s == "TypeRef" => continue,
                        ClangNodeKind::DeclRefExpr { .. } |
                        ClangNodeKind::IntegerLiteral { .. } |
                        ClangNodeKind::FloatingLiteral { .. } |
                        ClangNodeKind::BoolLiteral(_) |
                        ClangNodeKind::ImplicitCastExpr { .. } |
                        ClangNodeKind::BinaryOperator { .. } |
                        ClangNodeKind::UnaryOperator { .. } => {
                            args.push(self.expr_to_string(child));
                        }
                        _ => {
                            // Try to convert other expression types
                            let expr = self.expr_to_string(child);
                            if !expr.contains("unsupported") && !expr.is_empty() {
                                args.push(expr);
                            }
                        }
                    }
                }
            }
            // Handle implicit casts wrapping the construct expression
            ClangNodeKind::ImplicitCastExpr { .. } => {
                if !node.children.is_empty() {
                    return self.extract_constructor_args(&node.children[0]);
                }
            }
            _ => {}
        }
        args
    }

    /// Check if a node is a pointer dereference (possibly wrapped in casts).
    fn is_pointer_deref(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::UnaryOperator { op: UnaryOp::Deref, .. } => true,
            ClangNodeKind::ImplicitCastExpr { .. } => {
                !node.children.is_empty() && Self::is_pointer_deref(&node.children[0])
            }
            _ => false,
        }
    }

    /// Check if a node is an arrow member access (needs unsafe).
    fn is_arrow_member_access(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::MemberExpr { is_arrow, .. } => *is_arrow,
            ClangNodeKind::ImplicitCastExpr { .. } => {
                !node.children.is_empty() && Self::is_arrow_member_access(&node.children[0])
            }
            _ => false,
        }
    }

    /// Check if a node is an array subscript on a pointer (needs unsafe for assignment).
    fn is_pointer_subscript(&self, node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::ArraySubscriptExpr { .. } => {
                if !node.children.is_empty() {
                    // Check if the array expression is a pointer type
                    let arr_type = Self::get_expr_type(&node.children[0]);
                    matches!(arr_type, Some(CppType::Pointer { .. }))
                        || matches!(arr_type, Some(CppType::Array { size: None, .. }))
                        || self.is_ptr_var_expr(&node.children[0])
                } else {
                    false
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                !node.children.is_empty() && self.is_pointer_subscript(&node.children[0])
            }
            _ => false,
        }
    }

    /// Check if a node is an array subscript on a global array (needs unsafe for assignment).
    fn is_global_array_subscript(&self, node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::ArraySubscriptExpr { .. } => {
                if !node.children.is_empty() {
                    self.is_global_var_expr(&node.children[0])
                } else {
                    false
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                !node.children.is_empty() && self.is_global_array_subscript(&node.children[0])
            }
            _ => false,
        }
    }

    /// Check if a node is a static member access (needs unsafe for assignment).
    fn is_static_member_access(&self, node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::MemberExpr { is_static, .. } => *is_static,
            ClangNodeKind::DeclRefExpr { ty, namespace_path, name, .. } => {
                // Static members accessed via Class::member have namespace_path with class name
                if !namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                    return true;
                }
                // Also check if this is a static member of the current class (accessed without Class:: prefix)
                if namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                    if let Some(ref current_class) = self.current_class {
                        if self.static_members.contains_key(&(current_class.clone(), name.clone())) {
                            return true;
                        }
                    }
                }
                false
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                !node.children.is_empty() && self.is_static_member_access(&node.children[0])
            }
            _ => false,
        }
    }

    /// Get the raw identifier for a reference variable expression (without dereferencing).
    /// Returns None if not a reference variable expression.
    fn get_ref_var_ident(&self, node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, .. } => {
                if self.ref_vars.contains(name) {
                    Some(sanitize_identifier(name))
                } else {
                    None
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                if !node.children.is_empty() {
                    self.get_ref_var_ident(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if an expression is a pointer variable (parameter or local with pointer type).
    fn is_ptr_var_expr(&self, node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, .. } => {
                self.ptr_vars.contains(name)
            }
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                // Look through casts and unknown wrappers
                !node.children.is_empty() && self.is_ptr_var_expr(&node.children[0])
            }
            _ => {
                // Also check all children recursively for cases where the structure differs
                node.children.iter().any(|c| self.is_ptr_var_expr(c))
            }
        }
    }

    /// Check if an expression node refers to a global variable (needs unsafe access).
    fn is_global_var_expr(&self, node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, .. } => {
                self.global_vars.contains(name)
            }
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                // Look through casts and unknown wrappers
                !node.children.is_empty() && self.is_global_var_expr(&node.children[0])
            }
            _ => false,
        }
    }

    /// Get the raw variable name from a DeclRefExpr (unwrapping casts).
    fn get_raw_var_name(&self, node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, .. } => Some(sanitize_identifier(name)),
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                if !node.children.is_empty() {
                    self.get_raw_var_name(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if an expression is an array variable and get its identifier.
    fn get_array_var_ident(&self, node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, ty, .. } => {
                // Check both the type from AST and our tracked arrays
                if matches!(ty, CppType::Array { .. }) || self.arr_vars.contains(name) {
                    Some(sanitize_identifier(name))
                } else {
                    None
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                // Look through casts and unknown wrappers
                if !node.children.is_empty() {
                    self.get_array_var_ident(&node.children[0])
                } else {
                    None
                }
            }
            _ => {
                // Also check children recursively
                for child in &node.children {
                    if let Some(ident) = self.get_array_var_ident(child) {
                        return Some(ident);
                    }
                }
                None
            }
        }
    }

    /// Get the type of an expression node.
    fn get_expr_type(node: &ClangNode) -> Option<CppType> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { ty, .. } => Some(ty.clone()),
            ClangNodeKind::BinaryOperator { ty, .. } => Some(ty.clone()),
            ClangNodeKind::UnaryOperator { ty, .. } => Some(ty.clone()),
            ClangNodeKind::MemberExpr { ty, .. } => Some(ty.clone()),
            ClangNodeKind::CallExpr { ty } => Some(ty.clone()),
            ClangNodeKind::ImplicitCastExpr { ty, .. } => Some(ty.clone()),
            ClangNodeKind::CastExpr { ty, .. } => Some(ty.clone()),
            ClangNodeKind::ArraySubscriptExpr { ty } => Some(ty.clone()),
            ClangNodeKind::ParmVarDecl { ty, .. } => Some(ty.clone()),
            // Literal types
            ClangNodeKind::EvaluatedExpr { ty, .. } => Some(ty.clone()),
            ClangNodeKind::IntegerLiteral { cpp_type, .. } => cpp_type.clone(),
            ClangNodeKind::FloatingLiteral { cpp_type, .. } => cpp_type.clone(),
            ClangNodeKind::BoolLiteral(_) => Some(CppType::Bool),
            ClangNodeKind::StringLiteral(_) => Some(CppType::Named("const char*".to_string())),
            // For unknown or wrapper nodes, look through to children
            ClangNodeKind::Unknown(_) | ClangNodeKind::ParenExpr { .. } => {
                if !node.children.is_empty() {
                    Self::get_expr_type(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Extract the class name from a type, handling const qualifiers, references, and pointers.
    /// For example, "const Point" -> "Point", Reference { pointee: Named("Point") } -> "Point"
    fn extract_class_name(ty: &Option<CppType>) -> Option<String> {
        ty.as_ref().and_then(|t| Self::extract_class_name_from_type(t))
    }

    /// Helper to extract class name from a CppType.
    fn extract_class_name_from_type(ty: &CppType) -> Option<String> {
        match ty {
            CppType::Named(name) => {
                // Strip "const " prefix if present
                let stripped = name.strip_prefix("const ").unwrap_or(name);
                Some(stripped.to_string())
            }
            CppType::Reference { referent, .. } => Self::extract_class_name_from_type(referent),
            CppType::Pointer { pointee, .. } => Self::extract_class_name_from_type(pointee),
            _ => None,
        }
    }

    /// Get the base access path for a member declared in a specific base class.
    fn get_base_access_for_class(&self, current_class: &str, declaring_class: &str) -> BaseAccess {
        if let Some(vbases) = self.virtual_bases.get(current_class) {
            if vbases.iter().any(|b| b == declaring_class) {
                return BaseAccess::VirtualPtr(self.virtual_base_field_name(declaring_class));
            }
        }

        if let Some(base_classes) = self.class_bases.get(current_class) {
            let mut non_virtual_idx = 0;
            for base in base_classes {
                if base.name == declaring_class {
                    if base.is_virtual {
                        return BaseAccess::VirtualPtr(self.virtual_base_field_name(declaring_class));
                    }
                    let field = if non_virtual_idx == 0 { "__base".to_string() } else { format!("__base{}", non_virtual_idx) };
                    return BaseAccess::DirectField(field);
                }
                if !base.is_virtual {
                    non_virtual_idx += 1;
                }
            }

            // Declaring class not found in immediate bases - could be transitive
            for (base_idx, base) in base_classes.iter().enumerate() {
                if let Some(base_bases) = self.class_bases.get(&base.name) {
                    if base_bases.iter().any(|b| b.name == declaring_class) {
                        // Declaring class is in the chain of this base
                        let mut non_virtual_base_idx = 0;
                        for (i, b) in base_classes.iter().enumerate() {
                            if i == base_idx {
                                break;
                            }
                            if !b.is_virtual {
                                non_virtual_base_idx += 1;
                            }
                        }
                        let first_base = if non_virtual_base_idx == 0 { "__base".to_string() } else { format!("__base{}", non_virtual_base_idx) };
                        return BaseAccess::FieldChain(format!("{}.__base", first_base));
                    }
                }
            }
        }

        BaseAccess::DirectField("__base".to_string())
    }

    /// Get function parameter types from a function reference node.
    fn get_function_param_types(node: &ClangNode) -> Option<Vec<CppType>> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { ty, .. } => {
                if let CppType::Function { params, .. } = ty {
                    Some(params.clone())
                } else {
                    None
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                // Look through casts (e.g., FunctionToPointerDecay)
                if !node.children.is_empty() {
                    Self::get_function_param_types(&node.children[0])
                } else {
                    None
                }
            }
            ClangNodeKind::Unknown(_) => {
                // Unknown nodes (like UnexposedExpr) may wrap DeclRefExpr, recurse
                if !node.children.is_empty() {
                    Self::get_function_param_types(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if a MemberExpr (possibly wrapped) is a virtual base method call.
    /// Returns Some((base_expr, vbase_field, method_name)) if it is.
    fn get_virtual_base_method_call_info(&self, node: &ClangNode) -> Option<(String, String, String)> {
        let member_node = match &node.kind {
            ClangNodeKind::MemberExpr { .. } => node,
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                if !node.children.is_empty() {
                    return self.get_virtual_base_method_call_info(&node.children[0]);
                }
                return None;
            }
            _ => return None,
        };

        if let ClangNodeKind::MemberExpr { member_name, declaring_class, is_static, .. } = &member_node.kind {
            // Only care about non-static members
            if *is_static {
                return None;
            }

            if !member_node.children.is_empty() {
                let base_type = Self::get_expr_type(&member_node.children[0]);

                if let Some(decl_class) = declaring_class {
                    let base_class_name = Self::extract_class_name(&base_type);
                    if let Some(name) = base_class_name {
                        if name != *decl_class {
                            // Check if declaring class is a virtual base
                            let access = self.get_base_access_for_class(&name, decl_class);
                            if let BaseAccess::VirtualPtr(field) = access {
                                let base = self.expr_to_string(&member_node.children[0]);
                                let method = sanitize_identifier(member_name);
                                return Some((base, field, method));
                            }
                        }
                    }
                }
            } else {
                // Implicit this
                if let (Some(current), Some(decl_class)) = (&self.current_class, declaring_class) {
                    if current != decl_class {
                        let access = self.get_base_access_for_class(current, decl_class);
                        if let BaseAccess::VirtualPtr(field) = access {
                            let method = sanitize_identifier(member_name);
                            return Some(("self".to_string(), field, method));
                        }
                    }
                }
            }
        }
        None
    }

    /// Get a default value for a C++ type (for static member initialization).
    fn default_value_for_type(ty: &CppType) -> String {
        match ty {
            CppType::Int { .. } | CppType::Long { .. } | CppType::Short { .. } |
            CppType::Char { .. } | CppType::LongLong { .. } => "0".to_string(),
            CppType::Float => "0.0f32".to_string(),
            CppType::Double => "0.0f64".to_string(),
            CppType::Bool => "false".to_string(),
            CppType::Pointer { .. } => "std::ptr::null_mut()".to_string(),
            CppType::Array { element, size } => {
                let elem_default = Self::default_value_for_type(element);
                if let Some(n) = size {
                    format!("[{}; {}]", elem_default, n)
                } else {
                    "[]".to_string()
                }
            }
            _ => "Default::default()".to_string(),
        }
    }

    /// Check if a CallExpr is an operator overload call.
    /// Returns Some((operator_name, left_operand_index, right_operand_index)) for binary operators,
    /// or Some((operator_name, operand_index, None)) for unary operators or operator() calls.
    fn get_operator_call_info(node: &ClangNode) -> Option<(String, usize, Option<usize>)> {
        // Operator calls have the pattern:
        // CallExpr
        //   UnexposedExpr -> left_operand
        //   UnexposedExpr -> DeclRefExpr { name: "operator+" }
        //   UnexposedExpr -> right_operand (for binary)
        // For operator() (function call operator), pattern is:
        //   UnexposedExpr -> callee
        //   UnexposedExpr -> DeclRefExpr { name: "operator()" }
        //   args...
        for (i, child) in node.children.iter().enumerate() {
            if let Some(op_name) = Self::find_operator_name(child) {
                if op_name.starts_with("operator") {
                    // Found an operator - determine type
                    if op_name == "operator()" {
                        // Function call operator: callee is before the operator ref
                        let callee = if i > 0 { i - 1 } else { 0 };
                        return Some((op_name, callee, None));
                    } else if node.children.len() == 3 {
                        // Binary operator: left is before, right is after
                        let left = if i > 0 { i - 1 } else { 0 };
                        let right = if i + 1 < node.children.len() { i + 1 } else { i };
                        return Some((op_name, left, Some(right)));
                    } else if node.children.len() == 2 {
                        // Unary operator
                        let operand = if i == 0 { 1 } else { 0 };
                        return Some((op_name, operand, None));
                    }
                }
            }
        }
        None
    }

    /// Check if a CallExpr is an explicit destructor call (obj->~ClassName() or obj.~ClassName()).
    /// Returns Some(pointer_expression) if it is, where the pointer can be passed to drop_in_place.
    fn get_explicit_destructor_call(&self, node: &ClangNode) -> Option<String> {
        // Explicit destructor calls have a MemberExpr child with member_name starting with "~"
        if !node.children.is_empty() {
            // The first child should be the MemberExpr for the destructor
            let child = &node.children[0];
            if let ClangNodeKind::MemberExpr { member_name, is_arrow, .. } = &child.kind {
                if member_name.starts_with('~') {
                    // This is an explicit destructor call
                    // Get the object/pointer expression from the MemberExpr's child
                    if !child.children.is_empty() {
                        if *is_arrow {
                            // ptr->~ClassName() - ptr is already a pointer
                            let obj_expr = self.expr_to_string(&child.children[0]);
                            return Some(obj_expr);
                        } else {
                            // obj.~ClassName() - check if obj is actually a deref of a pointer (*ptr)
                            // In that case, we can just use ptr directly
                            if let Some(ptr_expr) = Self::get_deref_pointer(&child.children[0]) {
                                return Some(self.expr_to_string(ptr_expr));
                            }
                            // Otherwise, need to take address
                            let obj_expr = self.expr_to_string(&child.children[0]);
                            return Some(format!("&mut {}", obj_expr));
                        }
                    }
                }
            }
            // Also check through wrapper nodes (UnexposedExpr, ImplicitCastExpr)
            if let ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } = &child.kind {
                if !child.children.is_empty() {
                    return self.get_explicit_destructor_call_inner(&child.children[0]);
                }
            }
        }
        None
    }

    /// Helper for get_explicit_destructor_call that checks inner nodes.
    fn get_explicit_destructor_call_inner(&self, node: &ClangNode) -> Option<String> {
        if let ClangNodeKind::MemberExpr { member_name, is_arrow, .. } = &node.kind {
            if member_name.starts_with('~') {
                if !node.children.is_empty() {
                    if *is_arrow {
                        let obj_expr = self.expr_to_string(&node.children[0]);
                        return Some(obj_expr);
                    } else {
                        if let Some(ptr_expr) = Self::get_deref_pointer(&node.children[0]) {
                            return Some(self.expr_to_string(ptr_expr));
                        }
                        let obj_expr = self.expr_to_string(&node.children[0]);
                        return Some(format!("&mut {}", obj_expr));
                    }
                }
            }
        }
        None
    }

    /// Check if a node is a dereference of a pointer (like *ptr or (*ptr)).
    /// Returns the pointer expression if so.
    fn get_deref_pointer(node: &ClangNode) -> Option<&ClangNode> {
        match &node.kind {
            ClangNodeKind::UnaryOperator { op: UnaryOp::Deref, .. } => {
                // *ptr - return the ptr
                if !node.children.is_empty() {
                    return Some(&node.children[0]);
                }
            }
            ClangNodeKind::ParenExpr { .. } => {
                // (...) - look inside
                if !node.children.is_empty() {
                    return Self::get_deref_pointer(&node.children[0]);
                }
            }
            _ => {}
        }
        None
    }

    /// Check if a node is a function reference (DeclRefExpr with Function type).
    fn is_function_reference(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { ty, .. } => {
                matches!(ty, CppType::Function { .. })
            }
            ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } => {
                // Look through wrapper nodes
                node.children.iter().any(|c| Self::is_function_reference(c))
            }
            _ => false,
        }
    }

    /// Strip `Some(...)` wrapper from a string if present.
    /// Used for function call callees where FunctionToPointerDecay shouldn't wrap.
    fn strip_some_wrapper(s: &str) -> String {
        if s.starts_with("Some(") && s.ends_with(")") {
            // Extract inner part
            s[5..s.len()-1].to_string()
        } else {
            s.to_string()
        }
    }

    /// Check if a node is a function pointer variable (not a direct function reference).
    /// Returns true if the node has type Pointer { pointee: Function { .. } }
    fn is_function_pointer_variable(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { ty, .. } => {
                matches!(ty, CppType::Pointer { pointee, .. } if matches!(pointee.as_ref(), CppType::Function { .. }))
            }
            ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } => {
                // Look through wrapper nodes (but not FunctionToPointerDecay)
                node.children.iter().any(|c| Self::is_function_pointer_variable(c))
            }
            _ => false,
        }
    }

    /// Check if a node is a nullptr literal (possibly wrapped in Unknown nodes).
    fn is_nullptr_literal(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::NullPtrLiteral => true,
            ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } => {
                // Look through wrapper nodes
                node.children.iter().any(|c| Self::is_nullptr_literal(c))
            }
            _ => false,
        }
    }

    /// Check if a type is a function pointer type.
    fn is_function_pointer_type(ty: &CppType) -> bool {
        matches!(ty, CppType::Pointer { pointee, .. } if matches!(pointee.as_ref(), CppType::Function { .. }))
    }

    /// Recursively find an operator name in a node tree.
    fn find_operator_name(node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, ty, .. } => {
                // Check if this is an operator function reference
                if name.starts_with("operator") {
                    // Also verify it's a function type
                    if matches!(ty, CppType::Function { .. }) {
                        return Some(name.clone());
                    }
                }
                None
            }
            ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } => {
                // Look through wrapper nodes
                for child in &node.children {
                    if let Some(op) = Self::find_operator_name(child) {
                        return Some(op);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if an expression is an I/O stream (stdout, stderr, or stdin).
    /// Returns the stream type if it is.
    fn get_io_stream_type(node: &ClangNode) -> Option<&'static str> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, namespace_path, .. } => {
                let is_std = namespace_path.len() == 1 && namespace_path[0] == "std";
                if is_std || namespace_path.is_empty() {
                    match name.as_str() {
                        "cout" => Some("stdout"),
                        "cerr" | "clog" => Some("stderr"),
                        "cin" => Some("stdin"),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } => {
                // Look through wrapper nodes
                for child in &node.children {
                    if let Some(stream) = Self::get_io_stream_type(child) {
                        return Some(stream);
                    }
                }
                None
            }
            ClangNodeKind::CallExpr { .. } => {
                // A chained operator<< also returns an ostream - check if this is one
                if let Some((op_name, left_idx, _)) = Self::get_operator_call_info(node) {
                    if op_name == "operator<<" || op_name == "operator>>" {
                        if !node.children.is_empty() && left_idx < node.children.len() {
                            return Self::get_io_stream_type(&node.children[left_idx]);
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if an expression is std::endl or std::flush.
    fn is_stream_manipulator(node: &ClangNode) -> Option<&'static str> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, namespace_path, .. } => {
                let is_std = namespace_path.len() == 1 && namespace_path[0] == "std";
                if is_std || namespace_path.is_empty() {
                    match name.as_str() {
                        "endl" => Some("newline"),
                        "flush" => Some("flush"),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } => {
                for child in &node.children {
                    if let Some(manip) = Self::is_stream_manipulator(child) {
                        return Some(manip);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if a node contains a TypeidExpr (possibly wrapped in Unknown/ImplicitCast).
    fn contains_typeid_expr(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::TypeidExpr { .. } => true,
            ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } => {
                node.children.iter().any(|c| Self::contains_typeid_expr(c))
            }
            _ => false,
        }
    }

    /// Collect all output arguments from a chained operator<< expression.
    /// Returns (stream_type, args_in_order) where args_in_order is left-to-right.
    fn collect_stream_output_args<'a>(
        &self,
        node: &'a ClangNode,
    ) -> Option<(&'static str, Vec<&'a ClangNode>)> {
        // This recursively collects arguments from chained << operators
        // cout << a << b << endl  is  ((cout << a) << b) << endl
        if let Some((op_name, left_idx, right_idx_opt)) = Self::get_operator_call_info(node) {
            if op_name == "operator<<" {
                if let Some(right_idx) = right_idx_opt {
                    if left_idx < node.children.len() && right_idx < node.children.len() {
                        // First check if left operand is directly a stream
                        if let Some(stream_type) = Self::get_io_stream_type(&node.children[left_idx]) {
                            // Base case: stream << arg
                            return Some((stream_type, vec![&node.children[right_idx]]));
                        }
                        // Recursive case: (stream << ...) << arg
                        // Check if left operand is another operator<< on a stream
                        if let Some((stream_type, mut args)) = self.collect_stream_output_args(&node.children[left_idx]) {
                            args.push(&node.children[right_idx]);
                            return Some((stream_type, args));
                        }
                    }
                }
            }
        }
        None
    }

    /// Generate a write!() or writeln!() macro call from stream output arguments.
    fn generate_stream_write(&self, stream_type: &str, args: &[&ClangNode]) -> String {
        let stream_expr = match stream_type {
            "stdout" => "std::io::stdout()",
            "stderr" => "std::io::stderr()",
            _ => "std::io::stdout()", // fallback
        };

        // Check if the last argument is std::endl
        let has_newline = args.last().map_or(false, |arg| {
            Self::is_stream_manipulator(arg) == Some("newline")
        });

        // Filter out endl/flush manipulators, collect format args
        let format_args: Vec<String> = args.iter()
            .filter(|arg| Self::is_stream_manipulator(arg).is_none())
            .map(|arg| self.expr_to_string(arg))
            .collect();

        if format_args.is_empty() {
            // Just endl or flush with no content
            if has_newline {
                format!("writeln!({}).unwrap()", stream_expr)
            } else {
                format!("{{ let _ = {}.flush(); {} }}", stream_expr, stream_expr)
            }
        } else {
            // Build format string with {} placeholders
            let format_str = vec!["{}"; format_args.len()].join("");
            let args_str = format_args.join(", ");
            if has_newline {
                format!("writeln!({}, \"{}\", {}).unwrap()", stream_expr, format_str, args_str)
            } else {
                format!("write!({}, \"{}\", {}).unwrap()", stream_expr, format_str, args_str)
            }
        }
    }

    /// Collect all input arguments from a chained operator>> expression.
    /// Returns (stream_type, args_in_order) where args_in_order is left-to-right.
    fn collect_stream_input_args<'a>(
        &self,
        node: &'a ClangNode,
    ) -> Option<(&'static str, Vec<&'a ClangNode>)> {
        // This recursively collects arguments from chained >> operators
        // cin >> a >> b  is  ((cin >> a) >> b)
        if let Some((op_name, left_idx, right_idx_opt)) = Self::get_operator_call_info(node) {
            if op_name == "operator>>" {
                if let Some(right_idx) = right_idx_opt {
                    if left_idx < node.children.len() && right_idx < node.children.len() {
                        // First check if left operand is directly a stream
                        if let Some(stream_type) = Self::get_io_stream_type(&node.children[left_idx]) {
                            if stream_type == "stdin" {
                                // Base case: stream >> arg
                                return Some((stream_type, vec![&node.children[right_idx]]));
                            }
                        }
                        // Recursive case: (stream >> ...) >> arg
                        if let Some((stream_type, mut args)) = self.collect_stream_input_args(&node.children[left_idx]) {
                            args.push(&node.children[right_idx]);
                            return Some((stream_type, args));
                        }
                    }
                }
            }
        }
        None
    }

    /// Generate Rust code for reading from stdin and parsing into variables.
    fn generate_stream_read(&self, args: &[&ClangNode]) -> String {
        // Generate code that reads a line from stdin and parses it into the variables
        // For chained reads like cin >> x >> y, we read one line and split by whitespace
        let var_reads: Vec<String> = args.iter().map(|arg| {
            let var_name = self.expr_to_string(arg);
            let var_type = Self::get_expr_type(arg);

            // Generate appropriate parse call based on type
            let parse_expr = match var_type {
                Some(CppType::Int { signed: true }) => "__parts.next().unwrap().parse::<i32>().unwrap()".to_string(),
                Some(CppType::Int { signed: false }) => "__parts.next().unwrap().parse::<u32>().unwrap()".to_string(),
                Some(CppType::Long { signed: true }) | Some(CppType::LongLong { signed: true }) => {
                    "__parts.next().unwrap().parse::<i64>().unwrap()".to_string()
                }
                Some(CppType::Long { signed: false }) | Some(CppType::LongLong { signed: false }) => {
                    "__parts.next().unwrap().parse::<u64>().unwrap()".to_string()
                }
                Some(CppType::Short { signed: true }) => "__parts.next().unwrap().parse::<i16>().unwrap()".to_string(),
                Some(CppType::Short { signed: false }) => "__parts.next().unwrap().parse::<u16>().unwrap()".to_string(),
                Some(CppType::Float) => "__parts.next().unwrap().parse::<f32>().unwrap()".to_string(),
                Some(CppType::Double) => "__parts.next().unwrap().parse::<f64>().unwrap()".to_string(),
                Some(CppType::Char { signed: true }) => "__parts.next().unwrap().chars().next().unwrap() as i8".to_string(),
                Some(CppType::Char { signed: false }) => "__parts.next().unwrap().chars().next().unwrap() as u8".to_string(),
                Some(CppType::Bool) => "__parts.next().unwrap().parse::<bool>().unwrap()".to_string(),
                Some(CppType::Named(ref name)) if name == "std::string" || name == "string" => {
                    "__parts.next().unwrap().to_string()".to_string()
                }
                _ => "__parts.next().unwrap().to_string()".to_string(),
            };

            format!("{} = {}", var_name, parse_expr)
        }).collect();

        // Generate the block that reads, splits, and parses
        format!(
            "{{ \
                let mut __line = String::new(); \
                std::io::stdin().read_line(&mut __line).unwrap(); \
                let mut __parts = __line.trim().split_whitespace(); \
                {}; \
                std::io::stdin() \
            }}",
            var_reads.join("; ")
        )
    }

    /// Generate a method or constructor.
    fn generate_method(&mut self, node: &ClangNode, struct_name: &str) {
        // Track current class for inherited member access
        let old_class = self.current_class.take();
        self.current_class = Some(struct_name.to_string());

        match &node.kind {
            ClangNodeKind::CXXMethodDecl { name, return_type, params, is_static, .. } => {
                // Check if method modifies self or returns a mutable reference
                let modifies_self = Self::method_modifies_self(node);
                let returns_mut_ref = matches!(return_type, CppType::Reference { is_const: false, .. });
                let is_mutable_method = modifies_self || returns_mut_ref;

                let self_param = if *is_static {
                    "".to_string()
                } else {
                    if is_mutable_method { "&mut self, ".to_string() } else { "&self, ".to_string() }
                };
                let params_str = params.iter()
                    .map(|(n, t)| format!("{}: {}", sanitize_identifier(n), t.to_rust_type_str()))
                    .collect::<Vec<_>>()
                    .join(", ");

                let ret_str = if *return_type == CppType::Void {
                    String::new()
                } else {
                    format!(" -> {}", return_type.to_rust_type_str())
                };

                // Special handling for operators that have const/non-const overloads
                // Skip the const version of operator* - only generate the mutable one
                // Note: operator-> always returns a pointer (not reference), so we don't skip it
                let skip_method = name == "operator*" && params.is_empty() && !is_mutable_method;

                if skip_method {
                    self.current_class = old_class;
                    return;
                }

                let method_name = if name == "operator*" && params.is_empty() {
                    // Unary dereference operator (mutable version only)
                    "op_deref".to_string()
                } else if name == "operator->" {
                    // Arrow operator (mutable version only)
                    "op_arrow".to_string()
                } else {
                    sanitize_identifier(name)
                };

                self.writeln(&format!("pub fn {}({}{}){} {{",
                    method_name, self_param, params_str, ret_str));
                self.indent += 1;

                // Track return type for reference return handling
                let old_return_type = self.current_return_type.take();
                self.current_return_type = Some(return_type.clone());

                // Find body
                for child in &node.children {
                    if let ClangNodeKind::CompoundStmt = &child.kind {
                        self.generate_block_contents(&child.children, return_type);
                    }
                }

                self.current_return_type = old_return_type;
                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }
            ClangNodeKind::ConstructorDecl { params, .. } => {
                // Always use new_N format (new_0, new_1, new_2) for consistency
                let fn_name = format!("new_{}", params.len());
                let internal_name = format!("__new_without_vbases_{}", params.len());

                let params_str = params.iter()
                    .map(|(n, t)| format!("{}: {}", sanitize_identifier(n), t.to_rust_type_str()))
                    .collect::<Vec<_>>()
                    .join(", ");
                let params_names = params.iter()
                    .map(|(n, _)| sanitize_identifier(n))
                    .collect::<Vec<_>>()
                    .join(", ");

                // Extract member initializers and base class initializers from constructor children
                // Pattern 1: MemberRef { name } followed by initialization expression (member initializer list)
                // Pattern 2: TypeRef:ClassName followed by CallExpr (base class initialization)
                // Pattern 3: CompoundStmt with assignments to member fields (body assignments)
                let mut initializers: Vec<(String, String)> = Vec::new();
                // base_inits: Vec<(field_name, constructor_call)> - supports multiple inheritance
                let mut base_inits: Vec<(String, String)> = Vec::new();
                let mut virtual_base_inits: Vec<(String, String)> = Vec::new();
                // Track constructor compound statement for non-member statements
                let mut ctor_compound_stmt: Option<usize> = None;

                // Get base classes for current class to determine field names
                let base_classes = self.current_class.as_ref()
                    .and_then(|c| self.class_bases.get(c))
                    .cloned()
                    .unwrap_or_default();

                let mut i = 0;
                while i < node.children.len() {
                    if let ClangNodeKind::MemberRef { name } = &node.children[i].kind {
                        // Next sibling should be the initializer expression
                        let init_val = if i + 1 < node.children.len() {
                            i += 1;
                            self.expr_to_string(&node.children[i])
                        } else {
                            "Default::default()".to_string()
                        };
                        initializers.push((name.clone(), init_val));
                    } else if let ClangNodeKind::Unknown(s) = &node.children[i].kind {
                        // Check for TypeRef:ClassName pattern indicating base class initializer
                        if let Some(base_class) = s.strip_prefix("TypeRef:") {
                            // Next sibling should be constructor call
                            if i + 1 < node.children.len() {
                                i += 1;
                                // Check if next is a CallExpr
                                if matches!(&node.children[i].kind, ClangNodeKind::CallExpr { .. }) {
                                    // Extract constructor arguments
                                    let args = self.extract_constructor_args(&node.children[i]);
                                    let ctor_call = format!("{}::new_{}({})", base_class, args.len(), args.join(", "));

                                    // Find the index of this base class to determine field name
                                    let mut non_virtual_idx = 0;
                                    let mut base_info: Option<BaseInfo> = None;
                                    for b in &base_classes {
                                        if b.name == base_class {
                                            base_info = Some(b.clone());
                                            break;
                                        }
                                        if !b.is_virtual {
                                            non_virtual_idx += 1;
                                        }
                                    }

                                    if let Some(info) = base_info {
                                        if info.is_virtual {
                                            virtual_base_inits.push((info.name, ctor_call));
                                        } else {
                                            let base_has_vbases = self.class_has_virtual_bases(&info.name);
                                            let ctor_name = if base_has_vbases {
                                                format!("{}::__new_without_vbases_{}", info.name, args.len())
                                            } else {
                                                format!("{}::new_{}", info.name, args.len())
                                            };
                                            let ctor_call = format!("{}({})", ctor_name, args.join(", "));
                                            let field_name = if non_virtual_idx == 0 { "__base".to_string() } else { format!("__base{}", non_virtual_idx) };
                                            base_inits.push((field_name, ctor_call));
                                        }
                                    } else {
                                        // Check if this is a transitive virtual base (not a direct base)
                                        let is_transitive_vbase = self.current_class.as_ref()
                                            .and_then(|c| self.virtual_bases.get(c))
                                            .map(|vbases| vbases.iter().any(|vb| vb == base_class))
                                            .unwrap_or(false);

                                        if is_transitive_vbase {
                                            // This is a virtual base initializer (e.g., A(v) in D::D() : A(v), B(v), C(v))
                                            virtual_base_inits.push((base_class.to_string(), ctor_call));
                                        } else {
                                            // Fallback to __base for direct non-virtual bases not found in class_bases
                                            base_inits.push(("__base".to_string(), ctor_call));
                                        }
                                    }
                                }
                            }
                        }
                    } else if let ClangNodeKind::CompoundStmt = &node.children[i].kind {
                        // Look for assignments in constructor body
                        Self::extract_member_assignments(&node.children[i], &mut initializers, self);
                        // Store compound stmt for later - non-member statements will be generated after Self {} literal
                        ctor_compound_stmt = Some(i);
                    }
                    i += 1;
                }

                let class_has_vbases = self.class_has_virtual_bases(struct_name);

                if class_has_vbases {
                    // Internal constructor that does not allocate virtual bases
                    self.writeln(&format!("pub(crate) fn {}({}) -> Self {{", internal_name, params_str));
                    self.indent += 1;
                    self.writeln("Self {");
                    self.indent += 1;

                    let mut initialized_vbase: std::collections::HashSet<String> = std::collections::HashSet::new();

                    for (field_name, base_call) in &base_inits {
                        self.writeln(&format!("{}: {},", field_name, base_call));
                        initialized_vbase.insert(field_name.clone());
                    }
                    let vbases_internal = self.virtual_bases.get(struct_name).cloned().unwrap_or_default();
                    for vb in &vbases_internal {
                        let field = self.virtual_base_field_name(vb);
                        let storage = self.virtual_base_storage_field_name(vb);
                        self.writeln(&format!("{}: std::ptr::null_mut(),", field));
                        self.writeln(&format!("{}: None,", storage));
                        initialized_vbase.insert(field);
                        initialized_vbase.insert(storage);
                    }
                    for (field, value) in &initializers {
                        let sanitized = sanitize_identifier(field);
                        self.writeln(&format!("{}: {},", sanitized, value));
                        initialized_vbase.insert(sanitized);
                    }

                    // Generate default values for uninitialized fields
                    let all_fields_vbase = self.class_fields.get(struct_name).cloned().unwrap_or_default();
                    for (field_name, field_type) in &all_fields_vbase {
                        if !initialized_vbase.contains(field_name) {
                            let default_val = default_value_for_type(field_type);
                            self.writeln(&format!("{}: {},", field_name, default_val));
                        }
                    }

                    self.indent -= 1;
                    self.writeln("}");
                    self.indent -= 1;
                    self.writeln("}");
                    self.writeln("");

                    // Public constructor that allocates virtual bases
                    self.writeln(&format!("pub fn {}({}) -> Self {{", fn_name, params_str));
                    self.indent += 1;
                    self.writeln(&format!("let mut __self = Self::{}({});", internal_name, params_names));

                    let vbases_public = self.virtual_bases.get(struct_name).cloned().unwrap_or_default();
                    for vb in &vbases_public {
                        let ctor = if let Some((_, call)) = virtual_base_inits.iter().find(|(name, _)| name == vb) {
                            call.clone()
                        } else {
                            format!("{}::new_0()", vb)
                        };
                        let vb_field = self.virtual_base_field_name(vb);
                        let vb_storage = self.virtual_base_storage_field_name(vb);
                        let temp_name = format!("__vb_{}", vb_field.trim_start_matches("__vbase_"));
                        self.writeln(&format!("let mut {} = Box::new({});", temp_name, ctor));
                        self.writeln(&format!("let {}_ptr = {}.as_mut() as *mut {};", temp_name, temp_name, vb));
                        self.writeln(&format!("__self.{} = {}_ptr;", vb_field, temp_name));
                        self.writeln(&format!("__self.{} = Some({});", vb_storage, temp_name));
                    }

                    // Propagate virtual base pointers into embedded bases that need them
                    let mut non_virtual_idx = 0;
                    for base in &base_classes {
                        if !base.is_virtual {
                            if self.class_has_virtual_bases(&base.name) {
                                let base_field = if non_virtual_idx == 0 { "__base".to_string() } else { format!("__base{}", non_virtual_idx) };
                                let base_vbases = self.virtual_bases.get(&base.name).cloned().unwrap_or_default();
                                for vb in &base_vbases {
                                    let vb_field = self.virtual_base_field_name(vb);
                                    self.writeln(&format!("__self.{}.{} = __self.{};", base_field, vb_field, vb_field));
                                }
                            }
                            non_virtual_idx += 1;
                        }
                    }

                    self.writeln("__self");
                    self.indent -= 1;
                    self.writeln("}");
                    self.writeln("");
                } else {
                    // Check if there are non-member statements that need to run after struct creation
                    let has_non_member_stmts = ctor_compound_stmt
                        .map(|idx| Self::has_non_member_ctor_stmts(&node.children[idx]))
                        .unwrap_or(false);

                    self.writeln(&format!("pub fn {}({}) -> Self {{", fn_name, params_str));
                    self.indent += 1;

                    if has_non_member_stmts {
                        // Need to run statements after construction, so use let + return pattern
                        self.writeln("let mut __self = Self {");
                    } else {
                        self.writeln("Self {");
                    }
                    self.indent += 1;

                    // Collect initialized field names
                    let mut initialized: std::collections::HashSet<String> = std::collections::HashSet::new();

                    // Generate base class initializers
                    for (field_name, base_call) in &base_inits {
                        self.writeln(&format!("{}: {},", field_name, base_call));
                        initialized.insert(field_name.clone());
                    }
                    // Generate field initializers
                    for (field, value) in &initializers {
                        let sanitized = sanitize_identifier(field);
                        self.writeln(&format!("{}: {},", sanitized, value));
                        initialized.insert(sanitized);
                    }

                    // Generate default values for uninitialized fields
                    // This avoids using ..Default::default() which can cause issues with Drop
                    let all_fields = self.class_fields.get(struct_name).cloned().unwrap_or_default();
                    for (field_name, field_type) in &all_fields {
                        if !initialized.contains(field_name) {
                            let default_val = default_value_for_type(field_type);
                            self.writeln(&format!("{}: {},", field_name, default_val));
                        }
                    }

                    self.indent -= 1;

                    if has_non_member_stmts {
                        self.writeln("};");
                        // Generate non-member statements with __self context
                        self.use_ctor_self = true;
                        if let Some(idx) = ctor_compound_stmt {
                            self.generate_non_member_ctor_stmts(&node.children[idx]);
                        }
                        self.use_ctor_self = false;
                        self.writeln("__self");
                    } else {
                        self.writeln("}");
                    }
                    self.indent -= 1;
                    self.writeln("}");
                    self.writeln("");
                }
            }
            _ => {}
        }

        // Restore previous class context
        self.current_class = old_class;
    }

    /// Generate the contents of a block (compound statement).
    fn generate_block_contents(&mut self, stmts: &[ClangNode], return_type: &CppType) {
        let len = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == len - 1;
            self.generate_stmt(stmt, is_last && *return_type != CppType::Void);
        }
    }

    /// Generate a statement.
    fn generate_stmt(&mut self, node: &ClangNode, is_tail_expr: bool) {
        match &node.kind {
            ClangNodeKind::DeclStmt => {
                // Variable declaration
                for child in &node.children {
                    if let ClangNodeKind::VarDecl { name, ty, .. } = &child.kind {
                        // Check if this is a reference, array, or pointer type
                        let is_ref = matches!(ty, CppType::Reference { .. });
                        let is_const_ref = matches!(ty, CppType::Reference { is_const: true, .. });
                        let is_array = matches!(ty, CppType::Array { .. });
                        let is_ptr = matches!(ty, CppType::Pointer { .. });

                        // Track typed variables for later
                        if is_ref {
                            self.ref_vars.insert(name.clone());
                        }
                        if is_array {
                            self.arr_vars.insert(name.clone());
                        }
                        if is_ptr {
                            self.ptr_vars.insert(name.clone());
                        }

                        // Find the actual initializer, skipping reference nodes and type nodes
                        // ParmVarDecl nodes appear in function pointer VarDecls to describe parameter types
                        let initializer = child.children.iter().find(|c| {
                            !matches!(&c.kind, ClangNodeKind::Unknown(s) if s == "TypeRef")
                                && !matches!(&c.kind, ClangNodeKind::Unknown(s) if s.contains("Type"))
                                && !matches!(&c.kind, ClangNodeKind::Unknown(s) if s == "NamespaceRef")
                                && !matches!(&c.kind, ClangNodeKind::Unknown(s) if s == "TemplateRef")
                                && !matches!(&c.kind, ClangNodeKind::ParmVarDecl { .. })
                        });

                        // Check if we have a real initializer
                        let has_real_init = if let Some(init_node) = &initializer {
                            // For arrays with just an integer literal child, it might be the array size
                            if is_array {
                                !matches!(&init_node.kind, ClangNodeKind::IntegerLiteral { .. })
                            } else {
                                true
                            }
                        } else {
                            false
                        };

                        let init = if has_real_init {
                            let init_node = initializer.unwrap();
                            // Special case: function pointer initialized with nullptr → None
                            if Self::is_function_pointer_type(ty) && Self::is_nullptr_literal(init_node) {
                                " = None".to_string()
                            } else {
                                // Skip type suffixes for literals when we have explicit type annotation
                                self.skip_literal_suffix = true;
                                let expr = self.expr_to_string(init_node);
                                self.skip_literal_suffix = false;
                                // If expression is unsupported, fall back to default
                                if expr.contains("unsupported") {
                                    format!(" = {}", default_value_for_type(ty))
                                } else if is_ref {
                                    // Reference initialization: add &mut or & prefix
                                    let prefix = if is_const_ref { "&" } else { "&mut " };
                                    format!(" = {}{}", prefix, expr)
                                } else if let Some(variant_args) = Self::get_variant_args(ty) {
                                    // std::variant initialization: wrap in enum variant constructor
                                    let enum_name = Self::get_variant_enum_name(ty).unwrap();
                                    // Find the actual value being passed to the variant constructor
                                    // (navigate through Unknown/CallExpr wrappers)
                                    let value_node = Self::find_variant_init_value(init_node).unwrap_or(init_node);
                                    let value_expr = self.expr_to_string(value_node);
                                    // Try to determine the initializer type
                                    if let Some(init_type) = Self::get_expr_type(value_node) {
                                        if let Some(idx) = Self::find_variant_index(&variant_args, &init_type) {
                                            format!(" = {}::V{}({})", enum_name, idx, value_expr)
                                        } else {
                                            // Couldn't match type to variant, use V0 as fallback
                                            format!(" = {}::V0({})", enum_name, value_expr)
                                        }
                                    } else {
                                        // Couldn't determine init type, use V0 as fallback
                                        format!(" = {}::V0({})", enum_name, value_expr)
                                    }
                                } else {
                                    format!(" = {}", expr)
                                }
                            }
                        } else {
                            // Default value for function pointers is None
                            if Self::is_function_pointer_type(ty) {
                                " = None".to_string()
                            } else {
                                format!(" = {}", default_value_for_type(ty))
                            }
                        };

                        // References don't need mut keyword
                        let mut_kw = if is_ref { "" } else { "mut " };
                        self.writeln(&format!("let {}{}: {}{};",
                            mut_kw, sanitize_identifier(name), ty.to_rust_type_str(), init));
                    }
                }
            }
            ClangNodeKind::ReturnStmt => {
                if node.children.is_empty() {
                    self.writeln("return;");
                } else {
                    let expr = self.expr_to_string(&node.children[0]);
                    // Check if we need to add &mut for reference return types
                    let expr = if let Some(CppType::Reference { is_const, .. }) = &self.current_return_type {
                        // Don't add & or &mut if returning 'self' (from *this in C++)
                        // because Rust's &mut self already provides the reference
                        if expr == "self" || expr == "__self" {
                            expr
                        } else if expr.starts_with("unsafe { ") && expr.ends_with(" }") {
                            // If expression is an unsafe block like "unsafe { *ptr }",
                            // put the & or &mut inside: "unsafe { &mut *ptr }"
                            let inner = &expr[9..expr.len()-2]; // Extract content between "unsafe { " and " }"
                            let prefix = if *is_const { "&" } else { "&mut " };
                            format!("unsafe {{ {}{} }}", prefix, inner)
                        } else if *is_const {
                            format!("&{}", expr)
                        } else {
                            format!("&mut {}", expr)
                        }
                    } else {
                        expr
                    };
                    self.writeln(&format!("return {};", expr));
                }
            }
            ClangNodeKind::IfStmt => {
                self.generate_if_stmt(node);
            }
            ClangNodeKind::WhileStmt => {
                self.generate_while_stmt(node);
            }
            ClangNodeKind::ForStmt => {
                self.generate_for_stmt(node);
            }
            ClangNodeKind::CXXForRangeStmt { var_name, var_type } => {
                self.generate_range_for_stmt(node, var_name, var_type);
            }
            ClangNodeKind::DoStmt => {
                self.generate_do_stmt(node);
            }
            ClangNodeKind::SwitchStmt => {
                self.generate_switch_stmt(node);
            }
            ClangNodeKind::CompoundStmt => {
                self.writeln("{");
                self.indent += 1;
                self.generate_block_contents(&node.children, &CppType::Void);
                self.indent -= 1;
                self.writeln("}");
            }
            ClangNodeKind::ExprStmt => {
                if !node.children.is_empty() {
                    let expr = self.expr_to_string(&node.children[0]);
                    if is_tail_expr {
                        self.writeln(&expr);
                    } else {
                        self.writeln(&format!("{};", expr));
                    }
                }
            }
            ClangNodeKind::BreakStmt => {
                self.writeln("break;");
            }
            ClangNodeKind::ContinueStmt => {
                self.writeln("continue;");
            }
            ClangNodeKind::TryStmt => {
                // try { ... } catch { ... } => match std::panic::catch_unwind(|| { ... })
                // Find the try body (first CompoundStmt) and catch handlers
                let mut try_body = None;
                let mut catch_handlers = Vec::new();

                for child in &node.children {
                    match &child.kind {
                        ClangNodeKind::CompoundStmt => {
                            if try_body.is_none() {
                                try_body = Some(child);
                            }
                        }
                        ClangNodeKind::CatchStmt { .. } => {
                            catch_handlers.push(child);
                        }
                        _ => {}
                    }
                }

                if let Some(body) = try_body {
                    // Generate: match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { ... }))
                    self.writeln("match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {");
                    self.indent += 1;
                    self.generate_block_contents(&body.children, &CppType::Void);
                    self.indent -= 1;
                    self.writeln("})) {");
                    self.indent += 1;
                    self.writeln("Ok(result) => result,");
                    self.writeln("Err(_e) => {");
                    self.indent += 1;

                    // Generate catch handler body (use first catch handler if any)
                    if let Some(catch) = catch_handlers.first() {
                        for catch_child in &catch.children {
                            if let ClangNodeKind::CompoundStmt = &catch_child.kind {
                                self.generate_block_contents(&catch_child.children, &CppType::Void);
                            }
                        }
                    } else {
                        self.writeln("// No catch handler");
                    }

                    self.indent -= 1;
                    self.writeln("}");
                    self.indent -= 1;
                    self.writeln("}");
                }
            }
            ClangNodeKind::CatchStmt { .. } => {
                // Handled as part of TryStmt
            }
            _ => {
                // For expressions at statement level
                let expr = self.expr_to_string(node);
                if is_tail_expr {
                    self.writeln(&expr);
                } else if !expr.is_empty() {
                    self.writeln(&format!("{};", expr));
                }
            }
        }
    }

    /// Generate an if statement.
    fn generate_if_stmt(&mut self, node: &ClangNode) {
        // Children: condition, then-branch, [else-branch]
        if node.children.len() >= 2 {
            let cond = self.expr_to_string(&node.children[0]);
            // In C++, pointers can be used in boolean context (non-null = true)
            // In Rust, we need to explicitly check .is_null()
            let cond_type = Self::get_expr_type(&node.children[0]);
            let cond = if matches!(cond_type, Some(CppType::Pointer { .. })) {
                format!("!{}.is_null()", cond)
            } else {
                cond
            };
            self.writeln(&format!("if {} {{", cond));
            self.indent += 1;
            self.generate_stmt(&node.children[1], false);
            self.indent -= 1;

            if node.children.len() > 2 {
                // Check if else is another if (else if)
                if let ClangNodeKind::IfStmt = &node.children[2].kind {
                    self.write("} else ");
                    self.generate_if_stmt(&node.children[2]);
                    return;
                }
                self.writeln("} else {");
                self.indent += 1;
                self.generate_stmt(&node.children[2], false);
                self.indent -= 1;
            }
            self.writeln("}");
        }
    }

    /// Generate a while statement.
    fn generate_while_stmt(&mut self, node: &ClangNode) {
        // Children: condition, body
        if node.children.len() >= 2 {
            let cond = self.expr_to_string(&node.children[0]);
            // In C++, pointers can be used in boolean context (non-null = true)
            let cond_type = Self::get_expr_type(&node.children[0]);
            let cond = if matches!(cond_type, Some(CppType::Pointer { .. })) {
                format!("!{}.is_null()", cond)
            } else {
                cond
            };
            self.writeln(&format!("while {} {{", cond));
            self.indent += 1;
            self.generate_stmt(&node.children[1], false);
            self.indent -= 1;
            self.writeln("}");
        }
    }

    /// Generate a do-while statement.
    fn generate_do_stmt(&mut self, node: &ClangNode) {
        // Children: body, condition
        // do { body } while (cond); => loop { body; if !cond { break; } }
        if node.children.len() >= 2 {
            self.writeln("loop {");
            self.indent += 1;
            // Body first (executes at least once)
            self.generate_stmt(&node.children[0], false);
            // Then condition check
            let cond = self.expr_to_string(&node.children[1]);
            self.writeln(&format!("if !({}) {{ break; }}", cond));
            self.indent -= 1;
            self.writeln("}");
        }
    }

    /// Generate a switch statement as Rust match.
    fn generate_switch_stmt(&mut self, node: &ClangNode) {
        // Switch structure: condition expr, then CompoundStmt with CaseStmt/DefaultStmt
        if node.children.len() < 2 {
            return;
        }

        let cond = self.expr_to_string(&node.children[0]);
        self.writeln(&format!("match {} {{", cond));
        self.indent += 1;

        // Find the body (CompoundStmt with cases)
        let body = &node.children[1];
        if let ClangNodeKind::CompoundStmt = &body.kind {
            // Process each case/default in the body
            let mut current_values: Vec<i128> = Vec::new();
            let mut case_body: Vec<&ClangNode> = Vec::new();

            for child in &body.children {
                match &child.kind {
                    ClangNodeKind::CaseStmt { value } => {
                        // If we have accumulated body statements, emit the previous case
                        if !case_body.is_empty() && !current_values.is_empty() {
                            self.emit_match_arm(&current_values, &case_body);
                            current_values.clear();
                            case_body.clear();
                        }

                        current_values.push(*value);

                        // Case children: the value literal, then the body statements
                        // Body can be inside the CaseStmt as children after the literal
                        for (i, case_child) in child.children.iter().enumerate() {
                            if i == 0 && matches!(&case_child.kind, ClangNodeKind::IntegerLiteral { .. }) {
                                continue; // Skip the case value literal
                            }
                            // Check for nested CaseStmt (fallthrough)
                            if let ClangNodeKind::CaseStmt { value: nested_val } = &case_child.kind {
                                current_values.push(*nested_val);
                                // Process nested case's children
                                for (j, nested_child) in case_child.children.iter().enumerate() {
                                    if j == 0 && matches!(&nested_child.kind, ClangNodeKind::IntegerLiteral { .. }) {
                                        continue;
                                    }
                                    case_body.push(nested_child);
                                }
                            } else {
                                case_body.push(case_child);
                            }
                        }
                    }
                    ClangNodeKind::DefaultStmt => {
                        // Emit previous case if any
                        if !current_values.is_empty() {
                            self.emit_match_arm(&current_values, &case_body);
                            current_values.clear();
                            case_body.clear();
                        }

                        // Collect default body
                        let default_body: Vec<&ClangNode> = child.children.iter().collect();
                        self.emit_default_arm(&default_body);
                    }
                    _ => {}
                }
            }

            // Emit final case if any
            if !current_values.is_empty() {
                self.emit_match_arm(&current_values, &case_body);
            }
        }

        // Add default arm if not present (Rust requires exhaustive match)
        // Note: We add _ => {} only if no DefaultStmt was found
        let has_default = node.children.get(1).map_or(false, |c| {
            if let ClangNodeKind::CompoundStmt = &c.kind {
                c.children.iter().any(|ch| matches!(&ch.kind, ClangNodeKind::DefaultStmt))
            } else {
                false
            }
        });
        if !has_default {
            self.writeln("_ => {}");
        }

        self.indent -= 1;
        self.writeln("}");
    }

    /// Emit a match arm for one or more case values.
    fn emit_match_arm(&mut self, values: &[i128], body: &[&ClangNode]) {
        let pattern = values.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" | ");

        self.writeln(&format!("{} => {{", pattern));
        self.indent += 1;
        for stmt in body {
            self.generate_stmt(stmt, false);
        }
        self.indent -= 1;
        self.writeln("}");
    }

    /// Emit the default arm of a match.
    fn emit_default_arm(&mut self, body: &[&ClangNode]) {
        self.writeln("_ => {");
        self.indent += 1;
        for stmt in body {
            self.generate_stmt(stmt, false);
        }
        self.indent -= 1;
        self.writeln("}");
    }

    /// Generate a for statement.
    fn generate_for_stmt(&mut self, node: &ClangNode) {
        // C++ for loops: for (init; cond; inc) { body }
        // Convert to: { init; loop { if !cond { break; } body; inc; } }
        // This correctly handles continue (which should go to inc, then cond)
        // Children: [init], [cond], [inc], body

        self.writeln("{");
        self.indent += 1;

        if node.children.len() >= 4 {
            // Init
            self.generate_stmt(&node.children[0], false);

            // Get condition and increment
            let cond = if matches!(&node.children[1].kind, ClangNodeKind::IntegerLiteral { .. }) {
                "true".to_string()
            } else {
                self.expr_to_string(&node.children[1])
            };

            let inc = self.expr_to_string(&node.children[2]);

            // Use loop with break for condition to handle continue correctly
            self.writeln("loop {");
            self.indent += 1;

            // Condition check with break
            self.writeln(&format!("if !({}) {{ break; }}", cond));

            // Body - we need to handle continue specially
            // Generate body with continue handling
            self.generate_for_body(&node.children[3], &inc);

            // Increment at end (only reached if no continue/break)
            if !inc.is_empty() {
                self.writeln(&format!("{};", inc));
            }

            self.indent -= 1;
            self.writeln("}");
        }

        self.indent -= 1;
        self.writeln("}");
    }

    /// Generate a range-based for statement.
    /// C++: for (T x : container) { body }
    /// Rust: for x in container.iter() { body } or for x in &container { body }
    fn generate_range_for_stmt(&mut self, node: &ClangNode, var_name: &str, var_type: &CppType) {
        // Children of CXXForRangeStmt:
        // - Various internal VarDecls (__range1, __begin1, __end1, etc.)
        // - The loop variable VarDecl
        // - DeclRefExpr for the range (container)
        // - CompoundStmt (body)

        // Find the range expression and body
        let mut range_expr = None;
        let mut body = None;

        for child in &node.children {
            match &child.kind {
                ClangNodeKind::DeclRefExpr { name, ty, .. } => {
                    // Skip internal variables, use the actual container
                    if !name.starts_with("__") {
                        range_expr = Some((name.clone(), ty.clone()));
                    }
                }
                ClangNodeKind::CompoundStmt => {
                    body = Some(child);
                }
                _ => {}
            }
        }

        // Generate: for var_name in range_expr { body }
        if let Some((range_name, range_type)) = range_expr {
            // Determine iterator method based on type
            let iter_suffix = if matches!(range_type, CppType::Array { .. }) {
                ".iter()"
            } else {
                "" // References work directly in Rust for loop
            };

            // Note: Rust for loops don't support type annotations, so we omit var_type
            let _ = var_type; // Silence unused warning
            self.writeln(&format!("for {} in {}{} {{",
                sanitize_identifier(var_name),
                sanitize_identifier(&range_name),
                iter_suffix));
            self.indent += 1;

            // Generate body
            if let Some(body_node) = body {
                self.generate_block_contents(&body_node.children, &CppType::Void);
            }

            self.indent -= 1;
            self.writeln("}");
        } else {
            // Fallback: try to find range in children of VarDecl
            self.writeln("/* range-based for: could not extract range */");
        }
    }

    /// Generate for loop body with special continue handling.
    /// Continue needs to run the increment before looping back.
    fn generate_for_body(&mut self, node: &ClangNode, inc: &str) {
        match &node.kind {
            ClangNodeKind::CompoundStmt => {
                self.writeln("{");
                self.indent += 1;
                for stmt in &node.children {
                    self.generate_for_body_stmt(stmt, inc);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            ClangNodeKind::ContinueStmt => {
                // For continue in for loop: increment then continue
                if !inc.is_empty() {
                    self.writeln(&format!("{}; continue;", inc));
                } else {
                    self.writeln("continue;");
                }
            }
            _ => {
                self.generate_for_body_stmt(node, inc);
            }
        }
    }

    /// Generate a statement inside a for loop body, handling continue specially.
    fn generate_for_body_stmt(&mut self, node: &ClangNode, inc: &str) {
        match &node.kind {
            ClangNodeKind::ContinueStmt => {
                // For continue in for loop: increment then continue
                if !inc.is_empty() {
                    self.writeln(&format!("{}; continue;", inc));
                } else {
                    self.writeln("continue;");
                }
            }
            ClangNodeKind::CompoundStmt => {
                self.writeln("{");
                self.indent += 1;
                for stmt in &node.children {
                    self.generate_for_body_stmt(stmt, inc);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            ClangNodeKind::IfStmt => {
                // Need special handling for if statements containing continue
                self.generate_for_if_stmt(node, inc);
            }
            _ => {
                self.generate_stmt(node, false);
            }
        }
    }

    /// Generate if statement inside for loop body, handling continue in branches.
    fn generate_for_if_stmt(&mut self, node: &ClangNode, inc: &str) {
        if node.children.len() >= 2 {
            let cond = self.expr_to_string(&node.children[0]);
            self.writeln(&format!("if {} {{", cond));
            self.indent += 1;
            self.generate_for_body_stmt(&node.children[1], inc);
            self.indent -= 1;

            if node.children.len() > 2 {
                if let ClangNodeKind::IfStmt = &node.children[2].kind {
                    self.write("} else ");
                    self.generate_for_if_stmt(&node.children[2], inc);
                    return;
                }
                self.writeln("} else {");
                self.indent += 1;
                self.generate_for_body_stmt(&node.children[2], inc);
                self.indent -= 1;
            }
            self.writeln("}");
        }
    }

    /// Convert an expression node to a Rust string (without unsafe wrapping for derefs).
    /// Used inside unsafe blocks where we don't want nested unsafe.
    fn expr_to_string_raw(&self, node: &ClangNode) -> String {
        match &node.kind {
            ClangNodeKind::UnaryOperator { op, ty } => {
                if !node.children.is_empty() {
                    let operand = self.expr_to_string_raw(&node.children[0]);
                    match op {
                        UnaryOp::Deref => format!("*{}", operand),
                        UnaryOp::Minus => format!("-{}", operand),
                        UnaryOp::Plus => operand,
                        UnaryOp::LNot => format!("!{}", operand),
                        UnaryOp::Not => format!("!{}", operand),
                        UnaryOp::AddrOf => {
                            // Check if this is a pointer to a polymorphic class
                            if let CppType::Pointer { pointee, is_const } = ty {
                                if let CppType::Named(class_name) = pointee.as_ref() {
                                    if self.polymorphic_classes.contains(class_name) {
                                        // For polymorphic types, just return reference
                                        return if *is_const {
                                            format!("&{}", operand)
                                        } else {
                                            format!("&mut {}", operand)
                                        };
                                    }
                                }
                            }
                            let rust_ty = ty.to_rust_type_str();
                            if rust_ty.starts_with("*mut ") {
                                format!("&mut {} as {}", operand, rust_ty)
                            } else if rust_ty.starts_with("*const ") {
                                format!("&{} as {}", operand, rust_ty)
                            } else {
                                format!("&{}", operand)
                            }
                        }
                        UnaryOp::PreInc => format!("{{ {} += 1; {} }}", operand, operand),
                        UnaryOp::PreDec => format!("{{ {} -= 1; {} }}", operand, operand),
                        UnaryOp::PostInc => format!("{{ let __v = {}; {} += 1; __v }}", operand, operand),
                        UnaryOp::PostDec => format!("{{ let __v = {}; {} -= 1; __v }}", operand, operand),
                    }
                } else {
                    "/* unary op error */".to_string()
                }
            }
            ClangNodeKind::ImplicitCastExpr { cast_kind, ty } => {
                // Handle implicit casts - some need explicit conversion in Rust
                if !node.children.is_empty() {
                    let inner = self.expr_to_string_raw(&node.children[0]);
                    match cast_kind {
                        CastKind::IntegralCast => {
                            // Need explicit cast for integral conversions
                            let rust_type = ty.to_rust_type_str();
                            format!("{} as {}", inner, rust_type)
                        }
                        CastKind::FloatingCast | CastKind::IntegralToFloating | CastKind::FloatingToIntegral => {
                            // Need explicit cast for floating conversions
                            let rust_type = ty.to_rust_type_str();
                            format!("{} as {}", inner, rust_type)
                        }
                        CastKind::FunctionToPointerDecay => {
                            // Function to pointer decay - wrap in Some() for Option<fn(...)> type
                            format!("Some({})", inner)
                        }
                        _ => {
                            // Most casts pass through (LValueToRValue, ArrayToPointerDecay, etc.)
                            inner
                        }
                    }
                } else {
                    "/* cast error */".to_string()
                }
            }
            ClangNodeKind::DeclRefExpr { name, namespace_path, ty, .. } => {
                if name == "this" {
                    "self".to_string()
                } else {
                    // Check for standard I/O streams (std::cout, std::cerr, std::cin)
                    // These should be mapped to Rust's std::io functions
                    let is_std_namespace = namespace_path.len() == 1 && namespace_path[0] == "std";
                    if is_std_namespace || namespace_path.is_empty() {
                        match name.as_str() {
                            "cout" => return "std::io::stdout()".to_string(),
                            "cerr" | "clog" => return "std::io::stderr()".to_string(),
                            "cin" => return "std::io::stdin()".to_string(),
                            _ => {}
                        }
                    }

                    let ident = sanitize_identifier(name);
                    // For static member access (class name in namespace path, non-function type),
                    // convert to global variable name (no unsafe wrapper since we're already in unsafe)
                    if !namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                        let class_name = &namespace_path[namespace_path.len() - 1];
                        // Try to find the global name from static_members
                        if let Some(global_name) = self.static_members.get(&(class_name.clone(), name.clone())) {
                            return global_name.clone();
                        }
                        // Fallback: generate from convention
                        let global_name = format!("{}_{}", class_name.to_uppercase(), ident.to_uppercase());
                        let is_static_member = self.static_members.values().any(|g| g == &global_name);
                        if is_static_member {
                            return global_name;
                        }
                    }
                    // Check if this is a static member of the current class (accessed without Class:: prefix)
                    if namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                        if let Some(ref current_class) = self.current_class {
                            if let Some(global_name) = self.static_members.get(&(current_class.clone(), name.clone())) {
                                return global_name.clone();
                            }
                        }
                    }

                    // Check if this is a global variable (already in unsafe context, no wrapper needed)
                    if self.global_vars.contains(name) {
                        return ident;
                    }

                    ident
                }
            }
            ClangNodeKind::IntegerLiteral { value, cpp_type } => {
                let suffix = match cpp_type {
                    Some(CppType::Char { signed: true }) => "i8",
                    Some(CppType::Char { signed: false }) => "u8",
                    Some(CppType::Short { signed: true }) => "i16",
                    Some(CppType::Short { signed: false }) => "u16",
                    Some(CppType::Int { signed: true }) => "i32",
                    Some(CppType::Int { signed: false }) => "u32",
                    Some(CppType::Long { signed: true }) => "i64",
                    Some(CppType::Long { signed: false }) => "u64",
                    _ => "i32",
                };
                format!("{}{}", value, suffix)
            }
            ClangNodeKind::EvaluatedExpr { int_value, float_value, ty } => {
                // Evaluated constant expression (e.g., default argument)
                if let Some(val) = int_value {
                    let suffix = match ty {
                        CppType::Int { signed: true } => "i32",
                        CppType::Int { signed: false } => "u32",
                        CppType::Long { signed: true } => "i64",
                        CppType::Long { signed: false } => "u64",
                        _ => "i32",
                    };
                    format!("{}{}", val, suffix)
                } else if let Some(val) = float_value {
                    let suffix = match ty {
                        CppType::Float => "f32",
                        CppType::Double => "f64",
                        _ => "f64",
                    };
                    format!("{}{}", val, suffix)
                } else {
                    "0".to_string()
                }
            }
            ClangNodeKind::ArraySubscriptExpr { .. } => {
                // For array subscript in raw context (inside unsafe block),
                // generate pointer arithmetic without wrapping in unsafe
                if node.children.len() >= 2 {
                    let arr = self.expr_to_string_raw(&node.children[0]);
                    let idx = self.expr_to_string_raw(&node.children[1]);
                    // Check if the array expression is a pointer type
                    let arr_type = Self::get_expr_type(&node.children[0]);
                    let is_pointer = matches!(arr_type, Some(CppType::Pointer { .. }))
                        || matches!(arr_type, Some(CppType::Array { size: None, .. }))
                        || self.is_ptr_var_expr(&node.children[0]);
                    if is_pointer {
                        // Raw pointer indexing without unsafe wrapper
                        format!("*{}.add({} as usize)", arr, idx)
                    } else {
                        // Array indexing
                        format!("{}[{} as usize]", arr, idx)
                    }
                } else {
                    "/* array subscript error */".to_string()
                }
            }
            ClangNodeKind::MemberExpr { member_name, is_static, is_arrow, declaring_class, .. } => {
                // For static member access, return the global name without unsafe wrapper
                if *is_static {
                    if let Some(class_name) = declaring_class {
                        if let Some(global_name) = self.static_members.get(&(class_name.clone(), member_name.clone())) {
                            return global_name.clone();
                        }
                        // Fallback: generate from convention
                        return format!("{}_{}", class_name.to_uppercase(), sanitize_identifier(member_name).to_uppercase());
                    }
                }
                // Non-static members: generate raw without unsafe wrapper
                if !node.children.is_empty() {
                    let base = self.expr_to_string_raw(&node.children[0]);
                    let member = sanitize_identifier(member_name);
                    if *is_arrow {
                        // Arrow access without unsafe wrapper (caller handles unsafe)
                        format!("(*{}).{}", base, member)
                    } else {
                        format!("{}.{}", base, member)
                    }
                } else {
                    sanitize_identifier(member_name)
                }
            }
            ClangNodeKind::BinaryOperator { op, .. } => {
                // Inside unsafe block, don't wrap sub-expressions in additional unsafe
                if node.children.len() >= 2 {
                    // Handle comma operator specially: (a, b) => { a; b }
                    if matches!(op, BinaryOp::Comma) {
                        let left = self.expr_to_string_raw(&node.children[0]);
                        let right = self.expr_to_string_raw(&node.children[1]);
                        return format!("{{ {}; {} }}", left, right);
                    }
                    let op_str = binop_to_string(op);
                    let left = self.expr_to_string_raw(&node.children[0]);
                    let right = self.expr_to_string_raw(&node.children[1]);
                    format!("{} {} {}", left, op_str, right)
                } else {
                    "/* binary op error */".to_string()
                }
            }
            ClangNodeKind::Unknown(_) => {
                // For unknown wrapper nodes (like UnexposedExpr for implicit casts),
                // recursively use raw conversion to avoid nested unsafe
                if !node.children.is_empty() {
                    self.expr_to_string_raw(&node.children[0])
                } else {
                    "/* unknown raw */".to_string()
                }
            }
            // For other expressions, use the regular conversion
            _ => self.expr_to_string(node),
        }
    }

    /// Convert an expression node to a Rust string.
    fn expr_to_string(&self, node: &ClangNode) -> String {
        match &node.kind {
            ClangNodeKind::IntegerLiteral { value, cpp_type } => {
                if self.skip_literal_suffix {
                    value.to_string()
                } else {
                    let suffix = match cpp_type {
                        Some(CppType::Int { signed: true }) => "i32",
                        Some(CppType::Int { signed: false }) => "u32",
                        Some(CppType::Long { signed: true }) => "i64",
                        Some(CppType::Long { signed: false }) => "u64",
                        Some(CppType::LongLong { signed: true }) => "i64",
                        Some(CppType::LongLong { signed: false }) => "u64",
                        Some(CppType::Short { signed: true }) => "i16",
                        Some(CppType::Short { signed: false }) => "u16",
                        Some(CppType::Char { signed: true }) => "i8",
                        Some(CppType::Char { signed: false }) => "u8",
                        _ => "i32",
                    };
                    format!("{}{}", value, suffix)
                }
            }
            ClangNodeKind::FloatingLiteral { value, cpp_type } => {
                if self.skip_literal_suffix {
                    // For floats, we need to ensure there's a decimal point
                    let s = value.to_string();
                    if s.contains('.') || s.contains('e') || s.contains('E') {
                        s
                    } else {
                        format!("{}.0", s)
                    }
                } else {
                    let suffix = match cpp_type {
                        Some(CppType::Float) => "f32",
                        _ => "f64",
                    };
                    format!("{}{}", value, suffix)
                }
            }
            ClangNodeKind::EvaluatedExpr { int_value, float_value, ty } => {
                // Evaluated constant expression (e.g., default argument)
                if let Some(val) = int_value {
                    if self.skip_literal_suffix {
                        val.to_string()
                    } else {
                        let suffix = match ty {
                            CppType::Int { signed: true } => "i32",
                            CppType::Int { signed: false } => "u32",
                            CppType::Long { signed: true } => "i64",
                            CppType::Long { signed: false } => "u64",
                            _ => "i32",
                        };
                        format!("{}{}", val, suffix)
                    }
                } else if let Some(val) = float_value {
                    if self.skip_literal_suffix {
                        let s = val.to_string();
                        if s.contains('.') || s.contains('e') || s.contains('E') {
                            s
                        } else {
                            format!("{}.0", s)
                        }
                    } else {
                        let suffix = match ty {
                            CppType::Float => "f32",
                            _ => "f64",
                        };
                        format!("{}{}", val, suffix)
                    }
                } else {
                    "0".to_string()
                }
            }
            ClangNodeKind::BoolLiteral(b) => b.to_string(),
            ClangNodeKind::NullPtrLiteral => "std::ptr::null_mut()".to_string(),
            ClangNodeKind::CXXNewExpr { ty, is_array, is_placement } => {
                if *is_placement && *is_array {
                    // Array placement new: new (ptr) T[n] → construct n elements at ptr
                    // Children typically: [placement_ptr, size_expr, CXXConstructExpr or InitListExpr]
                    let element_type = ty.pointee().unwrap_or(ty);
                    let type_str = element_type.to_rust_type_str();
                    let default_val = default_value_for_type(element_type);

                    // Extract placement pointer (first child)
                    let ptr_str = if !node.children.is_empty() {
                        let ptr_node = &node.children[0];
                        let ptr_type = Self::get_expr_type(ptr_node);
                        let ptr_expr = self.expr_to_string(ptr_node);
                        if matches!(ptr_type, Some(CppType::Array { .. })) {
                            format!("{}.as_mut_ptr()", ptr_expr)
                        } else {
                            ptr_expr
                        }
                    } else {
                        "/* missing placement ptr */".to_string()
                    };

                    // Extract size expression (typically second child)
                    let size_str = if node.children.len() >= 2 {
                        self.expr_to_string(&node.children[1])
                    } else {
                        "0".to_string()
                    };

                    // Generate array placement new: write each element at ptr + offset
                    format!(
                        "{{ let __ptr = {} as *mut {}; let __n = {} as usize; debug_assert!((__ptr as usize) % std::mem::align_of::<{}>() == 0, \"array placement new: pointer not aligned for {}\"); unsafe {{ for __i in 0..__n {{ std::ptr::write(__ptr.add(__i), {}) }} }}; __ptr }}",
                        ptr_str, type_str, size_str, type_str, type_str, default_val
                    )
                } else if *is_placement {
                    // Single-object placement new: new (ptr) T(args) → std::ptr::write(ptr, T::new(args))
                    // AST children order: [CXXConstructExpr, ImplicitCastExpr(placement_arg)]
                    // The placement argument (ptr) is the last child
                    // The constructor/initializer is in the first child
                    let type_str = ty.pointee().unwrap_or(ty).to_rust_type_str();

                    // Find placement argument and constructor
                    // In libclang traversal, the order appears to be: [placement_ptr, CXXConstructExpr]
                    // (opposite of the AST dump display order)
                    let (ptr_str, init_str) = if node.children.len() >= 2 {
                        // First child is the placement pointer (where to write)
                        // Check if it's an array and needs .as_mut_ptr() conversion
                        let ptr_node = &node.children[0];
                        let ptr_type = Self::get_expr_type(ptr_node);
                        let ptr_expr = self.expr_to_string(ptr_node);
                        let ptr = if matches!(ptr_type, Some(CppType::Array { .. })) {
                            // Array needs explicit pointer conversion
                            format!("{}.as_mut_ptr()", ptr_expr)
                        } else {
                            ptr_expr
                        };
                        // Last child is the constructor expression (the value to write)
                        let init = self.expr_to_string(&node.children[node.children.len() - 1]);
                        (ptr, init)
                    } else if node.children.len() == 1 {
                        let init = self.expr_to_string(&node.children[0]);
                        ("/* missing placement ptr */".to_string(), init)
                    } else {
                        ("/* missing placement ptr */".to_string(), default_value_for_type(ty))
                    };

                    // Generate: cast ptr to target type, verify alignment, write constructor value, return ptr
                    // The debug_assert checks alignment requirements at runtime in debug builds
                    format!(
                        "{{ let __ptr = {} as *mut {}; debug_assert!((__ptr as usize) % std::mem::align_of::<{}>() == 0, \"placement new: pointer not aligned for {}\"); unsafe {{ std::ptr::write(__ptr, {}) }}; __ptr }}",
                        ptr_str, type_str, type_str, type_str, init_str
                    )
                } else if *is_array {
                    // new T[n] → allocate n elements and return raw pointer
                    // ty is the result type (T*), we need the element type (T)
                    let element_type = ty.pointee().unwrap_or(ty);
                    // Children[0] should be the size expression
                    let size_expr = if !node.children.is_empty() {
                        self.expr_to_string(&node.children[0])
                    } else {
                        "0".to_string()
                    };
                    let default_val = default_value_for_type(element_type);
                    // Allocate with size header so delete[] can free correctly
                    format!(
                        "unsafe {{ fragile_new_array::<{}>({} as usize, {}) }}",
                        element_type.to_rust_type_str(),
                        size_expr,
                        default_val
                    )
                } else {
                    // new T(args) → Box::into_raw(Box::new(value))
                    // Find the actual initializer, skipping TypeRef nodes
                    let init_node = node.children.iter().find(|c| {
                        !matches!(&c.kind, ClangNodeKind::Unknown(s) if s.starts_with("TypeRef"))
                    });
                    let init = if let Some(init_node) = init_node {
                        // Constructor argument or initializer
                        self.expr_to_string(init_node)
                    } else {
                        // Default value for type
                        default_value_for_type(ty)
                    };
                    format!("Box::into_raw(Box::new({}))", init)
                }
            }
            ClangNodeKind::CXXDeleteExpr { is_array } => {
                if *is_array {
                    if !node.children.is_empty() {
                        let ptr = self.expr_to_string(&node.children[0]);
                        let elem_type = Self::get_expr_type(&node.children[0])
                            .and_then(|t| t.pointee().cloned());
                        let elem_type_str = elem_type
                            .map(|t| t.to_rust_type_str())
                            .unwrap_or_else(|| "u8".to_string());
                        format!("unsafe {{ fragile_delete_array::<{}>({}) }}", elem_type_str, ptr)
                    } else {
                        "/* delete[] error: no pointer */".to_string()
                    }
                } else if !node.children.is_empty() {
                    // delete p → drop(unsafe { Box::from_raw(p) })
                    let ptr = self.expr_to_string(&node.children[0]);
                    format!("drop(unsafe {{ Box::from_raw({}) }})", ptr)
                } else {
                    "/* delete error */".to_string()
                }
            }
            ClangNodeKind::StringLiteral(s) => {
                // Convert C++ string literal to Rust *const i8 using byte string
                // "hello" -> b"hello\0".as_ptr() as *const i8
                format!("b\"{}\\0\".as_ptr() as *const i8", s.escape_default())
            }
            ClangNodeKind::DeclRefExpr { name, namespace_path, ty, .. } => {
                if name == "this" {
                    if self.use_ctor_self { "__self".to_string() } else { "self".to_string() }
                } else {
                    // Check for standard I/O streams (std::cout, std::cerr, std::cin)
                    // These should be mapped to Rust's std::io functions
                    let is_std_namespace = namespace_path.len() == 1 && namespace_path[0] == "std";
                    if is_std_namespace || namespace_path.is_empty() {
                        match name.as_str() {
                            "cout" => return "std::io::stdout()".to_string(),
                            "cerr" | "clog" => return "std::io::stderr()".to_string(),
                            "cin" => return "std::io::stdin()".to_string(),
                            _ => {}
                        }
                    }

                    let ident = sanitize_identifier(name);
                    // Check if this is a static member access (class name in namespace path)
                    // For static member variables (not functions), convert to global with unsafe
                    if !namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                        // Check if the last component is a class name with a static member
                        let class_name = &namespace_path[namespace_path.len() - 1];
                        if let Some(global_name) = self.static_members.get(&(class_name.clone(), name.clone())) {
                            return format!("unsafe {{ {} }}", global_name);
                        }
                        // Try fallback: generate from convention if it looks like a static member
                        // (class name followed by member name, no function type)
                        let global_name = format!("{}_{}", class_name.to_uppercase(), ident.to_uppercase());
                        // Check if this global exists in our static_members for any class
                        let is_static_member = self.static_members.values().any(|g| g == &global_name);
                        if is_static_member {
                            return format!("unsafe {{ {} }}", global_name);
                        }
                    }

                    // Check if this is a static member of the current class (accessed without Class:: prefix)
                    if namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                        if let Some(ref current_class) = self.current_class {
                            if let Some(global_name) = self.static_members.get(&(current_class.clone(), name.clone())) {
                                return format!("unsafe {{ {} }}", global_name);
                            }
                        }
                    }

                    // Check if this is a global variable (needs unsafe access)
                    if self.global_vars.contains(name) {
                        return format!("unsafe {{ {} }}", ident);
                    }

                    // Compute relative path based on current namespace context
                    // Only apply to functions (not local variables or parameters)
                    // For functions, even if namespace_path is empty, we may need super:: to reach global scope
                    let full_path = if matches!(ty, CppType::Function { .. }) {
                        self.compute_relative_path(namespace_path, &ident)
                    } else if namespace_path.is_empty() {
                        // Local variable or parameter - just use the identifier
                        ident.clone()
                    } else {
                        // Namespaced non-function (shouldn't happen often)
                        self.compute_relative_path(namespace_path, &ident)
                    };
                    // Dereference reference variables (parameters or locals with & type)
                    if self.ref_vars.contains(name) {
                        format!("*{}", full_path)
                    } else {
                        full_path
                    }
                }
            }
            ClangNodeKind::CXXThisExpr { .. } => {
                if self.use_ctor_self { "__self".to_string() } else { "self".to_string() }
            }
            ClangNodeKind::BinaryOperator { op, .. } => {
                if node.children.len() >= 2 {
                    // Handle comma operator specially: (a, b) => { a; b }
                    if matches!(op, BinaryOp::Comma) {
                        let left = self.expr_to_string(&node.children[0]);
                        let right = self.expr_to_string(&node.children[1]);
                        return format!("{{ {}; {} }}", left, right);
                    }

                    // Handle three-way comparison (spaceship) operator: a <=> b
                    // Returns an i8 that can be compared to 0 (like C++ std::strong_ordering)
                    if matches!(op, BinaryOp::Spaceship) {
                        let left = self.expr_to_string(&node.children[0]);
                        let right = self.expr_to_string(&node.children[1]);
                        // Use Ord::cmp and cast to i8 (-1, 0, 1) to match C++ semantics
                        return format!("({}.cmp(&{}) as i8)", left, right);
                    }

                    let op_str = binop_to_string(op);

                    // Check if left side is a pointer dereference, pointer subscript, static member,
                    // global array subscript, global variable, or arrow member access (needs whole assignment in unsafe)
                    let left_is_deref = Self::is_pointer_deref(&node.children[0]);
                    let left_is_ptr_subscript = self.is_pointer_subscript(&node.children[0]);
                    let left_is_static_member = self.is_static_member_access(&node.children[0]);
                    let left_is_global_subscript = self.is_global_array_subscript(&node.children[0]);
                    let left_is_global_var = self.is_global_var_expr(&node.children[0]);
                    let left_is_arrow = Self::is_arrow_member_access(&node.children[0]);
                    let needs_unsafe = left_is_deref || left_is_ptr_subscript || left_is_static_member || left_is_global_subscript || left_is_global_var || left_is_arrow;

                    // Check if left side is a pointer type for += / -= (need .add() / .sub())
                    let left_type = Self::get_expr_type(&node.children[0]);
                    let left_is_pointer = matches!(left_type, Some(CppType::Pointer { .. }));

                    // Handle function pointer comparison with nullptr: use .is_none() / .is_some()
                    let left_is_fn_ptr = left_type.as_ref().map_or(false, |t| Self::is_function_pointer_type(t));
                    if left_is_fn_ptr && matches!(op, BinaryOp::Eq | BinaryOp::Ne) && Self::is_nullptr_literal(&node.children[1]) {
                        let left = self.expr_to_string(&node.children[0]);
                        return if matches!(op, BinaryOp::Eq) {
                            format!("{}.is_none()", left)
                        } else {
                            format!("{}.is_some()", left)
                        };
                    }

                    // Handle pointer arithmetic specially
                    if left_is_pointer && matches!(op, BinaryOp::AddAssign | BinaryOp::SubAssign) {
                        let left = self.expr_to_string(&node.children[0]);
                        let right = self.expr_to_string(&node.children[1]);
                        let method = if matches!(op, BinaryOp::AddAssign) { "add" } else { "sub" };
                        format!("{} = {}.{}({} as usize)", left, left, method, right)
                    } else if matches!(op, BinaryOp::Assign | BinaryOp::AddAssign | BinaryOp::SubAssign |
                                   BinaryOp::MulAssign | BinaryOp::DivAssign |
                                   BinaryOp::RemAssign | BinaryOp::AndAssign |
                                   BinaryOp::OrAssign | BinaryOp::XorAssign |
                                   BinaryOp::ShlAssign | BinaryOp::ShrAssign) && needs_unsafe {
                        // For pointer dereference, subscript, or static member on left side, wrap entire assignment in unsafe
                        let left_raw = self.expr_to_string_raw(&node.children[0]);
                        let right_raw = self.expr_to_string_raw(&node.children[1]);
                        format!("unsafe {{ {} {} {} }}", left_raw, op_str, right_raw)
                    } else {
                        let left = self.expr_to_string(&node.children[0]);
                        let right = self.expr_to_string(&node.children[1]);
                        format!("{} {} {}", left, op_str, right)
                    }
                } else {
                    "/* binary op error */".to_string()
                }
            }
            ClangNodeKind::UnaryOperator { op, ty } => {
                if !node.children.is_empty() {
                    // Check if operand is a global variable (needs special handling for inc/dec)
                    let is_global = self.is_global_var_expr(&node.children[0]);

                    let operand = self.expr_to_string(&node.children[0]);
                    match op {
                        UnaryOp::Minus => format!("-{}", operand),
                        UnaryOp::Plus => operand,
                        UnaryOp::LNot => format!("!{}", operand),
                        UnaryOp::Not => format!("!{}", operand),  // bitwise not ~ in C++
                        UnaryOp::AddrOf => {
                            // Check if this is a pointer to a polymorphic class
                            if let CppType::Pointer { pointee, is_const } = ty {
                                if let CppType::Named(class_name) = pointee.as_ref() {
                                    if self.polymorphic_classes.contains(class_name) {
                                        // For polymorphic types, just return reference (no cast needed)
                                        return if *is_const {
                                            format!("&{}", operand)
                                        } else {
                                            format!("&mut {}", operand)
                                        };
                                    }
                                }
                            }
                            // For regular C++ pointers, cast reference to raw pointer
                            let rust_ty = ty.to_rust_type_str();
                            if rust_ty.starts_with("*mut ") {
                                format!("&mut {} as {}", operand, rust_ty)
                            } else if rust_ty.starts_with("*const ") {
                                format!("&{} as {}", operand, rust_ty)
                            } else {
                                format!("&{}", operand)
                            }
                        }
                        UnaryOp::Deref => {
                            // Check if we're dereferencing 'this' - in C++ *this gives the object,
                            // in Rust 'self' is already the object (not a pointer)
                            if matches!(&node.children[0].kind, ClangNodeKind::CXXThisExpr { .. }) {
                                operand // Just return 'self' directly
                            } else {
                                // Raw pointer dereference needs unsafe
                                format!("unsafe {{ *{} }}", operand)
                            }
                        }
                        UnaryOp::PreInc | UnaryOp::PreDec => {
                            let is_pointer = matches!(ty, CppType::Pointer { .. });
                            // For global variables, wrap entire operation in unsafe
                            if is_global {
                                let raw_name = self.get_raw_var_name(&node.children[0]).unwrap_or(operand.clone());
                                if is_pointer {
                                    let method = if matches!(op, UnaryOp::PreInc) { "add" } else { "sub" };
                                    format!("unsafe {{ {} = {}.{}(1); {} }}", raw_name, raw_name, method, raw_name)
                                } else {
                                    let op_str = if matches!(op, UnaryOp::PreInc) { "+=" } else { "-=" };
                                    format!("unsafe {{ {} {} 1; {} }}", raw_name, op_str, raw_name)
                                }
                            } else if is_pointer {
                                let method = if matches!(op, UnaryOp::PreInc) { "add" } else { "sub" };
                                format!("{{ {} = {}.{}(1); {} }}", operand, operand, method, operand)
                            } else {
                                let op_str = if matches!(op, UnaryOp::PreInc) { "+=" } else { "-=" };
                                format!("{{ {} {} 1; {} }}", operand, op_str, operand)
                            }
                        }
                        UnaryOp::PostInc | UnaryOp::PostDec => {
                            let is_pointer = matches!(ty, CppType::Pointer { .. });
                            // For global variables, wrap entire operation in unsafe
                            if is_global {
                                let raw_name = self.get_raw_var_name(&node.children[0]).unwrap_or(operand.clone());
                                if is_pointer {
                                    let method = if matches!(op, UnaryOp::PostInc) { "add" } else { "sub" };
                                    format!("unsafe {{ let __v = {}; {} = {}.{}(1); __v }}", raw_name, raw_name, raw_name, method)
                                } else {
                                    let op_str = if matches!(op, UnaryOp::PostInc) { "+=" } else { "-=" };
                                    format!("unsafe {{ let __v = {}; {} {} 1; __v }}", raw_name, raw_name, op_str)
                                }
                            } else if is_pointer {
                                let method = if matches!(op, UnaryOp::PostInc) { "add" } else { "sub" };
                                format!("{{ let __v = {}; {} = {}.{}(1); __v }}", operand, operand, operand, method)
                            } else {
                                let op_str = if matches!(op, UnaryOp::PostInc) { "+=" } else { "-=" };
                                format!("{{ let __v = {}; {} {} 1; __v }}", operand, operand, op_str)
                            }
                        }
                    }
                } else {
                    "/* unary op error */".to_string()
                }
            }
            ClangNodeKind::CallExpr { ty } => {
                // Check if this is a std::get call on a variant
                if let Some((variant_arg, variant_type, return_type)) = Self::is_std_get_call(node) {
                    if let Some(idx) = Self::get_variant_index_from_return_type(&variant_type, return_type) {
                        if let Some(enum_name) = Self::get_variant_enum_name(&variant_type) {
                            let variant_expr = self.expr_to_string(variant_arg);
                            // Generate match expression to extract the variant value
                            // Using clone() to copy the value out since we're borrowing
                            return format!(
                                "match &{} {{ {}::V{}(val) => val.clone(), _ => panic!(\"bad variant access\") }}",
                                variant_expr, enum_name, idx
                            );
                        }
                    }
                }

                // Check if this is a std::visit call on variant(s)
                if let Some((visitor_node, variants)) = Self::is_std_visit_call(node) {
                    return self.generate_visit_match(visitor_node, &variants, ty);
                }

                // Check if this is an I/O stream output operation (cout << x << y)
                if let Some((stream_type, args)) = self.collect_stream_output_args(node) {
                    return self.generate_stream_write(stream_type, &args);
                }

                // Check if this is an I/O stream input operation (cin >> x >> y)
                if let Some((_stream_type, args)) = self.collect_stream_input_args(node) {
                    return self.generate_stream_read(&args);
                }

                // Check if this is a std::views range adaptor call (filter, transform, take, drop, reverse)
                if let Some((adaptor, range_node, arg_node)) = Self::is_std_views_adaptor_call(node) {
                    let range_expr = self.expr_to_string(range_node);
                    match adaptor {
                        "rev" => {
                            // reverse doesn't take an argument
                            return format!("{}.iter().rev()", range_expr);
                        }
                        "take" | "skip" => {
                            // take/drop take a count argument
                            if let Some(arg) = arg_node {
                                let count_expr = self.expr_to_string(arg);
                                return format!("{}.iter().{}({})", range_expr, adaptor, count_expr);
                            }
                        }
                        "filter" | "map" | "take_while" | "skip_while" => {
                            // filter/transform take a predicate/function argument
                            if let Some(arg) = arg_node {
                                let pred_expr = self.expr_to_string(arg);
                                return format!("{}.iter().{}({})", range_expr, adaptor, pred_expr);
                            }
                        }
                        _ => {}
                    }
                }

                // Check if this is a std::ranges algorithm call (for_each, find, sort, copy)
                if let Some((algo, range_node, arg_node)) = Self::is_std_ranges_algorithm_call(node) {
                    let range_expr = self.expr_to_string(range_node);
                    match algo {
                        "for_each" => {
                            if let Some(arg) = arg_node {
                                let func_expr = self.expr_to_string(arg);
                                return format!("{}.iter().for_each({})", range_expr, func_expr);
                            }
                        }
                        "find" => {
                            if let Some(arg) = arg_node {
                                let pred_expr = self.expr_to_string(arg);
                                return format!("{}.iter().find({})", range_expr, pred_expr);
                            }
                        }
                        "sort" => {
                            // sort takes the range and optionally a comparator
                            if let Some(arg) = arg_node {
                                let cmp_expr = self.expr_to_string(arg);
                                return format!("{}.sort_by({})", range_expr, cmp_expr);
                            } else {
                                return format!("{}.sort()", range_expr);
                            }
                        }
                        "collect" => {
                            // copy → collect into a new container
                            return format!("{}.iter().cloned().collect::<Vec<_>>()", range_expr);
                        }
                        "any" => {
                            if let Some(arg) = arg_node {
                                let pred_expr = self.expr_to_string(arg);
                                return format!("{}.iter().any({})", range_expr, pred_expr);
                            }
                        }
                        "all" => {
                            if let Some(arg) = arg_node {
                                let pred_expr = self.expr_to_string(arg);
                                return format!("{}.iter().all({})", range_expr, pred_expr);
                            }
                        }
                        "count" => {
                            if let Some(arg) = arg_node {
                                let pred_expr = self.expr_to_string(arg);
                                return format!("{}.iter().filter({}).count()", range_expr, pred_expr);
                            } else {
                                return format!("{}.iter().count()", range_expr);
                            }
                        }
                        _ => {}
                    }
                }

                // Check if this is an explicit destructor call (obj->~ClassName())
                // For placement new cleanup, we need to call drop_in_place instead of ~ClassName()
                if let Some(destructor_ptr) = self.get_explicit_destructor_call(node) {
                    return format!("unsafe {{ std::ptr::drop_in_place({}) }}", destructor_ptr);
                }

                // Check if this is a lambda/closure call (operator() on a lambda type)
                // Lambda types look like "(lambda at /path/file.cpp:line:col)"
                if let Some((op_name, left_idx, _)) = Self::get_operator_call_info(node) {
                    if op_name == "operator()" {
                        // Check if the left operand is a lambda variable
                        let callee_type = Self::get_expr_type(&node.children[left_idx]);
                        if let Some(CppType::Named(name)) = callee_type {
                            if name.contains("lambda at ") {
                                // This is a closure call - generate simple function call syntax
                                let callee = self.expr_to_string(&node.children[left_idx]);
                                let args: Vec<String> = node.children.iter()
                                    .enumerate()
                                    .filter(|(i, c)| {
                                        // Skip the callee and the operator() reference
                                        *i != left_idx && !Self::is_function_reference(c)
                                    })
                                    .map(|(_, c)| self.expr_to_string(c))
                                    .collect();
                                return format!("{}({})", callee, args.join(", "));
                            }
                        }
                    }
                }

                // Check if this is an operator overload call (e.g., a + b)
                if let Some((op_name, left_idx, right_idx_opt)) = Self::get_operator_call_info(node) {
                    // Convert operator name to method name (operator+ -> op_add)
                    let method_name = sanitize_identifier(&op_name);
                    let left_operand = self.expr_to_string(&node.children[left_idx]);

                    if op_name == "operator()" {
                        // Function call operator: callee.op_call(args...)
                        // Collect all children except the callee and the operator() reference
                        let args: Vec<String> = node.children.iter()
                            .enumerate()
                            .filter(|(i, c)| {
                                *i != left_idx && !Self::is_function_reference(c)
                            })
                            .map(|(_, c)| self.expr_to_string(c))
                            .collect();
                        format!("{}.{}({})", left_operand, method_name, args.join(", "))
                    } else if op_name == "operator[]" {
                        // Subscript operator: *array.op_index(idx) - dereference for C++ semantics
                        // In C++, arr[i] returns a reference that auto-dereferences.
                        // We dereference here to make reads work; assignments need special handling.
                        if let Some(right_idx) = right_idx_opt {
                            let right_operand = self.expr_to_string(&node.children[right_idx]);
                            format!("*{}.{}({})", left_operand, method_name, right_operand)
                        } else {
                            format!("*{}.{}()", left_operand, method_name)
                        }
                    } else if op_name == "operator*" && right_idx_opt.is_none() {
                        // Unary dereference operator: *ptr → *ptr.op_deref()
                        // The operator returns a reference, so we dereference it
                        format!("*{}.op_deref()", left_operand)
                    } else if op_name == "operator->" {
                        // Arrow operator: ptr->member
                        // This is handled in MemberExpr, but if called directly, returns the pointer
                        format!("{}.op_arrow()", left_operand)
                    } else if let Some(right_idx) = right_idx_opt {
                        // Binary operator: left.op_X(right) or left.op_X(&right)
                        let right_operand = self.expr_to_string(&node.children[right_idx]);

                        // Special case: type_info comparison (typeid == typeid)
                        // Use native Rust == / != since std::any::TypeId supports it directly
                        let left_is_typeid = matches!(&node.children[left_idx].kind, ClangNodeKind::TypeidExpr { .. })
                            || Self::contains_typeid_expr(&node.children[left_idx]);
                        let right_is_typeid = matches!(&node.children[right_idx].kind, ClangNodeKind::TypeidExpr { .. })
                            || Self::contains_typeid_expr(&node.children[right_idx]);

                        if left_is_typeid && right_is_typeid && (op_name == "operator==" || op_name == "operator!=") {
                            let rust_op = if op_name == "operator==" { "==" } else { "!=" };
                            return format!("{} {} {}", left_operand, rust_op, right_operand);
                        }

                        let right_type = Self::get_expr_type(&node.children[right_idx]);
                        // Pass class/struct types by reference, primitives by value
                        let needs_ref = matches!(right_type, Some(CppType::Named(_)));
                        if needs_ref {
                            format!("{}.{}(&{})", left_operand, method_name, right_operand)
                        } else {
                            format!("{}.{}({})", left_operand, method_name, right_operand)
                        }
                    } else {
                        // Other unary operators: operand.op_X()
                        format!("{}.{}()", left_operand, method_name)
                    }
                } else if let CppType::Named(cpp_struct_name) = ty {
                    // Convert C++ type name to valid Rust identifier
                    let struct_name = CppType::Named(cpp_struct_name.clone()).to_rust_type_str();

                    // Check if this is a function call (not a constructor)
                    // A function call has a DeclRefExpr child with Function type
                    let is_function_call = node.children.iter().any(|c| {
                        Self::is_function_reference(c)
                    });

                    if is_function_call && !node.children.is_empty() {
                        // Regular function call that returns a struct
                        let func = self.expr_to_string(&node.children[0]);
                        // Strip Some() wrapper if present - callee shouldn't be wrapped
                        // (FunctionToPointerDecay on callee is just a C++ technicality)
                        let func = Self::strip_some_wrapper(&func);
                        let args: Vec<String> = node.children[1..].iter()
                            .map(|c| self.expr_to_string(c))
                            .collect();
                        format!("{}({})", func, args.join(", "))
                    } else {
                        // Constructor call: all children are arguments (but skip TypeRef nodes)
                        // For copy constructors (1 argument of same type), pass by reference
                        let args: Vec<String> = node.children.iter()
                            .filter(|c| {
                                // Skip TypeRef nodes (they're type references, not arguments)
                                if let ClangNodeKind::Unknown(s) = &c.kind {
                                    if s.starts_with("TypeRef:") || s == "TypeRef" {
                                        return false;
                                    }
                                }
                                true
                            })
                            .map(|c| {
                                let arg_str = self.expr_to_string(c);
                                // Check if this is a copy constructor call (arg type matches struct)
                                let arg_type = Self::get_expr_type(c);
                                let arg_class = Self::extract_class_name(&arg_type);
                                if let Some(name) = arg_class {
                                    // Compare using C++ name for copy constructor detection
                                    if name == *cpp_struct_name {
                                        // Pass by reference for copy constructor
                                        return format!("&{}", arg_str);
                                    }
                                }
                                arg_str
                            })
                            .collect();
                        let num_args = args.len();
                        // Always use StructName::new_N(args) to ensure custom constructor bodies run
                        format!("{}::new_{}({})", struct_name, num_args, args.join(", "))
                    }
                } else if !node.children.is_empty() {
                    // Check if this is a virtual base method call
                    if let Some((base, vbase_field, method)) = self.get_virtual_base_method_call_info(&node.children[0]) {
                        let args: Vec<String> = node.children[1..].iter()
                            .map(|c| self.expr_to_string(c))
                            .collect();
                        return format!("unsafe {{ (*{}.{}).{}({}) }}", base, vbase_field, method, args.join(", "));
                    }

                    // Regular function call: first child is the function reference, rest are arguments
                    let func = self.expr_to_string(&node.children[0]);
                    // Strip Some() wrapper if present - callee shouldn't be wrapped
                    // (FunctionToPointerDecay on callee is just a C++ technicality)
                    let func = Self::strip_some_wrapper(&func);

                    // Check if this is a call through a function pointer variable
                    // Function pointers are represented as Option<fn(...)>, so we need .unwrap()
                    let is_fn_ptr_call = Self::is_function_pointer_variable(&node.children[0]);

                    // Try to get function parameter types to handle reference parameters
                    let param_types = Self::get_function_param_types(&node.children[0]);

                    let args: Vec<String> = node.children[1..].iter()
                        .enumerate()
                        .map(|(i, c)| {
                            // Check if this parameter expects specific handling
                            if let Some(ref types) = param_types {
                                if i < types.len() {
                                    // Handle reference parameters
                                    if let CppType::Reference { is_const, .. } = &types[i] {
                                        // Check if argument is a reference variable
                                        if let Some(ref_ident) = self.get_ref_var_ident(c) {
                                            // Pass the reference variable directly (without dereferencing)
                                            return ref_ident;
                                        } else {
                                            // Add borrow for non-reference-variable arguments
                                            let arg_str = self.expr_to_string(c);
                                            let prefix = if *is_const { "&" } else { "&mut " };
                                            return format!("{}{}", prefix, arg_str);
                                        }
                                    }
                                    // Handle pointer parameters with array arguments
                                    // Also handle unsized array parameters (which are really pointers)
                                    if matches!(&types[i], CppType::Pointer { .. })
                                        || matches!(&types[i], CppType::Array { size: None, .. }) {
                                        let arg_type = Self::get_expr_type(c);
                                        let is_array = matches!(arg_type, Some(CppType::Array { .. }));
                                        if is_array {
                                            // Array to pointer decay
                                            let arg_str = self.expr_to_string(c);
                                            return format!("{}.as_mut_ptr()", arg_str);
                                        }
                                        // Also check using variable tracking
                                        if let Some(arr_ident) = self.get_array_var_ident(c) {
                                            return format!("{}.as_mut_ptr()", arr_ident);
                                        }
                                    }
                                }
                            }
                            self.expr_to_string(c)
                        })
                        .collect();

                    // Check if this is a compiler builtin function call
                    if let Some((rust_code, needs_unsafe)) = Self::map_builtin_function(&func, &args) {
                        return if needs_unsafe {
                            format!("unsafe {{ {} }}", rust_code)
                        } else {
                            rust_code
                        };
                    }

                    // Check if the function expression is wrapped in unsafe (from arrow member access)
                    // If so, put the function call inside the unsafe block
                    if func.starts_with("unsafe { ") && func.ends_with(" }") {
                        let inner = &func[9..func.len()-2]; // Extract "(*...).method" from "unsafe { (*...).method }"
                        format!("unsafe {{ {}({}) }}", inner, args.join(", "))
                    } else if is_fn_ptr_call {
                        // Function pointer call: need to unwrap the Option<fn(...)>
                        format!("{}.unwrap()({})", func, args.join(", "))
                    } else {
                        format!("{}({})", func, args.join(", "))
                    }
                } else {
                    "/* call error */".to_string()
                }
            }
            ClangNodeKind::MemberExpr { member_name, is_arrow, declaring_class, is_static, .. } => {
                // Check for static member access first
                if *is_static {
                    // Look up the global variable name for this static member
                    if let Some(class_name) = declaring_class {
                        if let Some(global_name) = self.static_members.get(&(class_name.clone(), member_name.clone())) {
                            return format!("unsafe {{ {} }}", global_name);
                        }
                    }
                    // Fallback: generate global name from convention
                    if let Some(class_name) = declaring_class {
                        let global_name = format!("{}_{}", class_name.to_uppercase(), sanitize_identifier(member_name).to_uppercase());
                        return format!("unsafe {{ {} }}", global_name);
                    }
                }

                if !node.children.is_empty() {
                    let base = self.expr_to_string(&node.children[0]);
                    // Check if this is accessing an inherited member
                    let base_type = Self::get_expr_type(&node.children[0]);

                    // Determine if we need base access and get the correct base field name
                    // Skip base access for anonymous struct members (they are flattened into parent)
                    let (needs_base_access, base_access) = if let Some(decl_class) = declaring_class {
                        // Anonymous struct members are flattened - access directly
                        if decl_class.starts_with("(anonymous") || decl_class.starts_with("__anon_") {
                            (false, BaseAccess::DirectField(String::new()))
                        } else {
                            let base_class_name = Self::extract_class_name(&base_type);
                            if let Some(name) = base_class_name {
                                if name != *decl_class {
                                    // Need base access - get correct field for MI support
                                    let access = self.get_base_access_for_class(&name, decl_class);
                                    (true, access)
                                } else {
                                    (false, BaseAccess::DirectField(String::new()))
                                }
                            } else {
                                (false, BaseAccess::DirectField(String::new()))
                            }
                        }
                    } else {
                        (false, BaseAccess::DirectField(String::new()))
                    };

                    let member = sanitize_identifier(member_name);
                    if *is_arrow {
                        // Check if this is a trait object (polymorphic pointer)
                        // Trait objects are already references, so no dereference needed
                        let is_trait_object = if let Some(ref ty) = base_type {
                            if let CppType::Pointer { pointee, .. } = ty {
                                if let CppType::Named(class_name) = pointee.as_ref() {
                                    self.polymorphic_classes.contains(class_name)
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        if is_trait_object {
                            // For trait objects, no dereference - call method directly
                            format!("{}.{}", base, member)
                        } else if needs_base_access {
                            match base_access {
                                BaseAccess::VirtualPtr(field) => {
                                    format!("unsafe {{ (*(*{}).{}).{} }}", base, field, member)
                                }
                                BaseAccess::DirectField(field) | BaseAccess::FieldChain(field) => {
                                    // Dereferencing raw pointers requires unsafe
                                    format!("unsafe {{ (*{}).{}.{} }}", base, field, member)
                                }
                            }
                        } else {
                            // Dereferencing raw pointers requires unsafe
                            format!("unsafe {{ (*{}).{} }}", base, member)
                        }
                    } else {
                        if needs_base_access {
                            match base_access {
                                BaseAccess::VirtualPtr(field) => {
                                    format!("unsafe {{ (*{}.{}).{} }}", base, field, member)
                                }
                                BaseAccess::DirectField(field) | BaseAccess::FieldChain(field) => {
                                    format!("{}.{}.{}", base, field, member)
                                }
                            }
                        } else {
                            format!("{}.{}", base, member)
                        }
                    }
                } else {
                    // Implicit this - check if member is inherited
                    let member = sanitize_identifier(member_name);
                    let self_name = if self.use_ctor_self { "__self" } else { "self" };
                    let (needs_base_access, base_access) = if let (Some(current), Some(decl_class)) = (&self.current_class, declaring_class) {
                        // Anonymous struct members are flattened - access directly
                        if decl_class.starts_with("(anonymous") || decl_class.starts_with("__anon_") {
                            (false, BaseAccess::DirectField(String::new()))
                        } else if current != decl_class {
                            let access = self.get_base_access_for_class(current, decl_class);
                            (true, access)
                        } else {
                            (false, BaseAccess::DirectField(String::new()))
                        }
                    } else {
                        (false, BaseAccess::DirectField(String::new()))
                    };
                    if needs_base_access {
                        match base_access {
                            BaseAccess::VirtualPtr(field) => {
                                format!("unsafe {{ (*{}.{}).{} }}", self_name, field, member)
                            }
                            BaseAccess::DirectField(field) | BaseAccess::FieldChain(field) => {
                                format!("{}.{}.{}", self_name, field, member)
                            }
                        }
                    } else {
                        format!("{}.{}", self_name, member)
                    }
                }
            }
            ClangNodeKind::ArraySubscriptExpr { .. } => {
                if node.children.len() >= 2 {
                    // Check if the array expression is a global variable
                    let is_global_array = self.is_global_var_expr(&node.children[0]);

                    let idx = self.expr_to_string(&node.children[1]);
                    // Check if the array expression is a pointer type
                    // (also check for unsized arrays which decay to pointers)
                    let arr_type = Self::get_expr_type(&node.children[0]);
                    let is_pointer = matches!(arr_type, Some(CppType::Pointer { .. }))
                        || matches!(arr_type, Some(CppType::Array { size: None, .. }))
                        || self.is_ptr_var_expr(&node.children[0]);

                    if is_global_array {
                        // For global arrays, get raw name and put indexing inside unsafe
                        let raw_name = self.get_raw_var_name(&node.children[0])
                            .unwrap_or_else(|| self.expr_to_string(&node.children[0]));
                        format!("unsafe {{ {}[{} as usize] }}", raw_name, idx)
                    } else if is_pointer {
                        let arr = self.expr_to_string(&node.children[0]);
                        // Pointer indexing requires unsafe pointer arithmetic
                        format!("unsafe {{ *{}.add({} as usize) }}", arr, idx)
                    } else {
                        let arr = self.expr_to_string(&node.children[0]);
                        // Array indexing - cast index to usize
                        format!("{}[{} as usize]", arr, idx)
                    }
                } else {
                    "/* array subscript error */".to_string()
                }
            }
            ClangNodeKind::ConditionalOperator { .. } => {
                if node.children.len() >= 3 {
                    let cond = self.expr_to_string(&node.children[0]);
                    let then_expr = self.expr_to_string(&node.children[1]);
                    let else_expr = self.expr_to_string(&node.children[2]);
                    format!("if {} {{ {} }} else {{ {} }}", cond, then_expr, else_expr)
                } else {
                    "/* ternary error */".to_string()
                }
            }
            ClangNodeKind::ParenExpr { .. } => {
                // Preserve parentheses
                if !node.children.is_empty() {
                    format!("({})", self.expr_to_string(&node.children[0]))
                } else {
                    "()".to_string()
                }
            }
            ClangNodeKind::ImplicitCastExpr { cast_kind, ty } => {
                // Handle implicit casts - some need explicit conversion in Rust
                if !node.children.is_empty() {
                    let inner = self.expr_to_string(&node.children[0]);
                    match cast_kind {
                        CastKind::IntegralCast => {
                            // Need explicit cast for integral conversions
                            let rust_type = ty.to_rust_type_str();
                            format!("{} as {}", inner, rust_type)
                        }
                        CastKind::FloatingCast | CastKind::IntegralToFloating | CastKind::FloatingToIntegral => {
                            // Need explicit cast for floating conversions
                            let rust_type = ty.to_rust_type_str();
                            format!("{} as {}", inner, rust_type)
                        }
                        CastKind::FunctionToPointerDecay => {
                            // Function to pointer decay - wrap in Some() for Option<fn(...)> type
                            format!("Some({})", inner)
                        }
                        _ => {
                            // Most casts pass through (LValueToRValue, ArrayToPointerDecay, etc.)
                            inner
                        }
                    }
                } else {
                    "()".to_string()
                }
            }
            ClangNodeKind::CastExpr { ty, cast_kind } => {
                // Explicit C++ casts: static_cast, reinterpret_cast, const_cast, C-style
                if !node.children.is_empty() {
                    // Check for functional cast to Named type (like Widget(v))
                    // This is a constructor call, just pass through
                    if let CppType::Named(_) = ty {
                        if *cast_kind == CastKind::Other {
                            // This is likely a CXXFunctionalCastExpr (constructor syntax)
                            // Find the CallExpr among children (skip TypeRef nodes)
                            for child in &node.children {
                                if matches!(&child.kind, ClangNodeKind::CallExpr { .. }) {
                                    return self.expr_to_string(child);
                                }
                                // Also check through Unknown wrappers
                                if let ClangNodeKind::Unknown(s) = &child.kind {
                                    if !s.starts_with("TypeRef") {
                                        return self.expr_to_string(child);
                                    }
                                }
                            }
                            // Fallback to first non-TypeRef child
                            for child in &node.children {
                                if let ClangNodeKind::Unknown(s) = &child.kind {
                                    if s.starts_with("TypeRef") {
                                        continue;
                                    }
                                }
                                return self.expr_to_string(child);
                            }
                        }
                    }

                    let inner = self.expr_to_string(&node.children[0]);
                    let rust_type = ty.to_rust_type_str();
                    match cast_kind {
                        CastKind::Static | CastKind::Reinterpret => {
                            // Generate Rust "as" cast
                            format!("{} as {}", inner, rust_type)
                        }
                        CastKind::Const => {
                            // const_cast usually just changes mutability, pass through
                            inner
                        }
                        CastKind::Other => {
                            // For other cast kinds (primitive types), generate as cast
                            format!("{} as {}", inner, rust_type)
                        }
                        _ => {
                            // For other cast kinds, generate as cast
                            format!("{} as {}", inner, rust_type)
                        }
                    }
                } else {
                    "()".to_string()
                }
            }
            ClangNodeKind::InitListExpr { ty } => {
                // Aggregate initialization
                if let CppType::Named(name) = ty {
                    // Check if this is designated initialization (children have MemberRef)
                    // Designated: { .x = 10, .y = 20 } produces UnexposedExpr(MemberRef, value)
                    // Non-designated: { 10, 20 } produces IntegerLiteral directly
                    let mut field_values: Vec<(String, String)> = Vec::new();
                    let mut has_designators = false;

                    for child in &node.children {
                        // Check if child is UnexposedExpr wrapper with MemberRef designator
                        if matches!(&child.kind, ClangNodeKind::Unknown(s) if s == "UnexposedExpr") {
                            if child.children.len() >= 2 {
                                if let ClangNodeKind::MemberRef { name: field_name } = &child.children[0].kind {
                                    // This is a designated initializer
                                    has_designators = true;
                                    // The value is the second child (or beyond)
                                    let value = self.expr_to_string(&child.children[1]);
                                    field_values.push((field_name.clone(), value));
                                    continue;
                                }
                            }
                        }
                        // Non-designated: just get the value
                        let value = self.expr_to_string(child);
                        field_values.push((String::new(), value));
                    }

                    if has_designators {
                        // All values have field names from designators
                        let inits: Vec<String> = field_values.iter()
                            .map(|(f, v)| format!("{}: {}", f, v))
                            .collect();
                        format!("{} {{ {} }}", name, inits.join(", "))
                    } else {
                        // Try to get field names for this struct (positional)
                        if let Some(struct_fields) = self.class_fields.get(name) {
                            let inits: Vec<String> = field_values.iter()
                                .enumerate()
                                .map(|(i, (_, v))| {
                                    if i < struct_fields.len() {
                                        format!("{}: {}", struct_fields[i].0, v)
                                    } else {
                                        v.clone()
                                    }
                                })
                                .collect();
                            format!("{} {{ {} }}", name, inits.join(", "))
                        } else {
                            // Fallback: can't determine field names
                            let values: Vec<String> = field_values.into_iter().map(|(_, v)| v).collect();
                            format!("{} {{ {} }}", name, values.join(", "))
                        }
                    }
                } else {
                    let elems: Vec<String> = node.children.iter()
                        .map(|c| self.expr_to_string(c))
                        .collect();
                    format!("[{}]", elems.join(", "))
                }
            }
            ClangNodeKind::LambdaExpr { params, return_type, capture_default, captures } => {
                // Generate Rust closure
                // C++: [captures](params) -> ret { body }
                // Rust: |params| -> ret { body } or move |params| { body }
                use crate::ast::CaptureDefault;

                // Determine if we need 'move' keyword
                let needs_move = *capture_default == CaptureDefault::ByCopy ||
                    captures.iter().any(|(_, by_ref)| !*by_ref);

                // Generate parameter list
                let params_str = params.iter()
                    .map(|(name, ty)| format!("{}: {}", sanitize_identifier(name), ty.to_rust_type_str()))
                    .collect::<Vec<_>>()
                    .join(", ");

                // Generate return type (omit if void)
                let ret_str = if *return_type == CppType::Void {
                    String::new()
                } else {
                    format!(" -> {}", return_type.to_rust_type_str())
                };

                // Find the body (CompoundStmt child)
                let body = node.children.iter()
                    .find(|c| matches!(&c.kind, ClangNodeKind::CompoundStmt));

                let body_str = if let Some(body_node) = body {
                    // Check for simple single-return lambdas
                    if body_node.children.len() == 1 {
                        if let ClangNodeKind::ReturnStmt = &body_node.children[0].kind {
                            if !body_node.children[0].children.is_empty() {
                                // Single return with expression - Rust closure can omit return
                                return if needs_move {
                                    format!("move |{}|{} {}", params_str, ret_str,
                                        self.expr_to_string(&body_node.children[0].children[0]))
                                } else {
                                    format!("|{}|{} {}", params_str, ret_str,
                                        self.expr_to_string(&body_node.children[0].children[0]))
                                };
                            }
                        }
                    }
                    // Multi-statement body - generate block
                    let stmts: Vec<String> = body_node.children.iter()
                        .map(|stmt| self.lambda_stmt_to_string(stmt))
                        .collect();
                    format!("{{ {} }}", stmts.join(" "))
                } else {
                    "{}".to_string()
                };

                if needs_move {
                    format!("move |{}|{} {}", params_str, ret_str, body_str)
                } else {
                    format!("|{}|{} {}", params_str, ret_str, body_str)
                }
            }
            ClangNodeKind::ThrowExpr { exception_ty } => {
                // throw expr → panic!("message")
                // If there's a child expression, try to extract a message
                if !node.children.is_empty() {
                    // Try to get the thrown value - look for StringLiteral in children
                    let msg = Self::extract_throw_message(node);
                    if let Some(m) = msg {
                        format!("panic!(\"{}\")", m)
                    } else if let Some(ty) = exception_ty {
                        format!("panic!(\"Threw {:?}\")", ty)
                    } else {
                        "panic!(\"Exception thrown\")".to_string()
                    }
                } else {
                    // throw; (rethrow) - in Rust, just continue panicking
                    "panic!(\"Rethrow\")".to_string()
                }
            }
            // C++ RTTI expressions
            ClangNodeKind::TypeidExpr { is_type_operand, operand_ty, .. } => {
                // typeid(expr) or typeid(Type) → std::any::TypeId::of::<T>()
                if *is_type_operand {
                    // typeid(Type) → TypeId::of::<RustType>()
                    format!("std::any::TypeId::of::<{}>()", operand_ty.to_rust_type_str())
                } else if !node.children.is_empty() {
                    // typeid(expr) → for polymorphic types, we'd need runtime RTTI
                    // For now, use the static type from the operand
                    let expr = self.expr_to_string(&node.children[0]);
                    format!("/* typeid({}) */ std::any::TypeId::of::<{}>()", expr, operand_ty.to_rust_type_str())
                } else {
                    format!("std::any::TypeId::of::<{}>()", operand_ty.to_rust_type_str())
                }
            }
            ClangNodeKind::DynamicCastExpr { target_ty } => {
                // dynamic_cast has different behavior for pointers vs references:
                // - dynamic_cast<T*>(expr) returns nullptr on failure
                // - dynamic_cast<T&>(expr) throws std::bad_cast on failure
                if !node.children.is_empty() {
                    let expr = self.expr_to_string(&node.children[0]);
                    let target_str = target_ty.to_rust_type_str();

                    match target_ty {
                        CppType::Reference { referent, is_const, .. } => {
                            // Reference dynamic_cast - throws on failure
                            // In Rust, we panic (equivalent to std::bad_cast)
                            let inner_type = referent.to_rust_type_str();
                            // For reference casts, if the cast fails we must panic
                            // This is wrapped in an unsafe block and uses transmute for now
                            format!("unsafe {{ *(({} as *const _ as *const {}) as *{} {}) }}",
                                    expr, inner_type, if *is_const { "const" } else { "mut" }, inner_type)
                        }
                        CppType::Pointer { pointee, is_const } => {
                            // Pointer dynamic_cast - returns null on failure
                            // Generate a safe cast that checks at runtime (placeholder for RTTI)
                            let inner_type = pointee.to_rust_type_str();
                            let ptr_prefix = if *is_const { "*const" } else { "*mut" };
                            format!("/* dynamic_cast: returns null on failure */ {} as {} {}",
                                    expr, ptr_prefix, inner_type)
                        }
                        _ => {
                            // Fallback for unexpected types
                            format!("/* dynamic_cast */ {} as {}", expr, target_str)
                        }
                    }
                } else {
                    format!("/* dynamic_cast to {} without operand */", target_ty.to_rust_type_str())
                }
            }
            // C++20 Coroutine expressions
            ClangNodeKind::CoawaitExpr { .. } => {
                // co_await expr → expr.await
                // In Rust async context, .await suspends until the future is ready
                if !node.children.is_empty() {
                    let operand = self.expr_to_string(&node.children[0]);
                    format!("{}.await", operand)
                } else {
                    "/* co_await without operand */".to_string()
                }
            }
            ClangNodeKind::CoyieldExpr { .. } => {
                // co_yield value → yield value
                // Note: Rust generators are unstable, this generates the syntax
                // that would work with #![feature(generators)]
                if !node.children.is_empty() {
                    let value = self.expr_to_string(&node.children[0]);
                    format!("yield {}", value)
                } else {
                    "yield".to_string()
                }
            }
            ClangNodeKind::CoreturnStmt { value_ty } => {
                // co_return [value] → return [value] (in async/generator context)
                if value_ty.is_some() && !node.children.is_empty() {
                    let value = self.expr_to_string(&node.children[0]);
                    format!("return {}", value)
                } else {
                    "return".to_string()
                }
            }
            _ => {
                // Fallback: try children
                if !node.children.is_empty() {
                    self.expr_to_string(&node.children[0])
                } else {
                    format!("/* unsupported: {:?} */", std::mem::discriminant(&node.kind))
                }
            }
        }
    }

    /// Try to extract a string message from a throw expression.
    /// Looks recursively for StringLiteral nodes.
    fn extract_throw_message(node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::StringLiteral(s) => Some(s.clone()),
            _ => {
                // Recursively search children
                for child in &node.children {
                    if let Some(msg) = Self::extract_throw_message(child) {
                        return Some(msg);
                    }
                }
                None
            }
        }
    }

    /// Convert a statement node to a string for lambda bodies.
    fn lambda_stmt_to_string(&self, node: &ClangNode) -> String {
        match &node.kind {
            ClangNodeKind::ReturnStmt => {
                if node.children.is_empty() {
                    "return;".to_string()
                } else {
                    format!("return {};", self.expr_to_string(&node.children[0]))
                }
            }
            ClangNodeKind::DeclStmt => {
                // Variable declaration - simplified handling
                for child in &node.children {
                    if let ClangNodeKind::VarDecl { name, ty, .. } = &child.kind {
                        let init = if !child.children.is_empty() {
                            format!(" = {}", self.expr_to_string(&child.children[0]))
                        } else {
                            String::new()
                        };
                        return format!("let mut {}: {}{};",
                            sanitize_identifier(name), ty.to_rust_type_str(), init);
                    }
                }
                "/* decl error */".to_string()
            }
            ClangNodeKind::ExprStmt => {
                if !node.children.is_empty() {
                    format!("{};", self.expr_to_string(&node.children[0]))
                } else {
                    ";".to_string()
                }
            }
            _ => {
                // For other statements, try as expression
                format!("{};", self.expr_to_string(node))
            }
        }
    }

    fn writeln(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
    }
}

impl Default for AstCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize a C++ identifier for Rust.
fn sanitize_identifier(name: &str) -> String {
    // Handle operators
    let mut result = if name.starts_with("operator") {
        match name {
            "operator=" => "op_assign".to_string(),
            "operator==" => "op_eq".to_string(),
            "operator!=" => "op_ne".to_string(),
            "operator<" => "op_lt".to_string(),
            "operator<=" => "op_le".to_string(),
            "operator>" => "op_gt".to_string(),
            "operator>=" => "op_ge".to_string(),
            "operator+" => "op_add".to_string(),
            "operator-" => "op_sub".to_string(),
            "operator*" => "op_mul".to_string(),
            "operator/" => "op_div".to_string(),
            "operator%" => "op_rem".to_string(),
            "operator+=" => "op_add_assign".to_string(),
            "operator-=" => "op_sub_assign".to_string(),
            "operator*=" => "op_mul_assign".to_string(),
            "operator/=" => "op_div_assign".to_string(),
            "operator%=" => "op_rem_assign".to_string(),
            "operator&=" => "op_and_assign".to_string(),
            "operator|=" => "op_or_assign".to_string(),
            "operator^=" => "op_xor_assign".to_string(),
            "operator<<=" => "op_shl_assign".to_string(),
            "operator>>=" => "op_shr_assign".to_string(),
            "operator[]" => "op_index".to_string(),
            "operator()" => "op_call".to_string(),
            "operator&" => "op_bitand".to_string(),
            "operator|" => "op_bitor".to_string(),
            "operator^" => "op_bitxor".to_string(),
            "operator~" => "op_bitnot".to_string(),
            "operator<<" => "op_shl".to_string(),
            "operator>>" => "op_shr".to_string(),
            "operator!" => "op_not".to_string(),
            "operator&&" => "op_and".to_string(),
            "operator||" => "op_or".to_string(),
            "operator++" => "op_inc".to_string(),
            "operator--" => "op_dec".to_string(),
            "operator->" => "op_arrow".to_string(),
            "operator->*" => "op_arrow_star".to_string(),
            _ => name.replace("operator", "op_")
        }
    } else {
        name.to_string()
    };

    // Replace invalid characters
    result = result.replace("::", "_")
        .replace('<', "_").replace('>', "_")
        .replace(' ', "").replace('%', "_")
        .replace('=', "_").replace('&', "_")
        .replace('|', "_").replace('!', "_")
        .replace('*', "_").replace('/', "_")
        .replace('+', "_").replace('-', "_")
        .replace('[', "_").replace(']', "_")
        .replace('(', "_").replace(')', "_");

    // Handle keywords
    if RUST_KEYWORDS.contains(&result.as_str()) {
        result = format!("r#{}", result);
    }

    // Handle empty names
    if result.is_empty() {
        result = "_unnamed".to_string();
    }

    result
}

/// Convert a snake_case or lowercase name to PascalCase.
fn to_pascal_case(name: &str) -> String {
    name.split('_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars: Vec<char> = word.chars().collect();
            if let Some(first) = chars.first_mut() {
                *first = first.to_ascii_uppercase();
            }
            chars.into_iter().collect::<String>()
        })
        .collect()
}

/// Convert binary operator to Rust string.
fn binop_to_string(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::And => "&",      // Bitwise AND
        BinaryOp::Or => "|",       // Bitwise OR
        BinaryOp::Xor => "^",      // Bitwise XOR
        BinaryOp::LAnd => "&&",    // Logical AND
        BinaryOp::LOr => "||",     // Logical OR
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::Assign => "=",
        BinaryOp::AddAssign => "+=",
        BinaryOp::SubAssign => "-=",
        BinaryOp::MulAssign => "*=",
        BinaryOp::DivAssign => "/=",
        BinaryOp::RemAssign => "%=",
        BinaryOp::ShlAssign => "<<=",
        BinaryOp::ShrAssign => ">>=",
        BinaryOp::AndAssign => "&=",
        BinaryOp::OrAssign => "|=",
        BinaryOp::XorAssign => "^=",
        BinaryOp::Comma => ",",
        BinaryOp::Spaceship => "cmp",  // Handled specially - placeholder
    }
}

/// Get default value for a type.
fn default_value_for_type(ty: &CppType) -> String {
    match ty {
        CppType::Void => "()".to_string(),
        CppType::Bool => "false".to_string(),
        CppType::Char { .. } | CppType::Short { .. } |
        CppType::Int { .. } | CppType::Long { .. } | CppType::LongLong { .. } => "0".to_string(),
        CppType::Float => "0.0f32".to_string(),
        CppType::Double => "0.0f64".to_string(),
        CppType::Pointer { .. } => "std::ptr::null_mut()".to_string(),
        CppType::Reference { .. } => "std::ptr::null_mut()".to_string(),
        CppType::Named(_) => "Default::default()".to_string(),
        CppType::Array { element, size } => {
            let elem_default = default_value_for_type(element);
            if let Some(n) = size {
                format!("[{}; {}]", elem_default, n)
            } else {
                "Default::default()".to_string()
            }
        }
        _ => "Default::default()".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::SourceLocation;

    fn make_node(kind: ClangNodeKind, children: Vec<ClangNode>) -> ClangNode {
        ClangNode {
            kind,
            children,
            location: SourceLocation::default(),
        }
    }

    #[test]
    fn test_simple_function() {
        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::FunctionDecl {
                    name: "add".to_string(),
                    mangled_name: "_Z3addii".to_string(),
                    return_type: CppType::Int { signed: true },
                    params: vec![
                        ("a".to_string(), CppType::Int { signed: true }),
                        ("b".to_string(), CppType::Int { signed: true }),
                    ],
                    is_definition: true,
                    is_variadic: false,
                    is_noexcept: false,
                    is_coroutine: false,
                    coroutine_info: None,
                },
                vec![make_node(
                    ClangNodeKind::CompoundStmt,
                    vec![make_node(
                        ClangNodeKind::ReturnStmt,
                        vec![make_node(
                            ClangNodeKind::BinaryOperator {
                                op: BinaryOp::Add,
                                ty: CppType::Int { signed: true },
                            },
                            vec![
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "a".to_string(),
                                    ty: CppType::Int { signed: true },
                                    namespace_path: vec![],
                                }, vec![]),
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "b".to_string(),
                                    ty: CppType::Int { signed: true },
                                    namespace_path: vec![],
                                }, vec![]),
                            ],
                        )],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        assert!(code.contains("pub fn add(a: i32, b: i32) -> i32"));
        assert!(code.contains("return a + b"));
    }

    #[test]
    fn test_if_statement() {
        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::FunctionDecl {
                    name: "max".to_string(),
                    mangled_name: "_Z3maxii".to_string(),
                    return_type: CppType::Int { signed: true },
                    params: vec![
                        ("a".to_string(), CppType::Int { signed: true }),
                        ("b".to_string(), CppType::Int { signed: true }),
                    ],
                    is_definition: true,
                    is_variadic: false,
                    is_noexcept: false,
                    is_coroutine: false,
                    coroutine_info: None,
                },
                vec![make_node(
                    ClangNodeKind::CompoundStmt,
                    vec![make_node(
                        ClangNodeKind::IfStmt,
                        vec![
                            // Condition: a > b
                            make_node(ClangNodeKind::BinaryOperator {
                                op: BinaryOp::Gt,
                                ty: CppType::Bool,
                            }, vec![
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "a".to_string(),
                                    ty: CppType::Int { signed: true },
                                    namespace_path: vec![],
                                }, vec![]),
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "b".to_string(),
                                    ty: CppType::Int { signed: true },
                                    namespace_path: vec![],
                                }, vec![]),
                            ]),
                            // Then: return a
                            make_node(ClangNodeKind::ReturnStmt, vec![
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "a".to_string(),
                                    ty: CppType::Int { signed: true },
                                    namespace_path: vec![],
                                }, vec![]),
                            ]),
                            // Else: return b
                            make_node(ClangNodeKind::ReturnStmt, vec![
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "b".to_string(),
                                    ty: CppType::Int { signed: true },
                                    namespace_path: vec![],
                                }, vec![]),
                            ]),
                        ],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        assert!(code.contains("if a > b {"));
        assert!(code.contains("return a"));
        assert!(code.contains("} else {"));
        assert!(code.contains("return b"));
    }

    #[test]
    fn test_async_coroutine_with_task_return() {
        use crate::ast::CoroutineInfo;
        // Test that a coroutine with Task<int> return type generates async fn -> i32
        let coroutine_info = CoroutineInfo {
            kind: CoroutineKind::Async,
            value_type: Some(CppType::Int { signed: true }),
            return_type_spelling: "Task<int>".to_string(),
        };

        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::FunctionDecl {
                    name: "compute".to_string(),
                    mangled_name: "_Z7computev".to_string(),
                    return_type: CppType::Named("Task<int>".to_string()),
                    params: vec![],
                    is_definition: true,
                    is_variadic: false,
                    is_noexcept: false,
                    is_coroutine: true,
                    coroutine_info: Some(coroutine_info),
                },
                vec![make_node(
                    ClangNodeKind::CompoundStmt,
                    vec![make_node(
                        ClangNodeKind::CoreturnStmt { value_ty: Some(CppType::Int { signed: true }) },
                        vec![make_node(
                            ClangNodeKind::IntegerLiteral { value: 42, cpp_type: Some(CppType::Int { signed: true }) },
                            vec![],
                        )],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Should generate async fn with i32 return type (not Task<int>)
        assert!(code.contains("pub async fn compute() -> i32"), "Expected 'pub async fn compute() -> i32', got:\n{}", code);
        // Should have coroutine comment
        assert!(code.contains("/// Coroutine: async (Task<int>)"), "Expected coroutine comment, got:\n{}", code);
    }

    #[test]
    fn test_generator_coroutine_with_value_type() {
        use crate::ast::CoroutineInfo;
        // Test that a generator with Generator<int> return type generates a state machine
        let coroutine_info = CoroutineInfo {
            kind: CoroutineKind::Generator,
            value_type: Some(CppType::Int { signed: true }),
            return_type_spelling: "Generator<int>".to_string(),
        };

        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::FunctionDecl {
                    name: "range".to_string(),
                    mangled_name: "_Z5rangev".to_string(),
                    return_type: CppType::Named("Generator<int>".to_string()),
                    params: vec![],
                    is_definition: true,
                    is_variadic: false,
                    is_noexcept: false,
                    is_coroutine: true,
                    coroutine_info: Some(coroutine_info),
                },
                vec![make_node(
                    ClangNodeKind::CompoundStmt,
                    vec![
                        make_node(
                            ClangNodeKind::CoyieldExpr {
                                value_ty: CppType::Int { signed: true },
                                result_ty: CppType::Void,
                            },
                            vec![make_node(
                                ClangNodeKind::IntegerLiteral { value: 1, cpp_type: Some(CppType::Int { signed: true }) },
                                vec![],
                            )],
                        ),
                        make_node(
                            ClangNodeKind::CoyieldExpr {
                                value_ty: CppType::Int { signed: true },
                                result_ty: CppType::Void,
                            },
                            vec![make_node(
                                ClangNodeKind::IntegerLiteral { value: 2, cpp_type: Some(CppType::Int { signed: true }) },
                                vec![],
                            )],
                        ),
                    ],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Generators should NOT be async
        assert!(!code.contains("async fn range"), "Generator should not be async, got:\n{}", code);
        // Should return impl Iterator<Item=i32>
        assert!(code.contains("impl Iterator<Item=i32>"), "Expected 'impl Iterator<Item=i32>', got:\n{}", code);
        // Should have coroutine comment
        assert!(code.contains("/// Coroutine: generator (Generator<int>)"), "Expected coroutine comment, got:\n{}", code);
        // Should generate state machine struct
        assert!(code.contains("pub struct RangeGenerator"), "Expected 'pub struct RangeGenerator', got:\n{}", code);
        assert!(code.contains("__state: i32"), "Expected '__state: i32' field, got:\n{}", code);
        // Should implement Iterator
        assert!(code.contains("impl Iterator for RangeGenerator"), "Expected Iterator impl, got:\n{}", code);
        assert!(code.contains("type Item = i32"), "Expected 'type Item = i32', got:\n{}", code);
        assert!(code.contains("fn next(&mut self)"), "Expected 'fn next(&mut self)', got:\n{}", code);
        // Should have state machine match arms
        assert!(code.contains("match self.__state"), "Expected match on __state, got:\n{}", code);
        assert!(code.contains("Some(1i32)"), "Expected 'Some(1i32)' for first yield, got:\n{}", code);
        assert!(code.contains("Some(2i32)"), "Expected 'Some(2i32)' for second yield, got:\n{}", code);
        // Function should return generator instance
        assert!(code.contains("RangeGenerator { __state: 0 }"), "Expected generator instance creation, got:\n{}", code);
    }

    #[test]
    fn test_coroutine_without_value_type() {
        use crate::ast::CoroutineInfo;
        // Test a coroutine where we couldn't extract the value type
        let coroutine_info = CoroutineInfo {
            kind: CoroutineKind::Custom,
            value_type: None,
            return_type_spelling: "CustomCoroutine".to_string(),
        };

        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::FunctionDecl {
                    name: "custom".to_string(),
                    mangled_name: "_Z6customv".to_string(),
                    return_type: CppType::Named("CustomCoroutine".to_string()),
                    params: vec![],
                    is_definition: true,
                    is_variadic: false,
                    is_noexcept: false,
                    is_coroutine: true,
                    coroutine_info: Some(coroutine_info),
                },
                vec![make_node(
                    ClangNodeKind::CompoundStmt,
                    vec![],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Should fallback to using the original return type
        assert!(code.contains("CustomCoroutine"), "Expected 'CustomCoroutine' in return type, got:\n{}", code);
        // Should have coroutine comment
        assert!(code.contains("/// Coroutine: custom"), "Expected coroutine comment, got:\n{}", code);
    }

    #[test]
    fn test_non_coroutine_function() {
        // Test that a regular function (not a coroutine) doesn't get async
        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::FunctionDecl {
                    name: "regular".to_string(),
                    mangled_name: "_Z7regularv".to_string(),
                    return_type: CppType::Int { signed: true },
                    params: vec![],
                    is_definition: true,
                    is_variadic: false,
                    is_noexcept: false,
                    is_coroutine: false,
                    coroutine_info: None,
                },
                vec![make_node(
                    ClangNodeKind::CompoundStmt,
                    vec![make_node(
                        ClangNodeKind::ReturnStmt,
                        vec![make_node(
                            ClangNodeKind::IntegerLiteral { value: 0, cpp_type: Some(CppType::Int { signed: true }) },
                            vec![],
                        )],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Should NOT be async
        assert!(!code.contains("async fn regular"), "Regular function should not be async, got:\n{}", code);
        // Should be just a regular pub fn
        assert!(code.contains("pub fn regular() -> i32"), "Expected 'pub fn regular() -> i32', got:\n{}", code);
    }

    #[test]
    fn test_variadic_function() {
        // Test that a variadic function gets extern "C" and ... in signature
        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::FunctionDecl {
                    name: "my_printf".to_string(),
                    mangled_name: "my_printf".to_string(),
                    return_type: CppType::Int { signed: true },
                    params: vec![
                        ("fmt".to_string(), CppType::Pointer { pointee: Box::new(CppType::Char { signed: true }), is_const: true }),
                    ],
                    is_definition: true,
                    is_variadic: true,
                    is_noexcept: false,
                    is_coroutine: false,
                    coroutine_info: None,
                },
                vec![make_node(
                    ClangNodeKind::CompoundStmt,
                    vec![make_node(
                        ClangNodeKind::ReturnStmt,
                        vec![make_node(
                            ClangNodeKind::IntegerLiteral { value: 0, cpp_type: Some(CppType::Int { signed: true }) },
                            vec![],
                        )],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Should have extern "C" and variadic signature
        assert!(code.contains("extern \"C\""), "Variadic function should have extern \"C\", got:\n{}", code);
        assert!(code.contains("..."), "Variadic function should have ... in signature, got:\n{}", code);
        assert!(code.contains("pub extern \"C\" fn my_printf(fmt: *const i8, ...)"),
            "Expected 'pub extern \"C\" fn my_printf(fmt: *const i8, ...)', got:\n{}", code);
    }

    #[test]
    fn test_bit_field_packing() {
        // Test that bit fields are packed into storage units
        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::RecordDecl {
                    name: "Flags".to_string(),
                    is_class: false,
                    fields: vec![],
                },
                vec![
                    // unsigned a : 3;
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "a".to_string(),
                            ty: CppType::Int { signed: false },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: Some(3),
                        },
                        vec![],
                    ),
                    // unsigned b : 5;
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "b".to_string(),
                            ty: CppType::Int { signed: false },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: Some(5),
                        },
                        vec![],
                    ),
                    // unsigned c : 8;
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "c".to_string(),
                            ty: CppType::Int { signed: false },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: Some(8),
                        },
                        vec![],
                    ),
                ],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Total bits = 3 + 5 + 8 = 16, should be packed into u16
        assert!(code.contains("_bitfield_0: u16"), "Expected bit field storage '_bitfield_0: u16', got:\n{}", code);
        // Should NOT have individual fields a, b, c
        assert!(!code.contains("pub a:"), "Should not have individual 'a' field, got:\n{}", code);
        assert!(!code.contains("pub b:"), "Should not have individual 'b' field, got:\n{}", code);
        assert!(!code.contains("pub c:"), "Should not have individual 'c' field, got:\n{}", code);
        // Should have getter/setter for each bit field
        assert!(code.contains("pub fn a(&self)"), "Expected getter 'fn a(&self)', got:\n{}", code);
        assert!(code.contains("pub fn set_a(&mut self"), "Expected setter 'fn set_a(&mut self)', got:\n{}", code);
        assert!(code.contains("pub fn b(&self)"), "Expected getter 'fn b(&self)', got:\n{}", code);
        assert!(code.contains("pub fn set_b(&mut self"), "Expected setter 'fn set_b(&mut self)', got:\n{}", code);
        assert!(code.contains("pub fn c(&self)"), "Expected getter 'fn c(&self)', got:\n{}", code);
        assert!(code.contains("pub fn set_c(&mut self"), "Expected setter 'fn set_c(&mut self)', got:\n{}", code);
    }

    #[test]
    fn test_bit_field_mixed_with_regular() {
        // Test that bit fields work alongside regular fields
        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::RecordDecl {
                    name: "Mixed".to_string(),
                    is_class: false,
                    fields: vec![],
                },
                vec![
                    // int x;
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "x".to_string(),
                            ty: CppType::Int { signed: true },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: None,
                        },
                        vec![],
                    ),
                    // unsigned a : 4;
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "a".to_string(),
                            ty: CppType::Int { signed: false },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: Some(4),
                        },
                        vec![],
                    ),
                    // unsigned b : 4;
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "b".to_string(),
                            ty: CppType::Int { signed: false },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: Some(4),
                        },
                        vec![],
                    ),
                    // int y;
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "y".to_string(),
                            ty: CppType::Int { signed: true },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: None,
                        },
                        vec![],
                    ),
                ],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Bit fields should be packed into u8 (4 + 4 = 8 bits)
        assert!(code.contains("_bitfield_0: u8"), "Expected bit field storage '_bitfield_0: u8', got:\n{}", code);
        // Regular fields should still exist
        assert!(code.contains("pub x: i32"), "Expected regular field 'x: i32', got:\n{}", code);
        assert!(code.contains("pub y: i32"), "Expected regular field 'y: i32', got:\n{}", code);
    }

    #[test]
    fn test_bit_field_multiple_groups() {
        // Test that non-adjacent bit fields create separate groups
        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::RecordDecl {
                    name: "MultiGroup".to_string(),
                    is_class: false,
                    fields: vec![],
                },
                vec![
                    // unsigned a : 3;
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "a".to_string(),
                            ty: CppType::Int { signed: false },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: Some(3),
                        },
                        vec![],
                    ),
                    // int x; (regular field breaks the group)
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "x".to_string(),
                            ty: CppType::Int { signed: true },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: None,
                        },
                        vec![],
                    ),
                    // unsigned b : 5;
                    make_node(
                        ClangNodeKind::FieldDecl {
                            name: "b".to_string(),
                            ty: CppType::Int { signed: false },
                            access: crate::ast::AccessSpecifier::Public,
                            is_static: false,
                            bit_field_width: Some(5),
                        },
                        vec![],
                    ),
                ],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Should have two bit field groups
        assert!(code.contains("_bitfield_0: u8"), "Expected first bit field storage '_bitfield_0: u8', got:\n{}", code);
        assert!(code.contains("_bitfield_1: u8"), "Expected second bit field storage '_bitfield_1: u8', got:\n{}", code);
        assert!(code.contains("pub x: i32"), "Expected regular field 'x: i32', got:\n{}", code);
    }
}
