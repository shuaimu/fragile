//! Direct AST to Rust source code generation.
//!
//! This module generates Rust source code directly from the Clang AST,
//! without going through an intermediate MIR representation.
//! This produces cleaner, more idiomatic Rust code.

use crate::ast::{
    AccessSpecifier, BinaryOp, CastKind, ClangNode, ClangNodeKind, ConstructorKind, CoroutineInfo,
    CoroutineKind, UnaryOp,
};
use crate::types::{parse_template_args, CppType};
use std::collections::{HashMap, HashSet};

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

/// Strip numeric literal suffixes (i32, u64, f32, etc.) from a string.
/// Used when Rust can infer the type from context.
fn strip_literal_suffix(s: &str) -> String {
    // Check for integer suffixes: i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
    // Check for float suffixes: f32, f64
    let suffixes = [
        "i128", "u128", "isize", "usize", // longest first
        "i64", "u64", "i32", "u32", "i16", "u16", "i8", "u8", "f64", "f32",
    ];
    for suffix in suffixes {
        if let Some(prefix) = s.strip_suffix(suffix) {
            // Make sure the prefix is a valid number (not ending with a letter)
            if !prefix.is_empty() && prefix.chars().last().is_some_and(|c| c.is_ascii_digit()) {
                return prefix.to_string();
            }
        }
    }
    s.to_string()
}

/// Check if a string is an integer literal (possibly with suffix).
/// Returns true for: "0", "123", "0i32", "456u64", etc.
fn is_integer_literal_str(s: &str) -> bool {
    let stripped = strip_literal_suffix(s);
    // Check if it's a valid integer (optionally with leading minus)
    let s = stripped.strip_prefix('-').unwrap_or(&stripped);
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Convert an integer literal string to a float literal.
/// E.g., "0" -> "0.0", "123i32" -> "123.0", "-5" -> "-5.0"
fn int_literal_to_float(s: &str) -> String {
    let stripped = strip_literal_suffix(s);
    // If it's already a float literal (contains '.'), return as-is
    if stripped.contains('.') {
        return stripped;
    }
    format!("{}.0", stripped)
}

/// Rust reserved keywords that need raw identifier syntax.
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Sanitize identifier for use in composite names (like function_method).
/// Returns just the sanitized name without the r# prefix.
fn sanitize_identifier_for_composite(name: &str) -> String {
    let result = sanitize_identifier(name);
    // Strip r# prefix if present - composite names like X_vtable_type are already valid
    if result.starts_with("r#") {
        result[2..].to_string()
    } else {
        result
    }
}

/// Information about a virtual method for vtable generation.
/// This represents a single entry in a C++ vtable.
#[derive(Clone, Debug)]
struct VTableEntry {
    /// Method name (e.g., "what", "~exception")
    name: String,
    /// Return type of the method
    return_type: CppType,
    /// Parameters (excluding implicit this/self)
    params: Vec<(String, CppType)>,
    /// True if method is const (affects self mutability)
    is_const: bool,
    /// True if method is pure virtual (= 0)
    is_pure_virtual: bool,
    /// Class where this method was originally declared (for override tracking)
    declaring_class: String,
    /// Index in the vtable (assigned during vtable construction)
    vtable_index: usize,
}

/// Backward compatibility alias for existing code
type VirtualMethodInfo = VTableEntry;

/// Complete vtable information for a polymorphic C++ class.
/// This includes both inherited and declared virtual methods.
#[derive(Clone, Debug)]
struct ClassVTableInfo {
    /// Class name this vtable is for
    class_name: String,
    /// All vtable entries (inherited + declared), in vtable order
    entries: Vec<VTableEntry>,
    /// Direct polymorphic base class name (for single inheritance chain)
    /// For multiple inheritance, see secondary_vtables
    base_class: Option<String>,
    /// True if class is abstract (has any pure virtual methods)
    is_abstract: bool,
    /// Secondary vtables for multiple inheritance (base class -> vtable entries)
    /// These are separate vtables for non-primary polymorphic bases
    #[allow(dead_code)]
    secondary_vtables: Vec<(String, Vec<VTableEntry>)>,
}

#[derive(Clone)]
struct BaseInfo {
    name: String,
    is_virtual: bool,
}

#[derive(Debug)]
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
    /// Diagnostic mode: when enabled, log problematic AST nodes and type conversions
    /// Enable via FRAGILE_DIAGNOSTIC=1 environment variable
    diagnostic_mode: bool,
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
    /// Complete vtable information per polymorphic class
    /// Built during analysis phase, used during code generation
    vtables: HashMap<String, ClassVTableInfo>,
    /// Track which methods are overridden in derived classes
    /// Key: (class_name, method_name), Value: original declaring class
    method_overrides: HashMap<(String, String), String>,
    /// Map from (class_name, member_name) to global variable name for static members
    static_members: HashMap<(String, String), String>,
    /// Track global variable names (require unsafe access)
    global_vars: HashSet<String>,
    /// Map from original variable name to prefixed global variable name
    /// This is needed to resolve DeclRefExpr references to globals with __gv_ prefix
    global_var_mapping: HashMap<String, String>,
    /// Current namespace path during code generation (for relative path computation)
    current_namespace: Vec<String>,
    /// When true, use __self instead of self for this expressions
    use_ctor_self: bool,
    /// Current method return type (for reference return handling)
    current_return_type: Option<CppType>,
    /// Map from class name to its field names (for constructor generation)
    class_fields: HashMap<String, Vec<(String, CppType)>>,
    /// Map from class name to its constructor signatures: class_name -> [(ctor_suffix, param_types)]
    /// e.g., "_Bit_iterator_base" -> [("new_2", [Pointer<u64>, u64])]
    constructor_signatures: HashMap<String, Vec<(String, Vec<CppType>)>>,
    /// Collected std::variant types: maps enum name (e.g., "Variant_i32_f64") to its Rust type arguments (e.g., ["i32", "f64"])
    variant_types: HashMap<String, Vec<String>>,
    /// Counter for generating unique anonymous namespace names
    anon_namespace_counter: usize,
    /// Track already generated struct names to avoid duplicates from template instantiation
    generated_structs: HashSet<String>,
    /// Track already generated type aliases to avoid duplicates
    generated_aliases: HashSet<String>,
    /// Track already generated module names to avoid duplicates (e.g., inline namespaces)
    generated_modules: HashSet<String>,
    /// Map from class/struct name to its bit field groups
    bit_field_groups: HashMap<String, Vec<BitFieldGroup>>,
    /// Track generated function signatures to handle overloads: name -> count
    generated_functions: HashMap<String, usize>,
    /// Track actual Rust module nesting depth (excludes flattened namespaces like std, __)
    module_depth: usize,
    /// Track method names within current struct impl to handle overloads: name -> count
    current_struct_methods: HashMap<String, usize>,
    /// Merged namespace contents: path -> list of child node indices from all occurrences
    /// Used for two-pass namespace merging (C++ can reopen namespaces, Rust cannot)
    merged_namespace_children: HashMap<String, Vec<usize>>,
    /// Reference to the original AST nodes (stored as indices into a collected vec)
    collected_nodes: Vec<ClangNode>,
    /// Template definitions: template name -> (template params, children nodes)
    /// Used to generate structs for template instantiations
    template_definitions: HashMap<String, (Vec<String>, Vec<ClangNode>)>,
    /// Template instantiations that need struct generation: full type name (e.g., "MyVec<int>")
    pending_template_instantiations: HashSet<String>,
    /// Function template definitions: template name -> (template params, return_type, params, body_node)
    fn_template_definitions: HashMap<String, FnTemplateInfo>,
    /// Pending function template instantiations: mangled name (e.g., "add_i32") -> (template_name, type_args)
    pending_fn_instantiations: HashMap<String, (String, Vec<String>)>,
}

/// Information about a function template definition
#[derive(Clone)]
struct FnTemplateInfo {
    /// Template type parameters (e.g., ["T", "U"])
    template_params: Vec<String>,
    /// Return type (may contain template parameter references)
    return_type: CppType,
    /// Parameter names and types
    params: Vec<(String, CppType)>,
    /// The function body (CompoundStmt node), if available
    body: Option<ClangNode>,
    /// Whether the function is noexcept (reserved for future use)
    #[allow(dead_code)]
    is_noexcept: bool,
}

impl AstCodeGen {
    pub fn new() -> Self {
        // Check for diagnostic mode via environment variable
        let diagnostic_mode = std::env::var("FRAGILE_DIAGNOSTIC")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        Self {
            output: String::new(),
            indent: 0,
            diagnostic_mode,
            ref_vars: HashSet::new(),
            ptr_vars: HashSet::new(),
            arr_vars: HashSet::new(),
            skip_literal_suffix: false,
            current_class: None,
            polymorphic_classes: HashSet::new(),
            class_bases: HashMap::new(),
            virtual_bases: HashMap::new(),
            virtual_methods: HashMap::new(),
            vtables: HashMap::new(),
            method_overrides: HashMap::new(),
            static_members: HashMap::new(),
            global_vars: HashSet::new(),
            global_var_mapping: HashMap::new(),
            current_namespace: Vec::new(),
            use_ctor_self: false,
            current_return_type: None,
            class_fields: HashMap::new(),
            constructor_signatures: HashMap::new(),
            variant_types: HashMap::new(),
            anon_namespace_counter: 0,
            generated_structs: HashSet::new(),
            generated_aliases: HashSet::new(),
            generated_modules: HashSet::new(),
            bit_field_groups: HashMap::new(),
            generated_functions: HashMap::new(),
            module_depth: 0,
            current_struct_methods: HashMap::new(),
            merged_namespace_children: HashMap::new(),
            collected_nodes: Vec::new(),
            template_definitions: HashMap::new(),
            pending_template_instantiations: HashSet::new(),
            fn_template_definitions: HashMap::new(),
            pending_fn_instantiations: HashMap::new(),
        }
    }

    /// Log a diagnostic message if diagnostic mode is enabled.
    /// Used for debugging problematic AST nodes and type conversions.
    fn log_diagnostic(&self, category: &str, message: &str) {
        if self.diagnostic_mode {
            eprintln!("[FRAGILE-DIAG] {}: {}", category, message);
        }
    }

    /// Sanitize a return type string, replacing invalid placeholders.
    /// The `_` placeholder is valid in variable types but NOT in function return types.
    fn sanitize_return_type(type_str: &str) -> String {
        // Replace standalone `_` with `()` (unit type) for stub functions
        if type_str == "_" {
            "()".to_string()
        } else {
            type_str.to_string()
        }
    }

    /// Get a default value for a Rust type (used for uninitialized template variables)
    fn get_default_value_for_type(rust_ty: &str) -> String {
        match rust_ty {
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => "0".to_string(),
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => "0".to_string(),
            "f32" => "0.0f32".to_string(),
            "f64" => "0.0".to_string(),
            "bool" => "false".to_string(),
            "char" => "'\\0'".to_string(),
            "()" => "()".to_string(),
            ty if ty.starts_with("*mut ") || ty.starts_with("*const ") => {
                "std::ptr::null_mut()".to_string()
            }
            _ => "Default::default()".to_string(),
        }
    }

    /// Generate Rust source code from a Clang AST.
    pub fn generate(mut self, ast: &ClangNode) -> String {
        // First pass: collect polymorphic class information
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            self.collect_polymorphic_info(&ast.children);
        }
        self.compute_virtual_bases();
        self.build_all_vtables();

        // Collect std::variant types used in the code
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            self.collect_variant_types(&ast.children);
        }

        // Collect all namespace contents (for two-pass namespace merging)
        // C++ allows reopening namespaces; Rust does not. We merge all occurrences.
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            self.collect_namespace_contents(&ast.children, Vec::new());
        }

        // Collect template definitions and instantiations
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            self.collect_template_info(&ast.children);
        }

        // File header
        self.writeln("#![allow(dead_code)]");
        self.writeln("#![allow(unused_variables)]");
        self.writeln("#![allow(unused_mut)]");
        self.writeln("#![allow(non_camel_case_types)]");
        self.writeln("#![allow(non_snake_case)]");
        self.writeln("");
        self.write_array_helpers();

        // Generate comparison category stubs for libstdc++/libc++
        self.generate_comparison_category_stubs();

        // Generate synthetic enum definitions for std::variant types
        self.generate_variant_enums();

        // Generate struct definitions for template instantiations
        self.generate_template_instantiations();

        // Generate function implementations for function template instantiations
        self.generate_fn_template_instantiations();

        // Generate vtable structs for polymorphic classes
        self.generate_all_vtable_structs();

        // Second pass: generate code
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            for child in &ast.children {
                self.generate_top_level(child);
            }
        }

        // Generate static vtable instances (after class definitions)
        self.generate_all_static_vtables();

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
                    name,
                    return_type,
                    params,
                    is_virtual,
                    is_pure_virtual,
                    is_const,
                    ..
                } => {
                    if *is_virtual {
                        virtual_methods.push(VTableEntry {
                            name: name.clone(),
                            return_type: return_type.clone(),
                            params: params.clone(),
                            is_const: *is_const,
                            is_pure_virtual: *is_pure_virtual,
                            declaring_class: class_name.to_string(),
                            vtable_index: virtual_methods.len(), // Will be updated during full vtable construction
                        });
                    }
                }
                ClangNodeKind::CXXBaseSpecifier {
                    base_type,
                    is_virtual,
                    ..
                } => {
                    // Extract base class name - collect ALL bases for MI
                    if let CppType::Named(base_name) = base_type {
                        let base_name = base_name
                            .strip_prefix("const ")
                            .unwrap_or(base_name)
                            .to_string();
                        base_classes.push(BaseInfo {
                            name: base_name,
                            is_virtual: *is_virtual,
                        });
                    }
                }
                _ => {}
            }
        }

        // If this class has virtual methods, mark it as polymorphic
        if !virtual_methods.is_empty() {
            self.polymorphic_classes.insert(class_name.to_string());
            self.virtual_methods
                .insert(class_name.to_string(), virtual_methods);
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
            self.class_bases
                .insert(class_name.to_string(), base_classes);
        }
    }

    /// Build complete vtable information for all polymorphic classes.
    /// Must be called after collect_polymorphic_info() has gathered all class information.
    fn build_all_vtables(&mut self) {
        // Get list of all polymorphic classes
        let class_names: Vec<String> = self.polymorphic_classes.iter().cloned().collect();

        // Build vtable for each class (order doesn't matter due to recursion)
        for class_name in class_names {
            if !self.vtables.contains_key(&class_name) {
                self.build_vtable_for_class(&class_name);
            }
        }
    }

    /// Build vtable for a single class, recursively building base class vtables first.
    /// Returns the ClassVTableInfo for the class (also stored in self.vtables).
    fn build_vtable_for_class(&mut self, class_name: &str) -> ClassVTableInfo {
        // Check if already built (memoization)
        if let Some(info) = self.vtables.get(class_name) {
            return info.clone();
        }

        // Get own virtual methods declared in this class
        let own_methods = self
            .virtual_methods
            .get(class_name)
            .cloned()
            .unwrap_or_default();

        // Get base class info
        let base_info = self.class_bases.get(class_name).cloned();
        let primary_base = base_info.as_ref().and_then(|bases| bases.first());

        // Start with base class vtable entries (if any)
        let mut entries: Vec<VTableEntry> = if let Some(base) = primary_base {
            // Recursively build base vtable
            let base_vtable = self.build_vtable_for_class(&base.name);
            base_vtable.entries.clone()
        } else {
            Vec::new()
        };

        // Merge own methods: override existing or append new
        for own_method in own_methods {
            // Check if this method overrides a base method
            let override_idx = entries.iter().position(|e| {
                e.name == own_method.name && e.params.len() == own_method.params.len()
            });

            if let Some(idx) = override_idx {
                // Record the override: (derived_class, method_name) -> original declaring class
                let original_declaring = entries[idx].declaring_class.clone();
                self.method_overrides.insert(
                    (class_name.to_string(), own_method.name.clone()),
                    original_declaring,
                );

                // Replace entry but preserve vtable_index
                let mut new_entry = own_method.clone();
                new_entry.vtable_index = idx;
                new_entry.declaring_class = class_name.to_string();
                entries[idx] = new_entry;
            } else {
                // New virtual method, append with next index
                let mut new_entry = own_method.clone();
                new_entry.vtable_index = entries.len();
                new_entry.declaring_class = class_name.to_string();
                entries.push(new_entry);
            }
        }

        // Compute is_abstract: true if any entry is pure virtual
        let is_abstract = entries.iter().any(|e| e.is_pure_virtual);

        // Build ClassVTableInfo
        let vtable_info = ClassVTableInfo {
            class_name: class_name.to_string(),
            entries,
            base_class: primary_base.map(|b| b.name.clone()),
            is_abstract,
            secondary_vtables: Vec::new(), // TODO: Handle multiple inheritance in 25.2+
        };

        // Store and return
        self.vtables
            .insert(class_name.to_string(), vtable_info.clone());
        vtable_info
    }

    /// Generate vtable structs for all polymorphic classes.
    fn generate_all_vtable_structs(&mut self) {
        // Only generate vtable for ROOT polymorphic classes (those without polymorphic bases)
        // Derived classes use the base class's vtable type
        let vtable_infos: Vec<_> = self.vtables.values().cloned().collect();
        for vtable_info in vtable_infos {
            // Only generate if this is a root polymorphic class (no polymorphic base)
            if vtable_info.base_class.is_none() {
                self.generate_vtable_struct(&vtable_info.class_name, &vtable_info);
            }
        }
    }

    /// Generate static vtable instances for all concrete (non-abstract) polymorphic classes.
    fn generate_all_static_vtables(&mut self) {
        let vtable_infos: Vec<_> = self.vtables.values().cloned().collect();
        for vtable_info in vtable_infos {
            // Skip abstract classes (have pure virtual methods)
            if vtable_info.is_abstract {
                continue;
            }
            self.generate_static_vtable(&vtable_info);
        }
    }

    /// Generate a static vtable instance for a concrete class.
    fn generate_static_vtable(&mut self, vtable_info: &ClassVTableInfo) {
        let class_name = &vtable_info.class_name;
        let sanitized_class = sanitize_identifier(class_name);

        // Find the root class (the one with the vtable type)
        let root_class = self.find_root_polymorphic_class(class_name);
        let sanitized_root = sanitize_identifier(&root_class);

        // Get inheritance chain for RTTI
        let inheritance_chain = self.get_inheritance_chain(class_name);
        let base_count = inheritance_chain.len();

        // Generate type ID constant
        let type_id = Self::compute_type_id(class_name);
        self.writeln("");
        self.writeln(&format!("/// Type ID for `{}` (FNV-1a hash)", class_name));
        self.writeln(&format!(
            "pub const {}_TYPE_ID: u64 = 0x{:016x};",
            sanitized_class.to_uppercase(),
            type_id
        ));

        // Generate base type IDs array
        self.writeln(&format!(
            "/// Base class type IDs for `{}` (derived to root)",
            class_name
        ));
        let type_ids: Vec<String> = inheritance_chain
            .iter()
            .map(|name| format!("0x{:016x}", Self::compute_type_id(name)))
            .collect();
        self.writeln(&format!(
            "pub static {}_BASE_TYPE_IDS: [u64; {}] = [{}];",
            sanitized_class.to_uppercase(),
            base_count,
            type_ids.join(", ")
        ));

        self.writeln("");
        self.writeln(&format!("/// Static vtable for `{}`", class_name));
        self.writeln(&format!(
            "pub static {}_VTABLE: {}_vtable = {}_vtable {{",
            sanitized_class.to_uppercase(),
            sanitized_root,
            sanitized_root
        ));
        self.indent += 1;

        // RTTI fields
        self.writeln(&format!(
            "__type_id: {}_TYPE_ID,",
            sanitized_class.to_uppercase()
        ));
        self.writeln(&format!("__base_count: {},", base_count));
        self.writeln(&format!(
            "__base_type_ids: &{}_BASE_TYPE_IDS,",
            sanitized_class.to_uppercase()
        ));

        // Track method names to handle overloaded methods consistently
        let mut method_name_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // Generate function pointer for each vtable entry
        for entry in &vtable_info.entries {
            let base_method_name = sanitize_identifier(&entry.name);
            // For composite function names, strip r# prefix
            let base_method_name_for_fn = sanitize_identifier_for_composite(&entry.name);

            // Handle overloaded methods by adding suffix for duplicates
            // Must use same counter logic as generate_vtable_struct and generate_vtable_wrappers
            let count = method_name_counts
                .entry(base_method_name.clone())
                .or_insert(0);
            let (method_name, method_name_for_fn) = if *count == 0 {
                *count += 1;
                (base_method_name, base_method_name_for_fn)
            } else {
                *count += 1;
                (
                    format!("{}_{}", base_method_name, *count - 1),
                    format!("{}_{}", base_method_name_for_fn, *count - 1),
                )
            };

            // Use the declaring class's wrapper for the function pointer.
            // If this class overrides the method, declaring_class == class_name.
            // If inherited without override, declaring_class is the parent that defined it.
            // BUT: if declaring class is abstract, we need to use the current class's wrapper
            // since abstract classes don't generate wrapper functions.
            let wrapper_class = if entry.declaring_class != *class_name {
                // Check if declaring class is abstract
                let declaring_is_abstract = self
                    .vtables
                    .get(&entry.declaring_class)
                    .map(|v| v.is_abstract)
                    .unwrap_or(false);
                if declaring_is_abstract {
                    // Use current class since abstract classes don't have wrappers
                    sanitize_identifier(class_name)
                } else {
                    sanitize_identifier(&entry.declaring_class)
                }
            } else {
                sanitize_identifier(&entry.declaring_class)
            };
            self.writeln(&format!(
                "{}: {}_vtable_{},",
                method_name, wrapper_class, method_name_for_fn
            ));
        }

        // Special handling for exception class stub - it has a 'what' field in the stub vtable
        if class_name == "exception" || class_name == "std::exception" {
            self.writeln("what: exception_vtable_what,");
        }

        // Add destructor
        self.writeln(&format!(
            "__destructor: {}_vtable_destructor,",
            sanitized_class
        ));

        self.indent -= 1;
        self.writeln("};");

        // Generate wrapper functions for this class's vtable
        self.generate_vtable_wrappers(vtable_info);
    }

    /// Generate vtable wrapper functions for a class.
    /// These are unsafe functions that take raw pointers and call the actual methods.
    fn generate_vtable_wrappers(&mut self, vtable_info: &ClassVTableInfo) {
        let class_name = &vtable_info.class_name;
        let sanitized_class = sanitize_identifier(class_name);

        // Find root class for pointer type
        let root_class = self.find_root_polymorphic_class(class_name);
        let sanitized_root = sanitize_identifier(&root_class);

        // Track wrapper function names to handle overloaded methods
        let mut wrapper_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // Generate wrapper for each virtual method that this class declares/overrides
        for entry in &vtable_info.entries {
            // Skip inherited methods that aren't overridden in this class,
            // UNLESS the declaring class is abstract (then we need to generate wrapper here)
            if entry.declaring_class != *class_name {
                // Check if declaring class is abstract
                let declaring_is_abstract = self
                    .vtables
                    .get(&entry.declaring_class)
                    .map(|v| v.is_abstract)
                    .unwrap_or(false);
                if !declaring_is_abstract {
                    // Skip - the non-abstract declaring class will have the wrapper
                    continue;
                }
                // Fall through to generate wrapper for inherited method from abstract class
            }
            // Check if this is an inherited method from an abstract class
            let is_inherited_from_abstract = if entry.declaring_class != *class_name {
                self.vtables
                    .get(&entry.declaring_class)
                    .map(|v| v.is_abstract)
                    .unwrap_or(false)
            } else {
                false
            };

            // For composite function names, use sanitize_identifier_for_composite
            // to avoid r# prefixes in function names like Class_vtable_type
            let base_method_name_for_fn = sanitize_identifier_for_composite(&entry.name);
            let method_name = sanitize_identifier(&entry.name);

            // Handle overloaded methods by adding suffix for duplicates
            let wrapper_key = format!("{}_vtable_{}", sanitized_class, base_method_name_for_fn);
            let count = wrapper_counts.entry(wrapper_key.clone()).or_insert(0);
            let method_name_for_fn = if *count == 0 {
                *count += 1;
                base_method_name_for_fn
            } else {
                *count += 1;
                format!("{}_{}", base_method_name_for_fn, *count - 1)
            };
            let return_type = Self::sanitize_return_type(&entry.return_type.to_rust_type_str());

            // Build parameter list
            let self_ptr = if entry.is_const {
                format!("*const {}", sanitized_root)
            } else {
                format!("*mut {}", sanitized_root)
            };

            let param_decls: Vec<String> = entry
                .params
                .iter()
                .enumerate()
                .map(|(i, (pname, ptype))| {
                    let pname = if pname.is_empty() {
                        format!("arg{}", i)
                    } else {
                        sanitize_identifier(pname)
                    };
                    format!("{}: {}", pname, ptype.to_rust_type_str())
                })
                .collect();

            let param_names: Vec<String> = entry
                .params
                .iter()
                .enumerate()
                .map(|(i, (pname, _))| {
                    if pname.is_empty() {
                        format!("arg{}", i)
                    } else {
                        sanitize_identifier(pname)
                    }
                })
                .collect();

            let all_params = if param_decls.is_empty() {
                format!("this: {}", self_ptr)
            } else {
                format!("this: {}, {}", self_ptr, param_decls.join(", "))
            };

            self.writeln("");
            self.writeln(&format!(
                "/// Vtable wrapper for `{}::{}`",
                class_name, entry.name
            ));

            if return_type == "()" {
                self.writeln(&format!(
                    "unsafe fn {}_vtable_{}({}) {{",
                    sanitized_class, method_name_for_fn, all_params
                ));
            } else {
                self.writeln(&format!(
                    "unsafe fn {}_vtable_{}({}) -> {} {{",
                    sanitized_class, method_name_for_fn, all_params, return_type
                ));
            }

            self.indent += 1;

            // Cast the root pointer to this class's type and call the method
            let args = if param_names.is_empty() {
                String::new()
            } else {
                param_names.join(", ")
            };

            if sanitized_class == sanitized_root {
                // Same class, call directly
                if args.is_empty() {
                    self.writeln(&format!("(*this).{}()", method_name));
                } else {
                    self.writeln(&format!("(*this).{}({})", method_name, args));
                }
            } else {
                // Different class, need to cast through pointer
                // Since derived class embeds base in __base, we cast the pointer
                self.writeln(&format!(
                    "let derived = this as *{} {};",
                    if entry.is_const { "const" } else { "mut" },
                    sanitized_class
                ));
                if is_inherited_from_abstract {
                    // Method is inherited from abstract base - call through __base
                    if args.is_empty() {
                        self.writeln(&format!("(*derived).__base.{}()", method_name));
                    } else {
                        self.writeln(&format!("(*derived).__base.{}({})", method_name, args));
                    }
                } else if args.is_empty() {
                    self.writeln(&format!("(*derived).{}()", method_name));
                } else {
                    self.writeln(&format!("(*derived).{}({})", method_name, args));
                }
            }

            self.indent -= 1;
            self.writeln("}");
        }

        // Special handling for exception class - generate 'what' wrapper
        if class_name == "exception" || class_name == "std::exception" {
            self.writeln("");
            self.writeln("/// Vtable wrapper for `exception::what`");
            self.writeln(&format!(
                "unsafe fn {}_vtable_what(this: *const {}) -> *const i8 {{",
                sanitized_class, sanitized_root
            ));
            self.indent += 1;
            self.writeln("(*this).what()");
            self.indent -= 1;
            self.writeln("}");
        }

        // Generate destructor wrapper
        self.writeln("");
        self.writeln(&format!(
            "/// Vtable destructor wrapper for `{}`",
            class_name
        ));
        self.writeln(&format!(
            "unsafe fn {}_vtable_destructor(this: *mut {}) {{",
            sanitized_class, sanitized_root
        ));
        self.indent += 1;
        if sanitized_class == sanitized_root {
            self.writeln("std::ptr::drop_in_place(this);");
        } else {
            self.writeln(&format!("let derived = this as *mut {};", sanitized_class));
            self.writeln("std::ptr::drop_in_place(derived);");
        }
        self.indent -= 1;
        self.writeln("}");
    }

    /// Find the root polymorphic class in the inheritance chain.
    fn find_root_polymorphic_class(&self, class_name: &str) -> String {
        if let Some(vtable_info) = self.vtables.get(class_name) {
            if let Some(ref base) = vtable_info.base_class {
                // Recursively find root
                self.find_root_polymorphic_class(base)
            } else {
                // This is the root
                class_name.to_string()
            }
        } else {
            class_name.to_string()
        }
    }

    /// Compute the path to access __vtable from a derived class.
    /// For a class like Level2 (which inherits Level1 which inherits Base),
    /// this returns "__base.__base" since the vtable is in Base.
    fn compute_vtable_access_path(&self, class_name: &str) -> String {
        let mut path_parts = Vec::new();
        let mut current = class_name.to_string();

        // Walk up the inheritance chain until we reach the root
        while let Some(vtable_info) = self.vtables.get(&current) {
            if let Some(ref base) = vtable_info.base_class {
                path_parts.push("__base");
                current = base.clone();
            } else {
                // Reached root class
                break;
            }
        }

        if path_parts.is_empty() {
            // This is the root class, no path needed
            String::new()
        } else {
            path_parts.join(".")
        }
    }

    /// Compute a stable type ID hash for a class name.
    /// Uses FNV-1a hash for consistency.
    fn compute_type_id(class_name: &str) -> u64 {
        // FNV-1a hash
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in class_name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Get the inheritance chain for a class (from derived to root base).
    /// Returns list of class names: [self, parent, grandparent, ..., root]
    fn get_inheritance_chain(&self, class_name: &str) -> Vec<String> {
        let mut chain = vec![class_name.to_string()];
        let mut current = class_name.to_string();

        while let Some(vtable_info) = self.vtables.get(&current) {
            if let Some(ref base) = vtable_info.base_class {
                chain.push(base.clone());
                current = base.clone();
            } else {
                break;
            }
        }

        chain
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

    fn collect_virtual_bases(
        &self,
        class_name: &str,
        out: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
    ) {
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
        self.virtual_bases
            .get(class_name)
            .is_some_and(|v| !v.is_empty())
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
                ClangNodeKind::FunctionDecl {
                    return_type,
                    params,
                    ..
                } => {
                    self.collect_variant_from_type(return_type);
                    for (_, param_ty) in params {
                        self.collect_variant_from_type(param_ty);
                    }
                    // Recurse into function body
                    self.collect_variant_types(&child.children);
                }
                ClangNodeKind::CXXMethodDecl {
                    return_type,
                    params,
                    ..
                } => {
                    self.collect_variant_from_type(return_type);
                    for (_, param_ty) in params {
                        self.collect_variant_from_type(param_ty);
                    }
                    // Recurse into method body
                    self.collect_variant_types(&child.children);
                }
                ClangNodeKind::RecordDecl { .. } | ClangNodeKind::NamespaceDecl { .. } => {
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
                        let rust_types: Vec<String> = args
                            .iter()
                            .map(|a| CppType::Named(a.clone()).to_rust_type_str())
                            .collect();

                        // Generate the enum name (same logic as in types.rs)
                        let sanitized_types: Vec<String> = rust_types
                            .iter()
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
                        self.variant_types.entry(enum_name).or_insert(rust_types);
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

    /// Collect all namespace contents for two-pass namespace merging.
    /// C++ allows reopening namespaces (adding items to the same namespace multiple times).
    /// Rust modules cannot be reopened. This pass collects all children from all occurrences
    /// of each namespace so we can generate a single merged module.
    fn collect_namespace_contents(&mut self, children: &[ClangNode], current_path: Vec<String>) {
        for child in children {
            if let ClangNodeKind::NamespaceDecl { name } = &child.kind {
                if let Some(ns_name) = name {
                    // Skip flattened namespaces (std, __-prefixed) but still recurse into them
                    let is_flattened = ns_name.starts_with("__") || ns_name == "std";

                    if is_flattened {
                        // Don't create module for flattened namespaces, just recurse
                        self.collect_namespace_contents(&child.children, current_path.clone());
                    } else {
                        // Build full path for this namespace
                        let mut full_path = current_path.clone();
                        full_path.push(ns_name.clone());
                        let path_key = full_path.join("::");

                        // Store each child node's index for later retrieval
                        for grandchild in &child.children {
                            let idx = self.collected_nodes.len();
                            self.collected_nodes.push(grandchild.clone());
                            self.merged_namespace_children
                                .entry(path_key.clone())
                                .or_default()
                                .push(idx);
                        }

                        // Recurse into nested namespaces
                        self.collect_namespace_contents(&child.children, full_path);
                    }
                } else {
                    // Anonymous namespace - just recurse with same path
                    self.collect_namespace_contents(&child.children, current_path.clone());
                }
            } else {
                // Non-namespace nodes at top level - recurse to find nested namespaces
                self.collect_namespace_contents(&child.children, current_path.clone());
            }
        }
    }

    /// Collect template definitions and find all template instantiation usages.
    /// This enables generating structs for template types like MyVec<int>.
    fn collect_template_info(&mut self, children: &[ClangNode]) {
        for child in children {
            match &child.kind {
                ClangNodeKind::ClassTemplateDecl {
                    name,
                    template_params,
                    ..
                } => {
                    // Store template definition
                    self.template_definitions.insert(
                        name.clone(),
                        (template_params.clone(), child.children.clone()),
                    );
                    // Recurse into template to find usages
                    self.collect_template_info(&child.children);
                }
                ClangNodeKind::FunctionTemplateDecl {
                    name,
                    template_params,
                    return_type,
                    params,
                    is_noexcept,
                    ..
                } => {
                    // Find the function body (CompoundStmt) among children
                    let body = child
                        .children
                        .iter()
                        .find(|c| matches!(c.kind, ClangNodeKind::CompoundStmt))
                        .cloned();

                    // Store function template definition
                    self.fn_template_definitions.insert(
                        name.clone(),
                        FnTemplateInfo {
                            template_params: template_params.clone(),
                            return_type: return_type.clone(),
                            params: params.clone(),
                            body,
                            is_noexcept: *is_noexcept,
                        },
                    );
                    // Recurse into template to find usages
                    self.collect_template_info(&child.children);
                }
                ClangNodeKind::VarDecl { ty, .. } | ClangNodeKind::FieldDecl { ty, .. } => {
                    self.collect_template_type(ty);
                    self.collect_template_info(&child.children);
                }
                ClangNodeKind::FunctionDecl {
                    return_type,
                    params,
                    ..
                } => {
                    self.collect_template_type(return_type);
                    for (_, param_ty) in params {
                        self.collect_template_type(param_ty);
                    }
                    self.collect_template_info(&child.children);
                }
                ClangNodeKind::CXXMethodDecl {
                    return_type,
                    params,
                    ..
                } => {
                    self.collect_template_type(return_type);
                    for (_, param_ty) in params {
                        self.collect_template_type(param_ty);
                    }
                    self.collect_template_info(&child.children);
                }
                ClangNodeKind::CallExpr { .. } => {
                    // Check if this is a call to a function template instantiation
                    // by looking at the callee (first child should be DeclRefExpr or ImplicitCastExpr)
                    self.collect_fn_template_instantiation(child);
                    self.collect_template_info(&child.children);
                }
                ClangNodeKind::RecordDecl { .. }
                | ClangNodeKind::NamespaceDecl { .. }
                | ClangNodeKind::CompoundStmt => {
                    self.collect_template_info(&child.children);
                }
                _ => {
                    self.collect_template_info(&child.children);
                }
            }
        }
    }

    /// Check if a CallExpr is a call to a function template, and if so, collect the instantiation.
    fn collect_fn_template_instantiation(&mut self, call_node: &ClangNode) {
        // The callee is typically the first child, either DeclRefExpr or ImplicitCastExpr->DeclRefExpr
        if call_node.children.is_empty() {
            return;
        }

        // Find the DeclRefExpr - it might be wrapped in ImplicitCastExpr
        let decl_ref =
            if let ClangNodeKind::DeclRefExpr { name, ty, .. } = &call_node.children[0].kind {
                Some((name, ty))
            } else if let ClangNodeKind::ImplicitCastExpr { .. } = &call_node.children[0].kind {
                // Look inside the cast
                call_node.children[0].children.iter().find_map(|c| {
                    if let ClangNodeKind::DeclRefExpr { name, ty, .. } = &c.kind {
                        Some((name, ty))
                    } else {
                        None
                    }
                })
            } else {
                None
            };

        if let Some((fn_name, fn_type)) = decl_ref {
            // Check if this function name corresponds to a function template
            if let Some(template_info) = self.fn_template_definitions.get(fn_name).cloned() {
                // Extract concrete type arguments from the instantiated function type
                if let CppType::Function {
                    return_type,
                    params,
                    ..
                } = fn_type
                {
                    // Build type substitution map by comparing template param patterns with instantiated types
                    // For example, if template has (T* a, T* b) and instantiated is (int*, int*),
                    // we need to extract T = int, not T = int*
                    let type_args: Vec<String> = template_info
                        .template_params
                        .iter()
                        .enumerate()
                        .map(|(i, param_name)| {
                            // Find the template parameter pattern and instantiated type
                            let (template_param_ty, instantiated_ty) =
                                if i < template_info.params.len() && i < params.len() {
                                    (&template_info.params[i].1, &params[i])
                                } else if matches!(
                                    &template_info.return_type,
                                    CppType::TemplateParam { .. }
                                ) {
                                    (&template_info.return_type, return_type.as_ref())
                                } else {
                                    // Fallback: use instantiated param directly
                                    if i < params.len() {
                                        return params[i].to_rust_type_str();
                                    } else {
                                        return return_type.to_rust_type_str();
                                    }
                                };
                            // Extract the template parameter from the pattern
                            extract_template_arg(template_param_ty, instantiated_ty, param_name)
                        })
                        .collect();

                    // Generate a mangled name for the instantiation (e.g., "add_i32")
                    // Sanitize type args for use in function names (replace * with ptr, spaces, etc.)
                    let sanitized_args: Vec<String> = type_args
                        .iter()
                        .map(|a| sanitize_type_for_fn_name(a))
                        .collect();
                    let mangled_name = format!("{}_{}", fn_name, sanitized_args.join("_"));

                    // Store the instantiation if not already present
                    self.pending_fn_instantiations.entry(mangled_name).or_insert_with(|| (fn_name.clone(), type_args));
                }
            }
        }
    }

    /// Check if a type is a template instantiation (e.g., MyVec<int>) and record it.
    fn collect_template_type(&mut self, ty: &CppType) {
        if let CppType::Named(name) = ty {
            // Check if this is a template instantiation (contains <>)
            if name.contains('<') && name.contains('>') {
                // Extract template name (everything before <)
                if let Some(idx) = name.find('<') {
                    let template_name = &name[..idx];
                    // Only add if we have a definition for this template
                    if self.template_definitions.contains_key(template_name) {
                        self.pending_template_instantiations.insert(name.clone());
                    }
                }
            }
        }
        // Also check inside pointer/reference/array types
        match ty {
            CppType::Pointer { pointee, .. } => self.collect_template_type(pointee),
            CppType::Reference { referent, .. } => self.collect_template_type(referent),
            CppType::Array { element, .. } => self.collect_template_type(element),
            _ => {}
        }
    }

    /// Generate struct definitions for pending template instantiations.
    fn generate_template_instantiations(&mut self) {
        let instantiations: Vec<String> = self
            .pending_template_instantiations
            .iter()
            .cloned()
            .collect();
        for inst_name in instantiations {
            // Parse template arguments
            if let Some(open_idx) = inst_name.find('<') {
                let template_name = &inst_name[..open_idx];
                let args_str = &inst_name[open_idx + 1..inst_name.len() - 1]; // Strip < and >
                let type_args = parse_template_args(args_str);

                if let Some((template_params, template_children)) =
                    self.template_definitions.get(template_name).cloned()
                {
                    // Generate struct with substituted types
                    self.generate_template_struct(
                        &inst_name,
                        &template_params,
                        &type_args,
                        &template_children,
                    );
                }
            }
        }
    }

    /// Generate a struct for a template instantiation.
    fn generate_template_struct(
        &mut self,
        inst_name: &str,
        template_params: &[String],
        type_args: &[String],
        children: &[ClangNode],
    ) {
        // Skip template DEFINITIONS that have unresolved type parameters.
        // Only generate structs for actual instantiations with concrete types.
        if inst_name.contains("_Tp")
            || inst_name.contains("_Alloc")
            || inst_name.contains("type-parameter-")
        {
            return;
        }

        // Skip deep STL internal types that cause compilation issues
        // These aren't needed for basic container usage and have complex template dependencies
        if inst_name.contains("__normal_iterator")  // Iterator wrapper with op_index issues
            || inst_name.contains("__wrap_iter")  // Iterator wrapper
            || inst_name.contains("allocator_traits<allocator<void>")  // Returns &c_void.clone()
            || inst_name.contains("allocator_traits<std::allocator<void>")
            || inst_name.contains("numeric_limits<ranges::__detail::")
            || inst_name.contains("hash<float>")
            || inst_name.contains("hash<double>")
            || inst_name.contains("hash<long double>")
            || inst_name.contains("memory_resource")
            || inst_name.contains("__uninitialized_copy")
            || inst_name.contains("_Bit_iterator")  // Bit iterator has op_index returning c_void
            || inst_name.contains("_Bit_const_iterator")
        {
            return;
        }

        // Convert instantiation name to valid Rust identifier
        let rust_name = CppType::Named(inst_name.to_string()).to_rust_type_str();

        // Skip if the rust_name is invalid (contains :: which means it's a qualified type like std::ffi::c_void)
        // These are placeholder types that shouldn't become struct definitions
        if rust_name.contains("::") {
            return;
        }

        // Skip if already generated
        if self.generated_structs.contains(&rust_name) {
            return;
        }
        self.generated_structs.insert(rust_name.clone());

        // Build substitution map: T -> int, etc.
        let mut subst_map = HashMap::new();
        for (param, arg) in template_params.iter().zip(type_args.iter()) {
            subst_map.insert(
                param.clone(),
                CppType::Named(arg.clone()).to_rust_type_str(),
            );
        }

        self.writeln(&format!("/// C++ template instantiation `{}`", inst_name));
        self.writeln("#[repr(C)]");
        self.writeln(&format!("pub struct {} {{", rust_name));
        self.indent += 1;

        // Generate fields with substituted types
        let mut fields = Vec::new();
        for child in children {
            if let ClangNodeKind::FieldDecl {
                name,
                ty,
                access,
                is_static,
                ..
            } = &child.kind
            {
                if *is_static {
                    continue;
                }
                let sanitized_name = if name.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(name)
                };
                // Substitute template parameters in type
                let rust_type = self.substitute_template_type(ty, &subst_map);
                let vis = access_to_visibility(*access);
                self.writeln(&format!("{}{}: {},", vis, sanitized_name, rust_type));
                fields.push((sanitized_name, ty.clone()));
            }
        }

        // Store field info for constructor generation
        self.class_fields.insert(inst_name.to_string(), fields);

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // Generate impl block with methods
        self.generate_template_impl(inst_name, &rust_name, children, &subst_map);
    }

    /// Substitute template parameters in a type.
    fn substitute_template_type(
        &self,
        ty: &CppType,
        subst_map: &HashMap<String, String>,
    ) -> String {
        match ty {
            CppType::TemplateParam { name, .. } => {
                // Template parameter - substitute directly
                if let Some(replacement) = subst_map.get(name) {
                    return replacement.clone();
                }
                // Fallback to the parameter name (shouldn't happen for proper instantiations)
                name.clone()
            }
            CppType::Named(name) => {
                // Check for direct substitution first
                if let Some(replacement) = subst_map.get(name) {
                    return replacement.clone();
                }

                // Handle array-like type names: e.g., "_Tp[_Size]"
                // These come from dependent-sized arrays in template definitions
                if let Some(bracket_idx) = name.find('[') {
                    let element_type = &name[..bracket_idx];
                    let rest = &name[bracket_idx + 1..];
                    if let Some(close_bracket) = rest.find(']') {
                        let size_str = rest[..close_bracket].trim();
                        if !size_str.is_empty() {
                            // Substitute element type
                            let elem_rust = if let Some(repl) = subst_map.get(element_type) {
                                repl.clone()
                            } else {
                                CppType::Named(element_type.to_string()).to_rust_type_str()
                            };

                            // Substitute size (could be a template parameter or numeric)
                            let size_rust = if let Some(repl) = subst_map.get(size_str) {
                                repl.clone()
                            } else if size_str.chars().all(|c| c.is_ascii_digit()) {
                                // Already a numeric size
                                size_str.to_string()
                            } else {
                                // Unknown size parameter - use 0 as fallback
                                // This handles cases like _PaddingSize that aren't substituted
                                "0".to_string()
                            };

                            return format!("[{}; {}]", elem_rust, size_rust);
                        }
                    }
                }

                ty.to_rust_type_str()
            }
            CppType::Pointer { pointee, is_const } => {
                let inner = self.substitute_template_type(pointee, subst_map);
                if *is_const {
                    format!("*const {}", inner)
                } else {
                    format!("*mut {}", inner)
                }
            }
            CppType::Reference {
                referent, is_const, ..
            } => {
                // Convert references to raw pointers for struct fields
                // (Rust struct fields can't have references without lifetime parameters)
                let inner = self.substitute_template_type(referent, subst_map);
                if *is_const {
                    format!("*const {}", inner)
                } else {
                    format!("*mut {}", inner)
                }
            }
            CppType::Array { element, size } => {
                let inner = self.substitute_template_type(element, subst_map);
                match size {
                    Some(n) => format!("[{}; {}]", inner, n),
                    None => format!("*mut {}", inner),
                }
            }
            _ => ty.to_rust_type_str(),
        }
    }

    /// Generate impl block for a template instantiation.
    fn generate_template_impl(
        &mut self,
        _inst_name: &str,
        rust_name: &str,
        children: &[ClangNode],
        subst_map: &HashMap<String, String>,
    ) {
        let mut has_methods = false;
        for child in children {
            if matches!(
                &child.kind,
                ClangNodeKind::CXXMethodDecl {
                    is_definition: true,
                    ..
                }
            ) {
                has_methods = true;
                break;
            }
        }

        if !has_methods {
            return;
        }

        self.writeln(&format!("impl {} {{", rust_name));
        self.indent += 1;

        // Track method names within this impl block to handle overloads
        let mut method_counts: HashMap<String, usize> = HashMap::new();

        for child in children {
            if let ClangNodeKind::CXXMethodDecl {
                name,
                return_type,
                params,
                is_definition,
                is_static,
                ..
            } = &child.kind
            {
                if *is_definition {
                    // Generate method with substituted types
                    let ret_type = self.substitute_template_type(return_type, subst_map);
                    let mut param_strs = Vec::new();

                    // Add self parameter for non-static methods
                    if !*is_static {
                        param_strs.push("&mut self".to_string());
                    }

                    // Deduplicate parameter names (C++ allows unnamed params, Rust doesn't)
                    let mut param_name_counts: HashMap<String, usize> = HashMap::new();
                    for (param_name, param_ty) in params {
                        let rust_ty = self.substitute_template_type(param_ty, subst_map);
                        let mut pname = sanitize_identifier(param_name);
                        let count = param_name_counts.entry(pname.clone()).or_insert(0);
                        if *count > 0 {
                            pname = format!("{}_{}", pname, *count);
                        }
                        *param_name_counts
                            .get_mut(&sanitize_identifier(param_name))
                            .unwrap() += 1;
                        param_strs.push(format!("{}: {}", pname, rust_ty));
                    }

                    let ret_str = if ret_type == "()" || ret_type.is_empty() || ret_type == "_" {
                        String::new()
                    } else {
                        format!(" -> {}", Self::sanitize_return_type(&ret_type))
                    };

                    // Handle method overloading by appending suffix for duplicates
                    let base_method_name = sanitize_identifier(name);
                    let count = method_counts.entry(base_method_name.clone()).or_insert(0);
                    let method_name = if *count == 0 {
                        *count += 1;
                        base_method_name
                    } else {
                        *count += 1;
                        format!("{}_{}", base_method_name, *count - 1)
                    };

                    self.writeln(&format!(
                        "pub fn {}({}){} {{",
                        method_name,
                        param_strs.join(", "),
                        ret_str
                    ));
                    self.indent += 1;
                    self.writeln("todo!(\"Template method body\")");
                    self.indent -= 1;
                    self.writeln("}");
                    self.writeln("");
                }
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate function implementations for pending function template instantiations.
    fn generate_fn_template_instantiations(&mut self) {
        // Clone the pending instantiations to avoid borrow issues
        let instantiations: Vec<(String, (String, Vec<String>))> = self
            .pending_fn_instantiations
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (mangled_name, (template_name, type_args)) in instantiations {
            if let Some(template_info) = self.fn_template_definitions.get(&template_name).cloned() {
                self.generate_fn_template_instance(
                    &mangled_name,
                    &template_name,
                    &type_args,
                    &template_info,
                );
            }
        }
    }

    /// Generate a concrete function for a function template instantiation.
    fn generate_fn_template_instance(
        &mut self,
        mangled_name: &str,
        template_name: &str,
        type_args: &[String],
        template_info: &FnTemplateInfo,
    ) {
        // Build substitution map: T -> i32, etc.
        let mut subst_map = HashMap::new();
        for (param, arg) in template_info.template_params.iter().zip(type_args.iter()) {
            subst_map.insert(param.clone(), arg.clone());
        }

        // Substitute types in return type and parameters
        let ret_type = self.substitute_template_type(&template_info.return_type, &subst_map);

        // Skip functions with variadic template parameters (C++ parameter packs)
        // These contain patterns like `_Tp &&...` or `_Args...` which can't be expressed in Rust
        // Also skip functions with unresolved template parameters or C-style function pointer syntax
        for (_, param_ty) in &template_info.params {
            let param_str = self.substitute_template_type(param_ty, &subst_map);
            if param_str.contains("&&...")
                || param_str.contains("...")
                || param_str.contains("_Tp")
                || param_str.contains("_Args")
                || param_str.contains("type_parameter_")
                || param_str.contains("(*)")
                || param_str.contains("_CharT")  // Skip unresolved template params
                || param_str.contains("__va_list_tag")  // Skip variadic internal types
                || param_str.contains("int (")  // Skip C-style function pointer: int (*)(...)
                || param_str.contains("void (")  // Skip C-style function pointer: void (*)(...)
                || param_str.contains("T[")  // Skip unresolved template array param like T[N]
                || param_str.contains(" N]")  // Skip unresolved array size
            {
                // C-style function pointer syntax like void (*)(void *) can't be parsed by Rust
                return;
            }
        }

        // Skip functions with decltype return types or unresolved template parameters
        if ret_type.contains("decltype")
            || ret_type.contains("_Tp")
            || ret_type.contains("_Args")
            || ret_type.contains("type_parameter_")
            || ret_type.contains("(*)")
            || ret_type.contains("_CharT")
            || ret_type.contains("__va_list_tag")
            || ret_type.contains("__gnu_cxx::")  // Skip GCC extension types
            || ret_type.contains("__enable_if")  // Skip SFINAE return types
            || ret_type.contains("typename ")  // Skip C++ dependent types with typename keyword
        {
            return;
        }
        let ret_str = if ret_type == "()" || ret_type.is_empty() || ret_type == "_" {
            String::new()
        } else {
            format!(" -> {}", Self::sanitize_return_type(&ret_type))
        };

        // Generate parameter list
        let mut param_strs = Vec::new();
        let mut param_name_counts: HashMap<String, usize> = HashMap::new();
        for (param_name, param_ty) in &template_info.params {
            let rust_ty = self.substitute_template_type(param_ty, &subst_map);
            let mut pname = sanitize_identifier(param_name);
            if pname.is_empty() {
                pname = format!("_arg{}", param_strs.len());
            }
            let count = param_name_counts.entry(pname.clone()).or_insert(0);
            if *count > 0 {
                pname = format!("{}_{}", pname, *count);
            }
            *param_name_counts
                .get_mut(&sanitize_identifier(param_name))
                .unwrap_or(&mut 0) += 1;
            param_strs.push(format!("{}: {}", pname, rust_ty));
        }

        // Sanitize the mangled name - it may contain `extern "C"` and other invalid characters
        let sanitized_mangled_name = sanitize_identifier(mangled_name);

        // Save output position so we can rollback if the function contains broken patterns
        let output_start = self.output.len();

        self.writeln(&format!(
            "/// Function template instantiation: {}",
            template_name
        ));
        self.writeln(&format!(
            "/// Instantiated with: [{}]",
            type_args.join(", ")
        ));
        self.writeln(&"#[inline]".to_string());
        self.writeln(&format!(
            "pub fn {}({}){} {{",
            sanitized_mangled_name,
            param_strs.join(", "),
            ret_str
        ));
        self.indent += 1;

        // Generate body by processing the template body with type substitutions
        if let Some(ref body) = template_info.body {
            // Save current state
            let saved_ref_vars = self.ref_vars.clone();
            let saved_ptr_vars = self.ptr_vars.clone();
            let saved_arr_vars = self.arr_vars.clone();

            // Clear for this function
            self.ref_vars.clear();
            self.ptr_vars.clear();
            self.arr_vars.clear();

            // Track reference parameters - they are converted to pointers in Rust,
            // so accesses need to be dereferenced (handled by ref_vars tracking)
            for (param_name, param_ty) in &template_info.params {
                if matches!(param_ty, CppType::Reference { .. }) {
                    self.ref_vars.insert(param_name.clone());
                }
            }

            // Generate the body statements with type substitution
            self.generate_fn_template_body(body, &subst_map);

            // Restore state
            self.ref_vars = saved_ref_vars;
            self.ptr_vars = saved_ptr_vars;
            self.arr_vars = saved_arr_vars;
        } else {
            self.writeln("todo!(\"Function template body not available\")");
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // Check if generated function contains broken patterns that can't compile
        // _dependent_type::new_N() calls are template-dependent constructors that aren't resolved
        let generated = &self.output[output_start..];
        if generated.contains("_dependent_type::new_")
            || generated.contains("_unnamed)")  // Unresolved value in function call
            || generated.contains("_unnamed,")  // Unresolved value in function call
            || generated.contains("-> std::ffi::c_void")  // Returns void type (placeholder)
            || generated.contains(": std::ffi::c_void)")  // Parameter is c_void placeholder
        {
            // Rollback - remove the generated function
            self.output.truncate(output_start);
        }
    }

    /// Generate the body of a function template instantiation with type substitution.
    fn generate_fn_template_body(&mut self, body: &ClangNode, subst_map: &HashMap<String, String>) {
        // For now, generate the body using expr_to_string and stmt generation
        // with type names substituted in the output
        if let ClangNodeKind::CompoundStmt = &body.kind {
            for stmt in &body.children {
                self.generate_fn_template_stmt(stmt, subst_map);
            }
        }
    }

    /// Generate a statement in a function template body with type substitution.
    fn generate_fn_template_stmt(&mut self, node: &ClangNode, subst_map: &HashMap<String, String>) {
        match &node.kind {
            ClangNodeKind::ReturnStmt => {
                if !node.children.is_empty() {
                    let expr = self.expr_to_string(&node.children[0]);
                    // Substitute template types in the expression
                    let expr = self.substitute_type_in_expr(&expr, subst_map);
                    self.writeln(&format!("return {};", expr));
                } else {
                    self.writeln("return;");
                }
            }
            ClangNodeKind::DeclStmt => {
                // Handle variable declarations
                for child in &node.children {
                    if let ClangNodeKind::VarDecl { name, ty, .. } = &child.kind {
                        let rust_ty = self.substitute_template_type(ty, subst_map);
                        let var_name = sanitize_identifier(name);

                        // Check if this is an array type
                        let is_array = rust_ty.starts_with('[') && rust_ty.contains(';');

                        // Find the initializer expression (skip TypeRef nodes)
                        // For arrays, skip IntegerLiteral which is the array size, not initializer
                        let init_expr = if is_array {
                            // For arrays, look specifically for InitListExpr first
                            child.children.iter().find(|c| {
                                matches!(&c.kind, ClangNodeKind::InitListExpr { .. })
                            }).or_else(|| {
                                // Fall back to other expressions (skip array size)
                                child.children.iter().find(|c| {
                                    !matches!(
                                        &c.kind,
                                        ClangNodeKind::Unknown(s) if s.starts_with("TypeRef") || s.starts_with("TemplateRef")
                                    ) && !matches!(
                                        &c.kind,
                                        ClangNodeKind::TemplateTypeParmDecl { .. }
                                    ) && !matches!(
                                        &c.kind,
                                        ClangNodeKind::IntegerLiteral { .. }
                                    )
                                })
                            })
                        } else {
                            child.children.iter().find(|c| {
                                !matches!(
                                    &c.kind,
                                    ClangNodeKind::Unknown(s) if s.starts_with("TypeRef") || s.starts_with("TemplateRef")
                                ) && !matches!(
                                    &c.kind,
                                    ClangNodeKind::TemplateTypeParmDecl { .. }
                                )
                            })
                        };
                        if let Some(init_node) = init_expr {
                            let init = self.expr_to_string(init_node);
                            let init = self.substitute_type_in_expr(&init, subst_map);
                            // Wrap in unsafe if the initializer dereferences a pointer
                            let init = if Self::needs_unsafe_wrapper(&init) {
                                format!("unsafe {{ {} }}", init)
                            } else {
                                init
                            };
                            self.writeln(&format!("let mut {}: {} = {};", var_name, rust_ty, init));
                        } else {
                            // No initializer, need a default value
                            let default_val = Self::get_default_value_for_type(&rust_ty);
                            self.writeln(&format!(
                                "let mut {}: {} = {};",
                                var_name, rust_ty, default_val
                            ));
                        }
                    }
                }
            }
            ClangNodeKind::CompoundStmt => {
                self.writeln("{");
                self.indent += 1;
                for child in &node.children {
                    self.generate_fn_template_stmt(child, subst_map);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            _ => {
                // Default: generate as expression statement
                let expr = self.expr_to_string(node);
                let expr = self.substitute_type_in_expr(&expr, subst_map);
                if !expr.is_empty() && expr != "()" {
                    // Wrap in unsafe if the expression dereferences a pointer
                    if Self::needs_unsafe_wrapper(&expr) {
                        self.writeln(&format!("unsafe {{ {} }};", expr));
                    } else {
                        // If expression contains `unsafe { ... }` followed by a comparison operator,
                        // Rust requires parentheses. E.g., `unsafe { X } > Y;` is invalid,
                        // but `(unsafe { X } > Y);` is valid (though typically unused).
                        // This can happen with static assertions or debug comparisons.
                        let needs_parens = expr.contains("unsafe {")
                            && (expr.contains("} >")
                                || expr.contains("} <")
                                || expr.contains("} ==")
                                || expr.contains("} !=")
                                || expr.contains("} >=")
                                || expr.contains("} <="));
                        if needs_parens {
                            self.writeln(&format!("({});", expr));
                        } else {
                            self.writeln(&format!("{};", expr));
                        }
                    }
                }
            }
        }
    }

    /// Check if an expression needs to be wrapped in an unsafe block.
    /// This is true if the expression contains a raw pointer dereference that isn't already unsafe.
    fn needs_unsafe_wrapper(expr: &str) -> bool {
        // If it already starts with "unsafe {", no need to wrap
        if expr.trim_start().starts_with("unsafe {") {
            return false;
        }
        // Check for dereference patterns: *varname (not in string literals)
        // Simple heuristic: contains '*' followed by an identifier char, and not inside quotes
        let bytes = expr.as_bytes();
        let mut in_string = false;
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'"' || bytes[i] == b'\'' {
                in_string = !in_string;
            } else if !in_string && bytes[i] == b'*' && i + 1 < bytes.len() {
                let next = bytes[i + 1];
                // Check if this looks like a pointer dereference (followed by identifier)
                if next.is_ascii_alphabetic() || next == b'_' {
                    return true;
                }
            }
            i += 1;
        }
        false
    }

    /// Substitute template type names in an expression string.
    fn substitute_type_in_expr(&self, expr: &str, subst_map: &HashMap<String, String>) -> String {
        let mut result = expr.to_string();
        for (from, to) in subst_map {
            // Replace type parameter references (be careful about word boundaries)
            result = result.replace(&format!("::{}", from), &format!("::{}", to));
            result = result.replace(&format!("<{}>", from), &format!("<{}>", to));
            result = result.replace(&format!("{} ", from), &format!("{} ", to));
        }
        result
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
            // Note: C's memcpy/memmove/memset return the destination pointer
            "__builtin_memcpy" => {
                // __builtin_memcpy(dst, src, n) -> { copy_nonoverlapping(src, dst, n); dst }
                if args.len() >= 3 {
                    // Note: memcpy copies n bytes, copy_nonoverlapping copies n elements
                    // We cast to u8 pointers to copy bytes, and count to usize
                    Some((
                        format!(
                            "{{ let __dst = {}; std::ptr::copy_nonoverlapping({} as *const u8, __dst as *mut u8, ({}) as usize); __dst }}",
                            args[0], args[1], args[2]
                        ),
                        true,
                    ))
                } else {
                    None
                }
            }
            "__builtin_memmove" => {
                // __builtin_memmove(dst, src, n) -> { copy(src, dst, n); dst }
                if args.len() >= 3 {
                    Some((
                        format!(
                            "{{ let __dst = {}; std::ptr::copy({} as *const u8, __dst as *mut u8, ({}) as usize); __dst }}",
                            args[0], args[1], args[2]
                        ),
                        true,
                    ))
                } else {
                    None
                }
            }
            "__builtin_memset" => {
                // __builtin_memset(dst, val, n) -> { write_bytes(dst, val, n); dst }
                if args.len() >= 3 {
                    Some((
                        format!(
                            "{{ let __dst = {}; std::ptr::write_bytes(__dst as *mut u8, ({}) as u8, ({}) as usize); __dst }}",
                            args[0], args[1], args[2]
                        ),
                        true,
                    ))
                } else {
                    None
                }
            }
            "__builtin_memcmp" => {
                // __builtin_memcmp(s1, s2, n) -> compare n bytes
                // Rust doesn't have a direct equivalent, use libc or slice comparison
                if args.len() >= 3 {
                    Some((
                        format!(
                            "{{ let s1 = std::slice::from_raw_parts({} as *const u8, ({}) as usize); \
                         let s2 = std::slice::from_raw_parts({} as *const u8, ({}) as usize); \
                         s1.cmp(s2) as i32 }}",
                            args[0], args[2], args[1], args[2]
                        ),
                        true,
                    ))
                } else {
                    None
                }
            }
            "__builtin_strlen" => {
                // __builtin_strlen(s) -> strlen equivalent (returns u64 for size_t)
                if !args.is_empty() {
                    Some((
                        format!(
                            "{{ let mut __len = 0u64; let mut __p = {} as *const u8; \
                         while *__p != 0 {{ __len += 1; __p = __p.add(1); }} __len }}",
                            args[0]
                        ),
                        true,
                    ))
                } else {
                    None
                }
            }
            "__builtin_expect" => {
                // __builtin_expect(exp, c) -> exp (hint for branch prediction, just return exp)
                if !args.is_empty() {
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
            "__builtin_abort" => Some(("std::process::abort()".to_string(), false)),
            "__builtin_clz" | "__builtin_clzl" | "__builtin_clzll" => {
                // Count leading zeros
                if !args.is_empty() {
                    Some((format!("({}).leading_zeros() as i32", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_ctz" | "__builtin_ctzl" | "__builtin_ctzll" => {
                // Count trailing zeros
                if !args.is_empty() {
                    Some((format!("({}).trailing_zeros() as i32", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_popcount" | "__builtin_popcountl" | "__builtin_popcountll" => {
                // Population count (number of 1 bits)
                if !args.is_empty() {
                    Some((format!("({}).count_ones() as i32", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_bswap16" => {
                if !args.is_empty() {
                    Some((format!("({}).swap_bytes()", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_bswap32" => {
                if !args.is_empty() {
                    Some((format!("({}).swap_bytes()", args[0]), false))
                } else {
                    None
                }
            }
            "__builtin_bswap64" => {
                if !args.is_empty() {
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
                Some((
                    "{ /* va_start: va_list already initialized */ }".to_string(),
                    false,
                ))
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
            "__builtin_strcmp" => {
                // __builtin_strcmp(s1, s2) -> compare C strings
                // Returns negative if s1 < s2, positive if s1 > s2, 0 if equal
                if args.len() >= 2 {
                    Some((
                        format!(
                            "{{ let mut __p1 = {} as *const u8; let mut __p2 = {} as *const u8; \
                         loop {{ let c1 = *__p1; let c2 = *__p2; \
                         if c1 != c2 {{ break (c1 as i32) - (c2 as i32); }} \
                         if c1 == 0 {{ break 0; }} \
                         __p1 = __p1.add(1); __p2 = __p2.add(1); }} }}",
                            args[0], args[1]
                        ),
                        true,
                    ))
                } else {
                    None
                }
            }
            // libc++ RTTI helper functions
            "__type_name_to_string" | "__string_to_type_name" => {
                // These convert between type_info and string representations
                // Return a placeholder (empty string or dummy pointer)
                if !args.is_empty() {
                    Some(("b\"\\0\".as_ptr() as *const i8".to_string(), false))
                } else {
                    Some(("b\"\\0\".as_ptr() as *const i8".to_string(), false))
                }
            }
            "__is_type_name_unique" => {
                // Returns true if the type name is unique (no duplicates in the program)
                // For simplicity, always return true
                Some(("true".to_string(), false))
            }
            "__libcpp_is_constant_evaluated" => {
                // Like __builtin_is_constant_evaluated but libc++ specific
                Some(("false".to_string(), false))
            }
            // Hash and comparison functions for libc++ internals
            "__hash" => {
                // Generic hash function - return a placeholder hash
                if !args.is_empty() {
                    Some((
                        format!("({} as usize).wrapping_mul(0x9e3779b9)", args[0]),
                        false,
                    ))
                } else {
                    Some(("0usize".to_string(), false))
                }
            }
            "__eq" | "__lt" => {
                // Comparison functions for type_info
                if args.len() >= 2 {
                    let op = if func_name == "__eq" { "==" } else { "<" };
                    Some((format!("({}) {} ({})", args[0], op, args[1]), false))
                } else {
                    Some(("false".to_string(), false))
                }
            }
            "__builtin_addressof" => {
                // __builtin_addressof(expr) -> &raw const expr (address of expr)
                // Special case: if the argument is a dereference (*ptr), just return ptr
                if args.len() == 1 {
                    let arg = args[0].trim();
                    if arg.starts_with('*') {
                        // *ptr -> ptr (address of dereference is the original pointer)
                        let ptr_expr = arg[1..].trim();
                        Some((format!("{} as *const _", ptr_expr), false))
                    } else if arg.starts_with("unsafe { *") && arg.ends_with('}') {
                        // unsafe { *ptr } -> ptr
                        let inner = arg
                            .strip_prefix("unsafe { *")
                            .and_then(|s| s.strip_suffix('}'))
                            .map(|s| s.trim());
                        if let Some(ptr_expr) = inner {
                            Some((format!("{} as *const _", ptr_expr), false))
                        } else {
                            // Fallback: take address with addr_of!
                            Some((format!("std::ptr::addr_of!({}) as *const _", arg), false))
                        }
                    } else {
                        // Regular case: take address of expression
                        Some((format!("&{} as *const _", arg), false))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Map C library function names to their fragile-runtime equivalents.
    /// Returns the renamed function name if the function should be remapped.
    ///
    /// When transpiling libc++ code, it calls standard C library functions
    /// (pthread_create, fopen, etc.). We redirect these to our fragile-runtime
    /// implementations which are prefixed with `fragile_`.
    fn map_runtime_function_name(func_name: &str) -> Option<&'static str> {
        match func_name {
            // pthread functions
            "pthread_create" => Some("crate::fragile_runtime::fragile_pthread_create"),
            "pthread_join" => Some("crate::fragile_runtime::fragile_pthread_join"),
            "pthread_self" => Some("crate::fragile_runtime::fragile_pthread_self"),
            "pthread_equal" => Some("crate::fragile_runtime::fragile_pthread_equal"),
            "pthread_detach" => Some("crate::fragile_runtime::fragile_pthread_detach"),
            "pthread_exit" => Some("crate::fragile_runtime::fragile_pthread_exit"),
            "pthread_attr_init" => Some("crate::fragile_runtime::fragile_pthread_attr_init"),
            "pthread_attr_destroy" => Some("crate::fragile_runtime::fragile_pthread_attr_destroy"),
            "pthread_attr_setdetachstate" => {
                Some("crate::fragile_runtime::fragile_pthread_attr_setdetachstate")
            }
            "pthread_attr_getdetachstate" => {
                Some("crate::fragile_runtime::fragile_pthread_attr_getdetachstate")
            }

            // pthread mutex functions
            "pthread_mutex_init" => Some("crate::fragile_runtime::fragile_pthread_mutex_init"),
            "pthread_mutex_destroy" => {
                Some("crate::fragile_runtime::fragile_pthread_mutex_destroy")
            }
            "pthread_mutex_lock" => Some("crate::fragile_runtime::fragile_pthread_mutex_lock"),
            "pthread_mutex_trylock" => {
                Some("crate::fragile_runtime::fragile_pthread_mutex_trylock")
            }
            "pthread_mutex_unlock" => Some("crate::fragile_runtime::fragile_pthread_mutex_unlock"),
            "pthread_mutexattr_init" => {
                Some("crate::fragile_runtime::fragile_pthread_mutexattr_init")
            }
            "pthread_mutexattr_destroy" => {
                Some("crate::fragile_runtime::fragile_pthread_mutexattr_destroy")
            }
            "pthread_mutexattr_settype" => {
                Some("crate::fragile_runtime::fragile_pthread_mutexattr_settype")
            }
            "pthread_mutexattr_gettype" => {
                Some("crate::fragile_runtime::fragile_pthread_mutexattr_gettype")
            }

            // pthread condition variable functions
            "pthread_cond_init" => Some("crate::fragile_runtime::fragile_pthread_cond_init"),
            "pthread_cond_destroy" => Some("crate::fragile_runtime::fragile_pthread_cond_destroy"),
            "pthread_cond_wait" => Some("crate::fragile_runtime::fragile_pthread_cond_wait"),
            "pthread_cond_timedwait" => {
                Some("crate::fragile_runtime::fragile_pthread_cond_timedwait")
            }
            "pthread_cond_signal" => Some("crate::fragile_runtime::fragile_pthread_cond_signal"),
            "pthread_cond_broadcast" => {
                Some("crate::fragile_runtime::fragile_pthread_cond_broadcast")
            }
            "pthread_condattr_init" => {
                Some("crate::fragile_runtime::fragile_pthread_condattr_init")
            }
            "pthread_condattr_destroy" => {
                Some("crate::fragile_runtime::fragile_pthread_condattr_destroy")
            }

            // pthread rwlock functions
            "pthread_rwlock_init" => Some("crate::fragile_runtime::fragile_pthread_rwlock_init"),
            "pthread_rwlock_destroy" => {
                Some("crate::fragile_runtime::fragile_pthread_rwlock_destroy")
            }
            "pthread_rwlock_rdlock" => {
                Some("crate::fragile_runtime::fragile_pthread_rwlock_rdlock")
            }
            "pthread_rwlock_tryrdlock" => {
                Some("crate::fragile_runtime::fragile_pthread_rwlock_tryrdlock")
            }
            "pthread_rwlock_wrlock" => {
                Some("crate::fragile_runtime::fragile_pthread_rwlock_wrlock")
            }
            "pthread_rwlock_trywrlock" => {
                Some("crate::fragile_runtime::fragile_pthread_rwlock_trywrlock")
            }
            "pthread_rwlock_unlock" => {
                Some("crate::fragile_runtime::fragile_pthread_rwlock_unlock")
            }
            "pthread_rwlockattr_init" => {
                Some("crate::fragile_runtime::fragile_pthread_rwlockattr_init")
            }
            "pthread_rwlockattr_destroy" => {
                Some("crate::fragile_runtime::fragile_pthread_rwlockattr_destroy")
            }

            // stdio functions
            "fopen" => Some("crate::fragile_runtime::fopen"),
            "fclose" => Some("crate::fragile_runtime::fclose"),
            "fread" => Some("crate::fragile_runtime::fread"),
            "fwrite" => Some("crate::fragile_runtime::fwrite"),
            "fseek" => Some("crate::fragile_runtime::fseek"),
            "fseeko" => Some("crate::fragile_runtime::fseeko"),
            "ftell" => Some("crate::fragile_runtime::ftell"),
            "ftello" => Some("crate::fragile_runtime::ftello"),
            "fflush" => Some("crate::fragile_runtime::fflush"),
            "feof" => Some("crate::fragile_runtime::feof"),
            "ferror" => Some("crate::fragile_runtime::ferror"),
            "clearerr" => Some("crate::fragile_runtime::clearerr"),
            "fileno" => Some("crate::fragile_runtime::fileno"),
            "fgetc" => Some("crate::fragile_runtime::fgetc"),
            "getc" => Some("crate::fragile_runtime::getc"),
            "getchar" => Some("crate::fragile_runtime::getchar"),
            "fputc" => Some("crate::fragile_runtime::fputc"),
            "putc" => Some("crate::fragile_runtime::putc"),
            "putchar" => Some("crate::fragile_runtime::putchar"),
            "ungetc" => Some("crate::fragile_runtime::ungetc"),
            "fputs" => Some("crate::fragile_runtime::fputs"),
            "puts" => Some("crate::fragile_runtime::puts"),
            "fgets" => Some("crate::fragile_runtime::fgets"),

            // C memory functions (used by libc++ allocator)
            "malloc" => Some("crate::fragile_runtime::fragile_malloc"),
            "free" => Some("crate::fragile_runtime::fragile_free"),
            "realloc" => Some("crate::fragile_runtime::fragile_realloc"),
            "calloc" => Some("crate::fragile_runtime::fragile_calloc"),

            _ => None,
        }
    }

    /// Check if a type is std::variant (or variant without std:: prefix) and return its C++ template arguments if so.
    fn get_variant_args(ty: &CppType) -> Option<Vec<String>> {
        if let CppType::Named(name) = ty {
            // Handle both "std::variant<...>" and "variant<...>" (libclang sometimes omits std::)
            let rest = name
                .strip_prefix("std::variant<")
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
            ClangNodeKind::IntegerLiteral { .. }
            | ClangNodeKind::FloatingLiteral { .. }
            | ClangNodeKind::StringLiteral(_)
            | ClangNodeKind::BoolLiteral(_) => Some(node),
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

    /// Try to generate vtable dispatch for a virtual method call.
    /// Returns Some(call_string) if this is a virtual method call through a polymorphic pointer.
    /// Returns None if this is not a virtual method call.
    fn try_generate_vtable_dispatch(&self, node: &ClangNode) -> Option<String> {
        // Virtual method calls have a MemberExpr as first child with is_arrow=true
        if node.children.is_empty() {
            return None;
        }

        // Find the MemberExpr - it might be wrapped in ImplicitCastExpr
        let member_expr = Self::find_member_expr(&node.children[0])?;

        // Check if it's an arrow access (ptr->method)
        let (member_name, is_arrow, _declaring_class) = match &member_expr.kind {
            ClangNodeKind::MemberExpr {
                member_name,
                is_arrow,
                declaring_class,
                is_static,
                ..
            } => {
                // Skip static methods
                if *is_static {
                    return None;
                }
                (member_name, *is_arrow, declaring_class.clone())
            }
            _ => return None,
        };

        // Must be arrow access (ptr->method)
        if !is_arrow {
            return None;
        }

        // Get the base expression type
        if member_expr.children.is_empty() {
            return None;
        }
        let base_type = Self::get_expr_type(&member_expr.children[0]);

        // Check if base is a pointer to a polymorphic class
        let class_name = if let Some(CppType::Pointer { pointee, .. }) = &base_type {
            if let CppType::Named(name) = pointee.as_ref() {
                // Strip "const " prefix if present for polymorphic class lookup
                let base_name = name.strip_prefix("const ").unwrap_or(name);
                if self.polymorphic_classes.contains(base_name) {
                    base_name.to_string()
                } else {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
        };

        // Check if the method is in the vtable (is virtual)
        let vtable_info = self.vtables.get(&class_name)?;
        let sanitized_member = sanitize_identifier(member_name);
        let is_virtual = vtable_info
            .entries
            .iter()
            .any(|e| sanitize_identifier(&e.name) == sanitized_member);

        if !is_virtual {
            return None;
        }

        // This is a virtual method call - generate vtable dispatch
        let base_expr = self.expr_to_string(&member_expr.children[0]);

        // Find the root polymorphic class (the one with the vtable type)
        let root_class = self.find_root_polymorphic_class(&class_name);

        // Collect arguments (skip the first child which is the MemberExpr)
        let args: Vec<String> = node.children[1..]
            .iter()
            .map(|c| self.expr_to_string(c))
            .collect();

        // Generate the vtable dispatch:
        // unsafe { ((*(*base).__vtable).method)(base, args...) }
        // For derived classes: unsafe { ((*(*base).__base.__vtable).method)(base, args...) }
        let vtable_access = if class_name == root_class {
            // Direct access to __vtable: (*base).__vtable
            format!("(*{}).", base_expr)
        } else {
            // Need to access through inheritance chain
            // Find path from class to root: (*base).__base.__vtable
            let path = self.get_vtable_access_path(&class_name);
            format!("(*{}){}.", base_expr, path)
        };

        // The vtable function expects a pointer to the root polymorphic class.
        // If we're calling through a derived class pointer, we need to cast it.
        let self_arg = if class_name == root_class {
            base_expr.clone()
        } else {
            // Cast derived pointer to root class pointer
            format!("{} as *mut {}", base_expr, root_class)
        };

        let all_args = if args.is_empty() {
            self_arg
        } else {
            format!("{}, {}", self_arg, args.join(", "))
        };

        Some(format!(
            "unsafe {{ ((*{}__vtable).{})({}) }}",
            vtable_access, sanitized_member, all_args
        ))
    }

    /// Find MemberExpr node, looking through wrapper nodes like ImplicitCastExpr
    fn find_member_expr(node: &ClangNode) -> Option<&ClangNode> {
        match &node.kind {
            ClangNodeKind::MemberExpr { .. } => Some(node),
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                // Look inside wrapper
                node.children.first().and_then(Self::find_member_expr)
            }
            _ => None,
        }
    }

    /// Get the path to access __vtable from a derived class pointer
    /// Returns something like ".__base" or ".__base.__base" for inheritance chains
    fn get_vtable_access_path(&self, class_name: &str) -> String {
        let mut path = String::new();
        let mut current = class_name.to_string();

        while let Some(vtable_info) = self.vtables.get(&current) {
            if let Some(ref base) = vtable_info.base_class {
                path.push_str(".__base");
                current = base.clone();
            } else {
                // Reached root
                break;
            }
        }

        path
    }

    /// Check if this is a std::get call on a variant.
    /// Returns (variant_arg_node, variant_type, return_type) if it is.
    fn is_std_get_call(node: &ClangNode) -> Option<(&ClangNode, CppType, &CppType)> {
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

            if let ClangNodeKind::DeclRefExpr {
                name, ty: func_ty, ..
            } = &decl_ref.kind
            {
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
    fn is_std_visit_call(node: &ClangNode) -> Option<(&ClangNode, Vec<(&ClangNode, CppType)>)> {
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

            if let ClangNodeKind::DeclRefExpr {
                name, ty: func_ty, ..
            } = &decl_ref.kind
            {
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
                                    CppType::Reference { referent, .. } => {
                                        referent.as_ref().clone()
                                    }
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
    fn is_std_views_adaptor_call(
        node: &ClangNode,
    ) -> Option<(&'static str, &ClangNode, Option<&ClangNode>)> {
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
    fn is_std_ranges_algorithm_call(
        node: &ClangNode,
    ) -> Option<(&'static str, &ClangNode, Option<&ClangNode>)> {
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
    fn get_variant_index_from_return_type(
        variant_type: &CppType,
        return_type: &CppType,
    ) -> Option<usize> {
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
                    let idx_num: String =
                        idx_str.chars().take_while(|c| c.is_ascii_digit()).collect();
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
    fn generate_visit_match(
        &self,
        visitor_node: &ClangNode,
        variants: &[(&ClangNode, CppType)],
        _return_type: &CppType,
    ) -> String {
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
                        .map(|i| {
                            format!(
                                "{}::V{}(__v) => {}",
                                enum_name,
                                i,
                                call_format.replace("{}", "__v")
                            )
                        })
                        .collect();
                    return format!("match &{} {{ {} }}", var_expr, arms.join(", "));
                }
            }
            return format!(
                "/* std::visit error: cannot process variant type {:?} */",
                var_type
            );
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
            let patterns: Vec<String> = var_info
                .iter()
                .enumerate()
                .map(|(i, (_, enum_name, _))| format!("{}::V{}(__v{})", enum_name, indices[i], i))
                .collect();
            // Build visitor call with appropriate call format
            let args: Vec<String> = (0..var_info.len()).map(|i| format!("__v{}", i)).collect();
            let args_str = args.join(", ");
            arms.push(format!(
                "({}) => {}",
                patterns.join(", "),
                call_format.replace("{}", &args_str)
            ));

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

        format!(
            "match ({}) {{ {} }}",
            tuple_expr.join(", "),
            arms.join(", ")
        )
    }

    /// Generate stub struct definitions for C++ comparison category types.
    /// These are internal types from libstdc++/libc++ that may be referenced
    /// but not fully defined in the transpiled code.
    fn generate_comparison_category_stubs(&mut self) {
        self.writeln("// Comparison category stubs for libstdc++/libc++");
        // Type aliases for comparison category internals
        self.writeln("pub type __cmp_cat_type = i8;");
        self.writeln("pub type __cmp_cat__Ord = i8;");
        self.writeln("pub type __cmp_cat__Ncmp = i8;");
        self.writeln("");
        // __cmp_cat___unspec - used in comparison expressions
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default, Copy, Clone)]");
        self.writeln("pub struct __cmp_cat___unspec { pub value: i8 }");
        self.writeln("impl __cmp_cat___unspec {");
        self.indent += 1;
        self.writeln("pub fn new_1(v: i32) -> Self { Self { value: v as i8 } }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // partial_ordering - C++20 comparison result type
        // Comparison methods are friend functions in C++, so we add them as methods here
        // Mark as generated to avoid duplicate from the C++ version
        self.generated_structs
            .insert("partial_ordering".to_string());
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default, Copy, Clone)]");
        self.writeln("pub struct partial_ordering { pub _M_value: __cmp_cat_type }");
        self.writeln("impl partial_ordering {");
        self.indent += 1;
        self.writeln("pub fn new_0() -> Self { Default::default() }");
        self.writeln("pub fn new_1(_v: __cmp_cat__Ord) -> Self { Self { _M_value: 0 } }");
        self.writeln("pub fn new_1_1(_v: __cmp_cat__Ncmp) -> Self { Self { _M_value: -127 } }");
        // Comparison operators against __cmp_cat___unspec
        self.writeln(
            "pub fn op_eq(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value == 0 }",
        );
        self.writeln(
            "pub fn op_ne(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value != 0 }",
        );
        self.writeln("pub fn op_lt(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value < 0 && self._M_value != -127 }");
        self.writeln("pub fn op_le(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value <= 0 && self._M_value != -127 }");
        self.writeln(
            "pub fn op_gt(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value > 0 }",
        );
        self.writeln(
            "pub fn op_ge(&self, _other: &__cmp_cat___unspec) -> bool { self._M_value >= 0 }",
        );
        self.indent -= 1;
        self.writeln("}");
        self.writeln("pub static PARTIAL_ORDERING_LESS: partial_ordering = partial_ordering { _M_value: -1 };");
        self.writeln("pub static PARTIAL_ORDERING_EQUIVALENT: partial_ordering = partial_ordering { _M_value: 0 };");
        self.writeln("pub static PARTIAL_ORDERING_GREATER: partial_ordering = partial_ordering { _M_value: 1 };");
        self.writeln("pub static PARTIAL_ORDERING_UNORDERED: partial_ordering = partial_ordering { _M_value: -127 };");
        self.writeln("");

        // Type trait stubs - common types from <type_traits>
        self.writeln("// Type trait stubs");
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default, Copy, Clone)]");
        self.writeln("pub struct __bool_constant_true;");
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default, Copy, Clone)]");
        self.writeln("pub struct __bool_constant_false;");
        self.writeln("");

        // Hash base stubs - used as base classes for std::hash specializations
        self.writeln("// Hash base stubs for std::hash specializations");
        for ty in &[
            "bool",
            "char",
            "signed_char",
            "unsigned_char",
            "wchar_t",
            "char8_t",
            "char16_t",
            "char32_t",
            "short",
            "int",
            "long",
            "long_long",
            "unsigned_short",
            "unsigned_int",
            "unsigned_long",
            "unsigned_long_long",
            "float",
            "double",
            "long_double",
            "nullptr_t",
        ] {
            let name = format!("__hash_base_size_t__{}", ty);
            self.generated_structs.insert(name.clone());
            self.writeln("#[repr(C)]");
            self.writeln("#[derive(Default, Copy, Clone)]");
            self.writeln(&format!("pub struct {};", name));
        }
        self.writeln("");

        // Numeric traits stubs - used as base classes for numeric_limits
        self.writeln("// Numeric traits stubs");
        for ty in &["float", "double", "long_double"] {
            let name = format!("__numeric_traits_floating_{}", ty);
            self.generated_structs.insert(name.clone());
            self.writeln("#[repr(C)]");
            self.writeln("#[derive(Default, Copy, Clone)]");
            self.writeln(&format!("pub struct {};", name));
        }
        self.writeln("");

        // Additional template placeholder stubs - only for abstract types that aren't generated from C++ code
        // These are abstract type placeholders, NOT template instantiations
        // NOTE: Do NOT add stubs for template instantiation names like std_vector_int or std__Bit_iterator
        // Those names should map to their actual generated types via types.rs mappings
        self.writeln("// Additional template placeholder stubs");
        for name in &["_dependent_type", "_Elt", "_Tag", "_Sink", "_Res", "_Ptr", "__size_type",
                     "integral_constant__Tp____v",
                     "__cv_selector__Unqualified___IsConst___IsVol",
                     "_Maybe_unary_or_binary_function__Res___Class___ArgTypes___",
                     "__detected_or_t_ptrdiff_t____diff_t___Ptr",
                     "__detected_or_t_false_type__std___allocator_traits_base___pocca___Alloc",
                     "__detected_or_t_false_type__std___allocator_traits_base___pocs___Alloc",
                     "__strictest_alignment__Types___", "_Tuple_impl_0___Elements___",
                     "std___detail___range_iter_t__Container",
                     "__detail___clamp_iter_cat_typename___traits_type_iterator_category__random_access_iterator_tag",
                     "integral_constant_size_t__sizeof_____ArgTypes_",
                     // STL iterator base types (used as empty base classes)
                     "std_iterator_std_random_access_iterator_tag__bool",
                     // Smart pointer internal types
                     "_Sp___rep",
                     // Bit vector implementation types
                     "_Bit_pointer", "_Bvector_impl",
                     // libc++ RTTI implementation types
                     "__impl___type_name_t",
                     // libc++ internal string type
                     "std___libcpp_refstring"] {
            // Don't add to generated_structs to avoid conflict with C++ definitions
            self.writeln("#[repr(C)]");
            self.writeln("#[derive(Default, Copy, Clone)]");
            self.writeln(&format!("pub struct {};", name));
        }
        self.writeln("");

        // Generate std::vector<T> template instantiation stubs
        // Since we skip template definitions, we need stubs for common instantiations
        self.writeln("// std::vector<int> instantiation stub");
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default)]");
        self.writeln("pub struct std_vector_int {");
        self.indent += 1;
        self.writeln("_data: *mut i32,");
        self.writeln("_size: usize,");
        self.writeln("_capacity: usize,");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl std_vector_int {");
        self.indent += 1;
        self.writeln("pub fn new_0() -> Self { Self { _data: std::ptr::null_mut(), _size: 0, _capacity: 0 } }");
        self.writeln("pub fn push_back(&mut self, val: i32) {");
        self.indent += 1;
        self.writeln("if self._size >= self._capacity {");
        self.indent += 1;
        self.writeln("let new_cap = if self._capacity == 0 { 4 } else { self._capacity * 2 };");
        self.writeln("let new_layout = std::alloc::Layout::array::<i32>(new_cap).unwrap();");
        self.writeln("let new_data = unsafe { std::alloc::alloc(new_layout) as *mut i32 };");
        self.writeln("if !self._data.is_null() {");
        self.indent += 1;
        self.writeln("unsafe { std::ptr::copy_nonoverlapping(self._data, new_data, self._size); }");
        self.writeln("let old_layout = std::alloc::Layout::array::<i32>(self._capacity).unwrap();");
        self.writeln("unsafe { std::alloc::dealloc(self._data as *mut u8, old_layout); }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self._data = new_data;");
        self.writeln("self._capacity = new_cap;");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("unsafe { *self._data.add(self._size) = val; }");
        self.writeln("self._size += 1;");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("pub fn size(&self) -> usize { self._size }");
        self.writeln("pub fn capacity(&self) -> usize { self._capacity }");
        self.writeln("pub fn reserve(&mut self, new_cap: i32) {");
        self.writeln("let new_cap = new_cap as usize;");
        self.indent += 1;
        self.writeln("if new_cap > self._capacity {");
        self.indent += 1;
        self.writeln("let new_layout = std::alloc::Layout::array::<i32>(new_cap).unwrap();");
        self.writeln("let new_data = unsafe { std::alloc::alloc(new_layout) as *mut i32 };");
        self.writeln("if !self._data.is_null() && self._size > 0 {");
        self.indent += 1;
        self.writeln("unsafe { std::ptr::copy_nonoverlapping(self._data, new_data, self._size); }");
        self.writeln("let old_layout = std::alloc::Layout::array::<i32>(self._capacity).unwrap();");
        self.writeln("unsafe { std::alloc::dealloc(self._data as *mut u8, old_layout); }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self._data = new_data;");
        self.writeln("self._capacity = new_cap;");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("pub fn resize(&mut self, new_size: i32) {");
        self.writeln("let new_size = new_size as usize;");
        self.indent += 1;
        self.writeln("if new_size > self._capacity {");
        self.indent += 1;
        self.writeln("self.reserve(new_size as i32);");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("while self._size < new_size {");
        self.indent += 1;
        self.writeln("unsafe { *self._data.add(self._size) = 0; }");
        self.writeln("self._size += 1;");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self._size = new_size;");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        // Implement IntoIterator for range-based for loops
        self.writeln("impl IntoIterator for std_vector_int {");
        self.indent += 1;
        self.writeln("type Item = i32;");
        self.writeln("type IntoIter = std_vector_int_iter;");
        self.writeln("fn into_iter(self) -> Self::IntoIter {");
        self.indent += 1;
        self.writeln("std_vector_int_iter { vec: self, index: 0 }");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        // Iterator struct
        self.writeln("pub struct std_vector_int_iter {");
        self.indent += 1;
        self.writeln("vec: std_vector_int,");
        self.writeln("index: usize,");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl Iterator for std_vector_int_iter {");
        self.indent += 1;
        self.writeln("type Item = i32;");
        self.writeln("fn next(&mut self) -> Option<Self::Item> {");
        self.indent += 1;
        self.writeln("if self.index < self.vec._size {");
        self.indent += 1;
        self.writeln("let val = unsafe { *self.vec._data.add(self.index) };");
        self.writeln("self.index += 1;");
        self.writeln("Some(val)");
        self.indent -= 1;
        self.writeln("} else {");
        self.indent += 1;
        self.writeln("None");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.generated_structs.insert("std_vector_int".to_string());

        // std::string stub implementation
        self.writeln("// std::string stub implementation");
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default)]");
        self.writeln("pub struct std_string {");
        self.indent += 1;
        self.writeln("_data: *mut i8,");
        self.writeln("_size: usize,");
        self.writeln("_capacity: usize,");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl std_string {");
        self.indent += 1;
        // Default constructor
        self.writeln("pub fn new_0() -> Self {");
        self.indent += 1;
        self.writeln("Self { _data: std::ptr::null_mut(), _size: 0, _capacity: 0 }");
        self.indent -= 1;
        self.writeln("}");
        // Constructor from C string
        self.writeln("pub fn new_1(s: *const i8) -> Self {");
        self.indent += 1;
        self.writeln("if s.is_null() {");
        self.indent += 1;
        self.writeln("return Self::new_0();");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("let mut len = 0usize;");
        self.writeln("unsafe { while *s.add(len) != 0 { len += 1; } }");
        self.writeln("let cap = len + 1;");
        self.writeln("let layout = std::alloc::Layout::array::<i8>(cap).unwrap();");
        self.writeln("let data = unsafe { std::alloc::alloc(layout) as *mut i8 };");
        self.writeln("unsafe { std::ptr::copy_nonoverlapping(s, data, len); }");
        self.writeln("unsafe { *data.add(len) = 0; }");
        self.writeln("Self { _data: data, _size: len, _capacity: cap }");
        self.indent -= 1;
        self.writeln("}");
        // c_str() - returns null-terminated string
        self.writeln("pub fn c_str(&self) -> *const i8 {");
        self.indent += 1;
        self.writeln("if self._data.is_null() {");
        self.indent += 1;
        self.writeln("b\"\\0\".as_ptr() as *const i8");
        self.indent -= 1;
        self.writeln("} else {");
        self.indent += 1;
        self.writeln("self._data as *const i8");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        // size() and length()
        self.writeln("pub fn size(&self) -> usize { self._size }");
        self.writeln("pub fn length(&self) -> usize { self._size }");
        // empty()
        self.writeln("pub fn empty(&self) -> bool { self._size == 0 }");
        // push_back(char)
        self.writeln("pub fn push_back(&mut self, c: i8) {");
        self.indent += 1;
        self.writeln("if self._size + 1 >= self._capacity {");
        self.indent += 1;
        self.writeln("let new_cap = if self._capacity == 0 { 16 } else { self._capacity * 2 };");
        self.writeln("let new_layout = std::alloc::Layout::array::<i8>(new_cap).unwrap();");
        self.writeln("let new_data = unsafe { std::alloc::alloc(new_layout) as *mut i8 };");
        self.writeln("if !self._data.is_null() {");
        self.indent += 1;
        self.writeln("unsafe { std::ptr::copy_nonoverlapping(self._data, new_data, self._size); }");
        self.writeln("let old_layout = std::alloc::Layout::array::<i8>(self._capacity).unwrap();");
        self.writeln("unsafe { std::alloc::dealloc(self._data as *mut u8, old_layout); }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self._data = new_data;");
        self.writeln("self._capacity = new_cap;");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("unsafe { *self._data.add(self._size) = c; }");
        self.writeln("self._size += 1;");
        self.writeln("unsafe { *self._data.add(self._size) = 0; }");
        self.indent -= 1;
        self.writeln("}");
        // append(const char*)
        self.writeln("pub fn append(&mut self, s: *const i8) -> &mut Self {");
        self.indent += 1;
        self.writeln("if s.is_null() { return self; }");
        self.writeln("let mut len = 0usize;");
        self.writeln("unsafe { while *s.add(len) != 0 { len += 1; } }");
        self.writeln("for i in 0..len {");
        self.indent += 1;
        self.writeln("self.push_back(unsafe { *s.add(i) });");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self");
        self.indent -= 1;
        self.writeln("}");
        // operator+=(const char*)
        self.writeln("pub fn op_plus_assign(&mut self, s: *const i8) -> &mut Self {");
        self.indent += 1;
        self.writeln("self.append(s)");
        self.indent -= 1;
        self.writeln("}");
        // clear()
        self.writeln("pub fn clear(&mut self) {");
        self.indent += 1;
        self.writeln("self._size = 0;");
        self.writeln("if !self._data.is_null() {");
        self.indent += 1;
        self.writeln("unsafe { *self._data = 0; }");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        // capacity()
        self.writeln("pub fn capacity(&self) -> usize { self._capacity }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        // Implement Drop to free memory
        self.writeln("impl Drop for std_string {");
        self.indent += 1;
        self.writeln("fn drop(&mut self) {");
        self.indent += 1;
        self.writeln("if !self._data.is_null() && self._capacity > 0 {");
        self.indent += 1;
        self.writeln("let layout = std::alloc::Layout::array::<i8>(self._capacity).unwrap();");
        self.writeln("unsafe { std::alloc::dealloc(self._data as *mut u8, layout); }");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.generated_structs.insert("std_string".to_string());

        // std::unordered_map<int, int> stub implementation
        self.writeln("// std::unordered_map<int, int> stub implementation");
        self.writeln("#[repr(C)]");
        self.writeln("pub struct std_unordered_map_int_int {");
        self.indent += 1;
        self.writeln("_buckets: Vec<Vec<(i32, i32)>>,");
        self.writeln("_size: usize,");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl Default for std_unordered_map_int_int {");
        self.indent += 1;
        self.writeln("fn default() -> Self {");
        self.indent += 1;
        self.writeln("Self { _buckets: vec![Vec::new(); 16], _size: 0 }");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl std_unordered_map_int_int {");
        self.indent += 1;
        // Default constructor
        self.writeln("pub fn new_0() -> Self { Default::default() }");
        // size()
        self.writeln("pub fn size(&self) -> usize { self._size }");
        // empty()
        self.writeln("pub fn empty(&self) -> bool { self._size == 0 }");
        // _hash helper
        self.writeln("#[inline]");
        self.writeln("fn _hash(key: i32) -> usize {");
        self.indent += 1;
        self.writeln("(key as u32 as usize) % 16");
        self.indent -= 1;
        self.writeln("}");
        // insert()
        self.writeln("pub fn insert(&mut self, key: i32, value: i32) {");
        self.indent += 1;
        self.writeln("let idx = Self::_hash(key);");
        self.writeln("for &mut (ref k, ref mut v) in &mut self._buckets[idx] {");
        self.indent += 1;
        self.writeln("if *k == key { *v = value; return; }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self._buckets[idx].push((key, value));");
        self.writeln("self._size += 1;");
        self.indent -= 1;
        self.writeln("}");
        // find()
        self.writeln("pub fn find(&self, key: i32) -> Option<i32> {");
        self.indent += 1;
        self.writeln("let idx = Self::_hash(key);");
        self.writeln("for &(k, v) in &self._buckets[idx] {");
        self.indent += 1;
        self.writeln("if k == key { return Some(v); }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("None");
        self.indent -= 1;
        self.writeln("}");
        // contains()
        self.writeln("pub fn contains(&self, key: i32) -> bool { self.find(key).is_some() }");
        // op_index() - operator[]
        self.writeln("pub fn op_index(&mut self, key: i32) -> &mut i32 {");
        self.indent += 1;
        self.writeln("let idx = Self::_hash(key);");
        self.writeln("for i in 0..self._buckets[idx].len() {");
        self.indent += 1;
        self.writeln("if self._buckets[idx][i].0 == key {");
        self.indent += 1;
        self.writeln("return &mut self._buckets[idx][i].1;");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self._buckets[idx].push((key, 0));");
        self.writeln("self._size += 1;");
        self.writeln("let len = self._buckets[idx].len();");
        self.writeln("&mut self._buckets[idx][len - 1].1");
        self.indent -= 1;
        self.writeln("}");
        // erase()
        self.writeln("pub fn erase(&mut self, key: i32) -> bool {");
        self.indent += 1;
        self.writeln("let idx = Self::_hash(key);");
        self.writeln("if let Some(pos) = self._buckets[idx].iter().position(|&(k, _)| k == key) {");
        self.indent += 1;
        self.writeln("self._buckets[idx].remove(pos);");
        self.writeln("self._size -= 1;");
        self.writeln("return true;");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("false");
        self.indent -= 1;
        self.writeln("}");
        // clear()
        self.writeln("pub fn clear(&mut self) {");
        self.indent += 1;
        self.writeln("for bucket in &mut self._buckets {");
        self.indent += 1;
        self.writeln("bucket.clear();");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self._size = 0;");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.generated_structs
            .insert("std_unordered_map_int_int".to_string());

        // std::unique_ptr<int> stub implementation
        self.writeln("// std::unique_ptr<int> stub implementation");
        self.writeln("#[repr(C)]");
        self.writeln("pub struct std_unique_ptr_int {");
        self.indent += 1;
        self.writeln("_ptr: *mut i32,");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl Default for std_unique_ptr_int {");
        self.indent += 1;
        self.writeln("fn default() -> Self { Self { _ptr: std::ptr::null_mut() } }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl std_unique_ptr_int {");
        self.indent += 1;
        self.writeln("pub fn new_0() -> Self { Default::default() }");
        self.writeln("pub fn new_1(ptr: *mut i32) -> Self { Self { _ptr: ptr } }");
        self.writeln("pub fn get(&self) -> *mut i32 { self._ptr }");
        self.writeln("pub fn op_deref(&self) -> &mut i32 {");
        self.indent += 1;
        self.writeln("unsafe { &mut *self._ptr }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("pub fn op_arrow(&self) -> *mut i32 { self._ptr }");
        self.writeln("pub fn release(&mut self) -> *mut i32 {");
        self.indent += 1;
        self.writeln("let ptr = self._ptr;");
        self.writeln("self._ptr = std::ptr::null_mut();");
        self.writeln("ptr");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("pub fn reset(&mut self) {");
        self.indent += 1;
        self.writeln("if !self._ptr.is_null() {");
        self.indent += 1;
        self.writeln("unsafe { drop(Box::from_raw(self._ptr)); }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self._ptr = std::ptr::null_mut();");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl Drop for std_unique_ptr_int {");
        self.indent += 1;
        self.writeln("fn drop(&mut self) {");
        self.indent += 1;
        self.writeln("if !self._ptr.is_null() {");
        self.indent += 1;
        self.writeln("unsafe { drop(Box::from_raw(self._ptr)); }");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.generated_structs
            .insert("std_unique_ptr_int".to_string());

        // std::shared_ptr<int> stub implementation
        self.writeln("// std::shared_ptr<int> stub implementation");
        self.writeln("#[repr(C)]");
        self.writeln("pub struct std_shared_ptr_int {");
        self.indent += 1;
        self.writeln("_ptr: *mut i32,");
        self.writeln("_refcount: *mut usize,");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl Default for std_shared_ptr_int {");
        self.indent += 1;
        self.writeln(
            "fn default() -> Self { Self { _ptr: std::ptr::null_mut(), _refcount: std::ptr::null_mut() } }",
        );
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl std_shared_ptr_int {");
        self.indent += 1;
        self.writeln("pub fn new_0() -> Self { Default::default() }");
        self.writeln("pub fn new_1(ptr: *mut i32) -> Self {");
        self.indent += 1;
        self.writeln("let refcount = Box::into_raw(Box::new(1usize));");
        self.writeln("Self { _ptr: ptr, _refcount: refcount }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("pub fn get(&self) -> *mut i32 { self._ptr }");
        self.writeln("pub fn op_deref(&self) -> &mut i32 {");
        self.indent += 1;
        self.writeln("unsafe { &mut *self._ptr }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("pub fn use_count(&self) -> usize {");
        self.indent += 1;
        self.writeln("if self._refcount.is_null() { 0 } else { unsafe { *self._refcount } }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("pub fn reset(&mut self) {");
        self.indent += 1;
        self.writeln("if !self._refcount.is_null() {");
        self.indent += 1;
        self.writeln("unsafe {");
        self.indent += 1;
        self.writeln("*self._refcount -= 1;");
        self.writeln("if *self._refcount == 0 {");
        self.indent += 1;
        self.writeln("if !self._ptr.is_null() { drop(Box::from_raw(self._ptr)); }");
        self.writeln("drop(Box::from_raw(self._refcount));");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("self._ptr = std::ptr::null_mut();");
        self.writeln("self._refcount = std::ptr::null_mut();");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl Clone for std_shared_ptr_int {");
        self.indent += 1;
        self.writeln("fn clone(&self) -> Self {");
        self.indent += 1;
        self.writeln("if !self._refcount.is_null() {");
        self.indent += 1;
        self.writeln("unsafe { *self._refcount += 1; }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("Self { _ptr: self._ptr, _refcount: self._refcount }");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("impl Drop for std_shared_ptr_int {");
        self.indent += 1;
        self.writeln("fn drop(&mut self) {");
        self.indent += 1;
        self.writeln("if !self._refcount.is_null() {");
        self.indent += 1;
        self.writeln("unsafe {");
        self.indent += 1;
        self.writeln("*self._refcount -= 1;");
        self.writeln("if *self._refcount == 0 {");
        self.indent += 1;
        self.writeln("if !self._ptr.is_null() { drop(Box::from_raw(self._ptr)); }");
        self.writeln("drop(Box::from_raw(self._refcount));");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.generated_structs
            .insert("std_shared_ptr_int".to_string());

        // STL algorithm stubs (std::sort, std::find, etc.)
        self.writeln("// STL algorithm stubs");
        self.writeln("");
        // std::sort
        self.writeln("/// std::sort(first, last) - sorts range [first, last) in ascending order");
        self.writeln("pub fn std_sort_int(first: *mut i32, last: *mut i32) {");
        self.indent += 1;
        self.writeln("if first.is_null() || last.is_null() { return; }");
        self.writeln("let len = unsafe { last.offset_from(first) as usize };");
        self.writeln("if len == 0 { return; }");
        self.writeln("let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };");
        self.writeln("slice.sort();");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        // std::find
        self.writeln("/// std::find(first, last, value) - returns iterator to first match or last");
        self.writeln(
            "pub fn std_find_int(first: *const i32, last: *const i32, value: i32) -> *const i32 {",
        );
        self.indent += 1;
        self.writeln("if first.is_null() || last.is_null() { return last; }");
        self.writeln("let len = unsafe { last.offset_from(first) as usize };");
        self.writeln("if len == 0 { return last; }");
        self.writeln("let slice = unsafe { std::slice::from_raw_parts(first, len) };");
        self.writeln("match slice.iter().position(|&x| x == value) {");
        self.indent += 1;
        self.writeln("Some(idx) => unsafe { first.add(idx) },");
        self.writeln("None => last,");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        // std::count
        self.writeln("/// std::count(first, last, value) - counts occurrences of value in range");
        self.writeln(
            "pub fn std_count_int(first: *const i32, last: *const i32, value: i32) -> usize {",
        );
        self.indent += 1;
        self.writeln("if first.is_null() || last.is_null() { return 0; }");
        self.writeln("let len = unsafe { last.offset_from(first) as usize };");
        self.writeln("if len == 0 { return 0; }");
        self.writeln("let slice = unsafe { std::slice::from_raw_parts(first, len) };");
        self.writeln("slice.iter().filter(|&&x| x == value).count()");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        // std::copy
        self.writeln(
            "/// std::copy(first, last, dest) - copies range to dest, returns end of dest",
        );
        self.writeln(
            "pub fn std_copy_int(first: *const i32, last: *const i32, dest: *mut i32) -> *mut i32 {",
        );
        self.indent += 1;
        self.writeln("if first.is_null() || last.is_null() || dest.is_null() { return dest; }");
        self.writeln("let len = unsafe { last.offset_from(first) as usize };");
        self.writeln("if len == 0 { return dest; }");
        self.writeln("unsafe { std::ptr::copy_nonoverlapping(first, dest, len); }");
        self.writeln("unsafe { dest.add(len) }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        // std::fill
        self.writeln("/// std::fill(first, last, value) - fills range with value");
        self.writeln("pub fn std_fill_int(first: *mut i32, last: *mut i32, value: i32) {");
        self.indent += 1;
        self.writeln("if first.is_null() || last.is_null() { return; }");
        self.writeln("let len = unsafe { last.offset_from(first) as usize };");
        self.writeln("if len == 0 { return; }");
        self.writeln("let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };");
        self.writeln("for elem in slice.iter_mut() { *elem = value; }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        // std::reverse
        self.writeln("/// std::reverse(first, last) - reverses range in place");
        self.writeln("pub fn std_reverse_int(first: *mut i32, last: *mut i32) {");
        self.indent += 1;
        self.writeln("if first.is_null() || last.is_null() { return; }");
        self.writeln("let len = unsafe { last.offset_from(first) as usize };");
        self.writeln("if len == 0 { return; }");
        self.writeln("let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };");
        self.writeln("slice.reverse();");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // Template placeholder types that appear in libc++ code
        // These are unresolved template parameters that we need stubs for
        for placeholder_type in [
            "tuple_type_parameter_0_0___",
            "_Int__Tp",
            "_Tp",
            "_Up",
            "_Args",
            "_Elements___",
        ] {
            self.writeln(&format!(
                "pub type {} = std::ffi::c_void;",
                placeholder_type
            ));
        }
        self.writeln("");

        // value_type is a special case - it's a template type alias that appears
        // in STL containers. Use c_void as a placeholder.
        self.writeln("// Template type alias placeholder");
        self.writeln("pub type value_type = std::ffi::c_void;");
        self.generated_aliases.insert("value_type".to_string());
        self.writeln("");

        // System header union types (from glibc headers)
        // These are anonymous unions that get sanitized names based on file location
        self.writeln("// System header union type stubs");
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default, Copy, Clone)]");
        self.writeln("pub struct union__unnamed_union_at__usr_include_x86_64_linux_gnu_bits_types___mbstate_t_h_16_3_ { pub __wch: u32 }");
        self.writeln("");

        // libc++ internal function stubs
        self.writeln("// libc++ internal function stubs");
        self.writeln("#[inline]");
        self.writeln("pub fn __hash(_ptr: *const i8) -> usize {");
        self.indent += 1;
        self.writeln("// FNV-1a hash for null-terminated string");
        self.writeln("let mut hash: usize = 14695981039346656037;");
        self.writeln("if _ptr.is_null() { return hash; }");
        self.writeln("let mut p = _ptr;");
        self.writeln("unsafe {");
        self.indent += 1;
        self.writeln("while *p != 0 {");
        self.indent += 1;
        self.writeln("hash ^= *p as usize;");
        self.writeln("hash = hash.wrapping_mul(1099511628211);");
        self.writeln("p = p.add(1);");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("hash");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("#[inline]");
        self.writeln("pub fn __string_to_type_name(_ptr: *const i8) -> *const i8 { _ptr }");
        self.writeln("");

        // Note: libc++ ABI namespace functions (__libcpp_is_constant_evaluated, swap, move)
        // are added to the _LIBCPP_ABI_NAMESPACE module in generate_top_level

        // Hash function stubs for libstdc++ hash implementation
        self.writeln("// Hash function stubs for libstdc++");
        self.writeln("#[inline]");
        self.writeln("pub fn _Hash_bytes(_ptr: *const (), _len: usize, _seed: usize) -> usize {");
        self.indent += 1;
        self.writeln("// Simple FNV-1a hash stub");
        self.writeln("let mut hash: usize = 14695981039346656037;");
        self.writeln("let slice = unsafe { std::slice::from_raw_parts(_ptr as *const u8, _len) };");
        self.writeln("for b in slice {");
        self.indent += 1;
        self.writeln("hash ^= *b as usize;");
        self.writeln("hash = hash.wrapping_mul(1099511628211);");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("hash ^ _seed");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("#[inline]");
        self.writeln(
            "pub fn _Fnv_hash_bytes(_ptr: *const (), _len: usize, _seed: usize) -> usize {",
        );
        self.indent += 1;
        self.writeln("// FNV-1a hash");
        self.writeln("_Hash_bytes(_ptr, _len, _seed)");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // numeric_limits stub for libstdc++
        self.writeln("// numeric_limits stub for libstdc++ allocator");
        self.writeln("pub mod numeric_limits {");
        self.indent += 1;
        self.writeln("#[inline]");
        self.writeln("pub fn min() -> isize { isize::MIN }");
        self.writeln("#[inline]");
        self.writeln("pub fn max() -> isize { isize::MAX }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // Locale nested class stubs
        // In C++, locale::facet is a nested class. When iostream is transpiled, we get both
        // references to locale_facet (qualified name) and the struct facet (unqualified).
        // Generate stubs that work regardless of whether the real types exist.
        self.writeln("// Locale nested class stubs");
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default, Clone)]");
        self.writeln("pub struct locale_facet { pub _phantom: u8 }");
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default, Clone)]");
        self.writeln("pub struct locale_id { pub _phantom: u8 }");
        self.writeln("");

        // System/pthread type stubs for libc++ threading support
        // Mark as generated to prevent duplicate struct definitions
        self.writeln("// System type stubs for libc++ threading");
        self.generated_structs.insert("__locale_struct".to_string());
        self.generated_structs
            .insert("pthread_mutexattr_t".to_string());
        self.writeln("pub type __locale_struct = std::ffi::c_void;");
        self.writeln("pub type __libcpp_mutex_t = usize;");
        self.writeln("pub type __libcpp_recursive_mutex_t = usize;");
        self.writeln("pub type __libcpp_condvar_t = usize;");
        self.writeln("pub type pthread_mutexattr_t = u32;");
        self.writeln("");

        // Missing ctype specialization stubs
        self.writeln("// ctype specialization stubs");
        self.writeln("pub type ctype_char_ = std::ffi::c_void;");
        self.writeln("pub type ctype_wchar_t_ = std::ffi::c_void;");
        self.writeln("pub type collate_char_ = std::ffi::c_void;");
        self.writeln("pub type collate_wchar_t_ = std::ffi::c_void;");
        self.writeln("");

        // Template placeholder type aliases for uninstantiated templates
        self.writeln("// Template placeholder stubs for uninstantiated template types");
        self.writeln("pub type basic_string__CharT___Traits___Allocator = std::ffi::c_void;");
        self.writeln(
            "pub type basic_string_view_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;",
        );
        self.writeln("pub type basic_string_type_parameter_0_0__char_traits_type_parameter_0_0__allocator_type_parameter_0_0 = std::ffi::c_void;");
        self.writeln("pub type basic_string_type_parameter_0_1__char_traits_type_parameter_0_1__type_parameter_0_2 = std::ffi::c_void;");
        self.writeln("pub type initializer_list_type_parameter_0_0 = std::ffi::c_void;");
        self.writeln("pub type optional__Tp = std::ffi::c_void;");
        self.writeln("pub type string_type = std::ffi::c_void;");
        self.writeln("pub type std_locale = std::ffi::c_void;"); // Stub - will be generated from iostream
        self.writeln("");

        // Iterator wrapper type stubs (skipped from generation but referenced)
        self.writeln("// Iterator wrapper type stubs");
        self.writeln("pub type __wrap_iter_typename_allocator_traits_type_parameter_0_2_const_pointer = std::ffi::c_void;");
        self.writeln("pub type __wrap_iter_typename_allocator_traits_type_parameter_0_2_pointer = std::ffi::c_void;");
        self.writeln("pub type reverse_iterator_const_type_parameter_0_0 = std::ffi::c_void;");
        self.writeln("pub type reverse_iterator_type_parameter_0_0 = std::ffi::c_void;");
        self.writeln("pub type reverse_iterator___wrap_iter_typename_allocator_traits_type_parameter_0_2_const_pointer = std::ffi::c_void;");
        self.writeln("pub type reverse_iterator___wrap_iter_typename_allocator_traits_type_parameter_0_2_pointer = std::ffi::c_void;");
        self.writeln("");

        // Chrono and format type stubs
        self.writeln("// Chrono and format type stubs");
        self.writeln("pub type chrono_nanoseconds = i64;");
        self.writeln("pub type std___extended_grapheme_custer_property_boundary___property = u32;");
        self.writeln("pub type std___format_spec___alignment = u32;");
        self.writeln("pub type _Real = f64;");
        self.writeln("pub type _Cp = std::ffi::c_void;");
        self.writeln("");

        // iostream base type stubs (libstdc++ uses different names than libc++)
        self.writeln("// iostream base type stubs");
        self.writeln("pub type std__Ios_Fmtflags = u32;");
        self.writeln("pub type std__Ios_Openmode = u32;");
        self.writeln("pub type std__Ios_Iostate = u32;");
        self.writeln("pub type std__Ios_Seekdir = i32;");
        self.writeln("pub type __gthread_mutex_t = usize;");
        self.writeln("pub type error_category = std::ffi::c_void;");
        self.writeln("pub type __ctype_abstract_base_wchar_t_ = std::ffi::c_void;");
        self.writeln("pub type _OI = std::ffi::c_void;");
        self.writeln("");

        // Template instantiation placeholders (for libstdc++ basic_string template)
        self.writeln("// libstdc++ template placeholders");
        self.writeln("pub type basic_string__CharT___Traits___Alloc = std::ffi::c_void;");
        self.writeln(
            "pub type basic_streambuf_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;",
        );
        self.writeln(
            "pub type basic_ios_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;",
        );
        self.writeln("pub type __normal_iterator_typename___alloc_traits_type_parameter_0_2__typename_type_parameter_0_2_value_type_const_pointer__basic_string__CharT___Traits___Alloc = std::ffi::c_void;");
        self.writeln("pub type __normal_iterator_typename___alloc_traits_type_parameter_0_2__typename_type_parameter_0_2_value_type_pointer__basic_string__CharT___Traits___Alloc = std::ffi::c_void;");
        self.writeln("pub type reverse_iterator___normal_iterator_typename___alloc_traits_type_parameter_0_2__typename_type_parameter_0_2_value_type_const_pointer__basic_string__CharT___Traits___Alloc = std::ffi::c_void;");
        self.writeln("pub type reverse_iterator___normal_iterator_typename___alloc_traits_type_parameter_0_2__typename_type_parameter_0_2_value_type_pointer__basic_string__CharT___Traits___Alloc = std::ffi::c_void;");
        self.writeln("");

        // More system type stubs
        self.writeln("// More system type stubs");
        self.writeln("pub type __gthread_recursive_mutex_t = usize;");
        self.writeln("pub type __gthread_cond_t = usize;");
        self.writeln("pub type _Words = std::ffi::c_void;");
        self.writeln("pub type _Alloc_hider = std::ffi::c_void;");
        self.writeln("");

        // char_traits module stub (libstdc++ uses std::char_traits)
        // Use generic functions to support char, wchar_t, char8_t, char16_t, char32_t
        self.writeln("// char_traits module stub");
        self.writeln("pub mod char_traits {");
        self.indent += 1;
        // Generic length function - counts null-terminated string length
        self.writeln("pub fn length<T: Copy + Default + PartialEq>(_s: *const T) -> u64 {");
        self.indent += 1;
        self.writeln("unsafe {");
        self.indent += 1;
        self.writeln("let mut len = 0u64;");
        self.writeln("let zero: T = Default::default();");
        self.writeln("while *_s.add(len as usize) != zero { len += 1; }");
        self.writeln("len");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("pub fn copy<T: Copy>(_dest: *mut T, _src: *const T, _n: u64) -> *mut T { unsafe { std::ptr::copy_nonoverlapping(_src, _dest, _n as usize); _dest } }");
        self.writeln("pub fn compare<T: Copy + Ord>(_s1: *const T, _s2: *const T, _n: u64) -> i32 {");
        self.indent += 1;
        self.writeln("unsafe {");
        self.indent += 1;
        self.writeln("for i in 0.._n as usize {");
        self.indent += 1;
        self.writeln("let a = *_s1.add(i);");
        self.writeln("let b = *_s2.add(i);");
        self.writeln("match a.cmp(&b) { std::cmp::Ordering::Less => return -1, std::cmp::Ordering::Greater => return 1, _ => {} }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("0");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        // Generic eq, lt functions
        self.writeln("pub fn eq<T: PartialEq>(_a: &T, _b: &T) -> bool { *_a == *_b }");
        self.writeln("pub fn lt<T: PartialOrd>(_a: &T, _b: &T) -> bool { *_a < *_b }");
        // eq_int_type is used for comparing int_type (the wider type for character comparisons)
        // Make it generic to support different int types
        self.writeln("pub fn eq_int_type<T: PartialEq>(_a: T, _b: T) -> bool { _a == _b }");
        self.writeln("pub fn to_char_type(_c: i32) -> i8 { _c as i8 }");
        self.writeln("pub fn to_int_type(_c: i8) -> i32 { _c as i32 }");
        self.writeln("pub fn eof() -> i32 { -1 }");
        self.writeln("pub fn not_eof(_c: i32) -> i32 { if _c == -1 { 0 } else { _c } }");
        self.writeln("");
        // Additional char_traits functions with type-mangled names (for wchar_t, char8_t, char16_t, char32_t)
        self.writeln("// move functions for different char types");
        self.writeln("pub fn move_ptr_mut_i8_ptr_const_i8(_dest: *mut i8, _src: *const i8, _n: u64) -> *mut i8 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }");
        self.writeln("pub fn move_ptr_mut_i32_ptr_const_i32(_dest: *mut i32, _src: *const i32, _n: u64) -> *mut i32 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }");
        self.writeln("pub fn move_ptr_mut_u8_ptr_const_u8(_dest: *mut u8, _src: *const u8, _n: u64) -> *mut u8 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }");
        self.writeln("pub fn move_ptr_mut_u16_ptr_const_u16(_dest: *mut u16, _src: *const u16, _n: u64) -> *mut u16 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }");
        self.writeln("pub fn move_ptr_mut_u32_ptr_const_u32(_dest: *mut u32, _src: *const u32, _n: u64) -> *mut u32 { unsafe { std::ptr::copy(_src, _dest, _n as usize); _dest } }");
        self.writeln("");
        self.writeln("// assign functions for different char types (fill)");
        self.writeln("pub fn assign_ptr_mut_i8(_s: *mut i8, _n: u64, _a: i8) -> *mut i8 { unsafe { for i in 0.._n as usize { *_s.add(i) = _a; } _s } }");
        self.writeln("pub fn assign_ptr_mut_i32(_s: *mut i32, _n: u64, _a: i32) -> *mut i32 { unsafe { for i in 0.._n as usize { *_s.add(i) = _a; } _s } }");
        self.writeln("pub fn assign_ptr_mut_u8(_s: *mut u8, _n: u64, _a: u8) -> *mut u8 { unsafe { for i in 0.._n as usize { *_s.add(i) = _a; } _s } }");
        self.writeln("pub fn assign_u16(_dest: &mut u16, _src: &u16) { *_dest = *_src; }");
        self.writeln("pub fn assign_u32(_dest: &mut u32, _src: &u32) { *_dest = *_src; }");
        self.writeln("");
        self.writeln("// compare functions for different char types");
        self.writeln("pub fn compare_ptr_const_i32(_s1: *const i32, _s2: *const i32, _n: u64) -> i32 { unsafe { for i in 0.._n as usize { let a = *_s1.add(i); let b = *_s2.add(i); if a != b { return if a < b { -1 } else { 1 }; } } 0 } }");
        self.writeln("pub fn compare_ptr_const_u8(_s1: *const u8, _s2: *const u8, _n: u64) -> i32 { unsafe { for i in 0.._n as usize { let a = *_s1.add(i); let b = *_s2.add(i); if a != b { return if a < b { -1 } else { 1 }; } } 0 } }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // construct_at stubs for placement new (C++20 std::construct_at)
        self.writeln("// construct_at stubs for placement new (C++20 std::construct_at)");
        self.writeln("#[inline]");
        self.writeln("pub fn construct_at_i8_ref_i8(_p: *const i8, _val: i8) -> *mut i8 { unsafe { let p = _p as *mut i8; *p = _val; p } }");
        self.writeln("#[inline]");
        self.writeln("pub fn construct_at_i32_ref_i32(_p: *const i32, _val: i32) -> *mut i32 { unsafe { let p = _p as *mut i32; *p = _val; p } }");
        self.writeln("#[inline]");
        self.writeln("pub fn construct_at_u8_ref_u8(_p: *const u8, _val: u8) -> *mut u8 { unsafe { let p = _p as *mut u8; *p = _val; p } }");
        self.writeln("#[inline]");
        self.writeln("pub fn construct_at_u16_ref_u16(_p: *const u16, _val: u16) -> *mut u16 { unsafe { let p = _p as *mut u16; *p = _val; p } }");
        self.writeln("#[inline]");
        self.writeln("pub fn construct_at_u32_ref_u32(_p: *const u32, _val: u32) -> *mut u32 { unsafe { let p = _p as *mut u32; *p = _val; p } }");
        self.writeln("");

        // More type stubs for libstdc++
        self.writeln("// More libstdc++ type stubs");
        self.writeln(
            "pub type basic_ostream_type_parameter_0_0__type_parameter_0_1 = std::ffi::c_void;",
        );
        self.writeln("pub type memory_resource = std::ffi::c_void;");
        self.writeln("");

        // Exception class stub - base class for all exception types
        // Forward declare exception_vtable to break circular dependency
        self.writeln("// Exception class stub (std::exception base class)");
        self.writeln("// Forward declaration of exception_vtable");
        self.writeln("#[repr(C)]");
        self.writeln("pub struct exception_vtable {");
        self.indent += 1;
        self.writeln("pub __type_id: u64,");
        self.writeln("pub __base_count: usize,");
        self.writeln("pub __base_type_ids: &'static [u64],");
        self.writeln("pub what: unsafe fn(*const exception) -> *const i8,");
        self.writeln("pub __destructor: unsafe fn(*mut exception),");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.generated_structs.insert("exception".to_string());
        self.generated_structs
            .insert("exception_vtable".to_string());
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Clone, Copy)]");
        self.writeln("pub struct exception {");
        self.indent += 1;
        self.writeln("pub __vtable: *const exception_vtable,");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("impl Default for exception {");
        self.indent += 1;
        self.writeln("fn default() -> Self { Self { __vtable: std::ptr::null() } }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("impl exception {");
        self.indent += 1;
        self.writeln("pub fn new_0() -> Self { Default::default() }");
        self.writeln("pub fn what(&self) -> *const i8 { b\"exception\\0\".as_ptr() as *const i8 }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // _V2 module stub for libstdc++ categories
        // Mark as generated to avoid duplicate from C++ code
        // The actual C++ _V2 namespace is usually inside std:: so track both
        self.generated_modules.insert("_V2".to_string());
        self.generated_modules.insert("std::_V2".to_string());
        self.writeln("pub mod _V2 {");
        self.indent += 1;
        self.writeln("pub fn generic_category() -> () { }");
        self.writeln("pub fn system_category() -> () { }");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // Builtin function stubs
        self.writeln("// Builtin function stubs");
        self.writeln("#[inline]");
        self.writeln("pub fn __builtin_addressof<T>(x: &T) -> *const T { x as *const T }");
        self.writeln("");

        // C library function stubs used by libstdc++ string conversion
        self.writeln("// C library function stubs");
        self.writeln("#[inline]");
        self.writeln("pub fn strtol(_s: *const i8, _endptr: *mut *mut i8, _base: i32) -> i64 {");
        self.indent += 1;
        self.writeln("// Stub: just return 0 for now");
        self.writeln("0");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("#[inline]");
        self.writeln("pub fn strtoul(_s: *const i8, _endptr: *mut *mut i8, _base: i32) -> u64 { 0 }");
        self.writeln("#[inline]");
        self.writeln("pub fn strtoll(_s: *const i8, _endptr: *mut *mut i8, _base: i32) -> i64 { 0 }");
        self.writeln("#[inline]");
        self.writeln("pub fn strtoull(_s: *const i8, _endptr: *mut *mut i8, _base: i32) -> u64 { 0 }");
        self.writeln("#[inline]");
        self.writeln("pub fn strtof(_s: *const i8, _endptr: *mut *mut i8) -> f32 { 0.0 }");
        self.writeln("#[inline]");
        self.writeln("pub fn strtod(_s: *const i8, _endptr: *mut *mut i8) -> f64 { 0.0 }");
        self.writeln("#[inline]");
        self.writeln("pub fn strtold(_s: *const i8, _endptr: *mut *mut i8) -> f64 { 0.0 }");
        self.writeln("");

        // glibc internal variable stubs
        self.writeln("// glibc internal variable stubs");
        self.writeln("pub static __libc_single_threaded: i8 = 0;");
        self.writeln("");

        // fragile_runtime stub for memory allocation
        self.writeln("// fragile_runtime stub for memory allocation");
        self.writeln("pub mod fragile_runtime {");
        self.indent += 1;
        self.writeln("#[inline]");
        self.writeln("pub unsafe fn fragile_malloc(size: usize) -> *mut () {");
        self.indent += 1;
        self.writeln("let layout = std::alloc::Layout::from_size_align(size.max(1), std::mem::align_of::<usize>()).unwrap();");
        self.writeln("std::alloc::alloc(layout) as *mut ()");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("#[inline]");
        self.writeln("pub unsafe fn fragile_free(ptr: *mut u8, size: usize) {");
        self.indent += 1;
        self.writeln("if !ptr.is_null() {");
        self.indent += 1;
        self.writeln("let layout = std::alloc::Layout::from_size_align(size.max(1), std::mem::align_of::<usize>()).unwrap();");
        self.writeln("std::alloc::dealloc(ptr, layout);");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate Rust enum definitions for all collected std::variant types.
    fn generate_variant_enums(&mut self) {
        if self.variant_types.is_empty() {
            return;
        }

        // Clone and sort by enum name for deterministic output
        let mut variants: Vec<_> = self
            .variant_types
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        variants.sort_by_key(|(name, _)| name.clone());

        for (enum_name, rust_types) in variants {
            self.writeln("/// Generated Rust enum for std::variant type");
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

        // Count how many namespaces in target_ns are "real" (generate modules)
        // vs "flattened" (std, __ prefixed namespaces that don't generate modules)
        let is_real_namespace = |ns: &str| -> bool { !ns.starts_with("__") && ns != "std" };

        // Find the common prefix length
        let common_len = target_ns
            .iter()
            .zip(self.current_namespace.iter())
            .take_while(|(a, b)| a == b)
            .count();

        // Calculate how many real module levels to go up
        // We can only go up as many levels as we have actual Rust modules
        let levels_up = self.module_depth.min(
            self.current_namespace
                .iter()
                .skip(common_len)
                .filter(|ns| is_real_namespace(ns))
                .count(),
        );

        // Build the path: super:: for going up, then the remaining target path
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..levels_up {
            parts.push("super".to_string());
        }

        // Add the remaining path segments from target_ns (after common prefix)
        // Only add segments that correspond to real modules
        for ns in target_ns.iter().skip(common_len) {
            if is_real_namespace(ns) {
                parts.push(sanitize_identifier(ns));
            }
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
        self.writeln(
            "let layout = std::alloc::Layout::from_size_align(total_size, align).unwrap();",
        );
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
        self.writeln(
            "let layout = std::alloc::Layout::from_size_align(total_size, align).unwrap();",
        );
        self.writeln("std::alloc::dealloc(base, layout);");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a top-level stub declaration (signatures only).
    fn generate_stub_top_level(&mut self, node: &ClangNode) {
        match &node.kind {
            ClangNodeKind::FunctionDecl {
                name,
                mangled_name,
                return_type,
                params,
                is_definition,
                is_variadic,
                ..
            } => {
                if *is_definition {
                    self.generate_function_stub(
                        name,
                        mangled_name,
                        return_type,
                        params,
                        *is_variadic,
                    );
                }
            }
            ClangNodeKind::RecordDecl {
                name,
                is_class,
                is_definition,
                ..
            } => {
                // Only generate struct stub for definitions
                if *is_definition {
                    self.generate_struct_stub(name, *is_class, &node.children);
                }
            }
            ClangNodeKind::EnumDecl {
                name,
                is_scoped,
                underlying_type,
            } => {
                self.generate_enum_stub(name, *is_scoped, underlying_type, &node.children);
            }
            ClangNodeKind::UnionDecl { name, .. } => {
                self.generate_union_stub(name, &node.children);
            }
            ClangNodeKind::NamespaceDecl { name } => {
                // Generate Rust module for namespace stubs
                if let Some(ns_name) = name {
                    // Skip internal namespaces or flatten them into the global scope
                    // std namespace is flattened, __ prefixed are internal, pmr has memory_resource issues
                    if ns_name.starts_with("__") || ns_name == "std" || ns_name == "pmr" {
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
    fn generate_function_stub(
        &mut self,
        name: &str,
        mangled_name: &str,
        return_type: &CppType,
        params: &[(String, CppType)],
        is_variadic: bool,
    ) {
        self.writeln(&format!("/// @fragile_cpp_mangled: {}", mangled_name));
        self.writeln(&format!("#[export_name = \"{}\"]", mangled_name));

        // Deduplicate parameter names (C++ allows unnamed params, Rust doesn't)
        let mut param_name_counts: HashMap<String, usize> = HashMap::new();
        let params_str = params
            .iter()
            .map(|(n, t)| {
                let mut param_name = sanitize_identifier(n);
                let count = param_name_counts.entry(param_name.clone()).or_insert(0);
                if *count > 0 {
                    param_name = format!("{}_{}", param_name, *count);
                }
                *param_name_counts.get_mut(&sanitize_identifier(n)).unwrap() += 1;
                format!("{}: {}", param_name, t.to_rust_type_str())
            })
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
            format!(
                " -> {}",
                Self::sanitize_return_type(&return_type.to_rust_type_str())
            )
        };

        // Variadic extern "C" functions require unsafe in Rust
        let unsafe_keyword = if is_variadic { "unsafe " } else { "" };
        self.writeln(&format!(
            "pub {}extern \"C\" fn {}({}){} {{",
            unsafe_keyword,
            sanitize_identifier(name),
            params_with_variadic,
            ret_str
        ));
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

        // Skip template DEFINITIONS that have unresolved type parameters
        if name.contains("_Tp")
            || name.contains("_Alloc")
            || name.contains("type-parameter-")
            || name.contains("type_parameter_")
        {
            return;
        }

        // Skip deep STL internal types that cause compilation issues
        if name.contains("__normal_iterator")
            || name.contains("__wrap_iter")
            || name.contains("allocator_traits<allocator<void>")
            || name.contains("allocator_traits<std::allocator<void>")
            || name.contains("numeric_limits<ranges::__detail::")
            || name.contains("hash<float>")
            || name.contains("hash<double>")
            || name.contains("hash<long double>")
            || name.contains("memory_resource")
            || name.contains("__uninitialized_copy")
            || name.contains("_Bit_iterator")  // Bit iterator has op_index returning c_void
            || name.contains("_Bit_const_iterator")
        {
            return;
        }

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

        // Add vtable pointer for ROOT polymorphic classes (those without a polymorphic base)
        // Derived classes inherit the vtable pointer through __base
        if let Some(vtable_info) = self.vtables.get(name).cloned() {
            if vtable_info.base_class.is_none() {
                // This is a root polymorphic class - add vtable pointer as first field
                self.writeln(&format!("pub __vtable: *const {}_vtable,", rust_name));
            }
        }

        // First, embed non-virtual base classes as fields (supports multiple inheritance)
        // Also collect base fields for class_fields tracking
        let mut base_fields = Vec::new();
        let mut base_idx = 0;
        for child in children {
            if let ClangNodeKind::CXXBaseSpecifier {
                base_type,
                access,
                is_virtual,
                ..
            } = &child.kind
            {
                if !matches!(access, crate::ast::AccessSpecifier::Private) {
                    if *is_virtual {
                        continue;
                    }
                    let base_name = base_type.to_rust_type_str();
                    // Use __base for single inheritance, __base0/__base1/etc for MI
                    let field_name = if base_idx == 0 {
                        "__base".to_string()
                    } else {
                        format!("__base{}", base_idx)
                    };
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
            if let ClangNodeKind::FieldDecl {
                name: field_name,
                ty,
                access,
                ..
            } = &child.kind
            {
                let sanitized_name = if field_name.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(field_name)
                };
                let vis = access_to_visibility(*access);
                self.writeln(&format!(
                    "{}{}: {},",
                    vis,
                    sanitized_name,
                    ty.to_rust_type_str()
                ));
                fields.push((sanitized_name, ty.clone()));
            } else if let ClangNodeKind::RecordDecl {
                name: anon_name, ..
            } = &child.kind
            {
                // Flatten anonymous struct fields into parent
                if anon_name.starts_with("(anonymous") || anon_name.starts_with("__anon_") {
                    for anon_child in &child.children {
                        if let ClangNodeKind::FieldDecl {
                            name: field_name,
                            ty,
                            access,
                            ..
                        } = &anon_child.kind
                        {
                            let sanitized_name = if field_name.is_empty() {
                                "_field".to_string()
                            } else {
                                sanitize_identifier(field_name)
                            };
                            let vis = access_to_visibility(*access);
                            self.writeln(&format!(
                                "{}{}: {},",
                                vis,
                                sanitized_name,
                                ty.to_rust_type_str()
                            ));
                            fields.push((sanitized_name, ty.clone()));
                        }
                    }
                }
            } else if let ClangNodeKind::UnionDecl {
                name: anon_name, ..
            } = &child.kind
            {
                // Flatten anonymous union fields into parent
                // In C++, anonymous unions allow direct access to their members from the parent
                if anon_name.starts_with("(anonymous") || anon_name.starts_with("__anon_union_") {
                    for anon_child in &child.children {
                        if let ClangNodeKind::FieldDecl {
                            name: field_name,
                            ty,
                            access,
                            ..
                        } = &anon_child.kind
                        {
                            let sanitized_name = if field_name.is_empty() {
                                "_field".to_string()
                            } else {
                                sanitize_identifier(field_name)
                            };
                            let vis = access_to_visibility(*access);
                            self.writeln(&format!(
                                "{}{}: {},",
                                vis,
                                sanitized_name,
                                ty.to_rust_type_str()
                            ));
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
    fn generate_enum_stub(
        &mut self,
        name: &str,
        is_scoped: bool,
        underlying_type: &CppType,
        children: &[ClangNode],
    ) {
        let kind = if is_scoped { "enum class" } else { "enum" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));

        // Generate as Rust enum
        // Use a valid primitive type for repr - fall back to i32 if the type is not a standard primitive
        let repr_type = match underlying_type.to_rust_type_str().as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "usize" => underlying_type.to_rust_type_str(),
            _ => "i32".to_string(),
        };
        self.writeln(&format!("#[repr({})]", repr_type));
        self.writeln("#[derive(Clone, Copy, PartialEq, Eq, Debug)]");
        self.writeln(&format!("pub enum {} {{", name));
        self.indent += 1;

        for child in children {
            if let ClangNodeKind::EnumConstantDecl {
                name: const_name,
                value,
            } = &child.kind
            {
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
        // For union DEFINITIONS, use sanitize_identifier() instead of to_rust_type_str()
        // sanitize_identifier properly escapes Rust keywords with r#
        let rust_name = sanitize_identifier(name);

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
            if let ClangNodeKind::FieldDecl {
                name: field_name,
                ty,
                access,
                ..
            } = &child.kind
            {
                let sanitized_name = if field_name.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(field_name)
                };
                let vis = access_to_visibility(*access);
                self.writeln(&format!(
                    "{}{}: {},",
                    vis,
                    sanitized_name,
                    ty.to_rust_type_str_for_field()
                ));
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a top-level declaration.
    fn generate_top_level(&mut self, node: &ClangNode) {
        match &node.kind {
            ClangNodeKind::FunctionDecl {
                name,
                mangled_name,
                return_type,
                params,
                is_definition,
                is_variadic,
                is_coroutine,
                coroutine_info,
                ..
            } => {
                if *is_definition {
                    self.generate_function(
                        name,
                        mangled_name,
                        return_type,
                        params,
                        *is_variadic,
                        *is_coroutine,
                        coroutine_info,
                        &node.children,
                    );
                }
            }
            ClangNodeKind::RecordDecl {
                name,
                is_class,
                is_definition,
                ..
            } => {
                // Only generate struct for definitions, not forward declarations
                if *is_definition {
                    self.generate_struct(name, *is_class, &node.children);
                }
            }
            ClangNodeKind::EnumDecl {
                name,
                is_scoped,
                underlying_type,
            } => {
                self.generate_enum(name, *is_scoped, underlying_type, &node.children);
            }
            ClangNodeKind::UnionDecl { name, .. } => {
                self.generate_union(name, &node.children);
            }
            ClangNodeKind::TypedefDecl {
                name,
                underlying_type,
            } => {
                self.generate_type_alias(name, underlying_type);
            }
            ClangNodeKind::TypeAliasDecl {
                name,
                underlying_type,
            } => {
                self.generate_type_alias(name, underlying_type);
            }
            ClangNodeKind::VarDecl { name, ty, has_init } => {
                // Skip out-of-class static member definitions (TypeRef child indicates qualified name)
                // These are already handled in the class generation
                let is_static_member_def = node.children.iter().any(
                    |c| matches!(&c.kind, ClangNodeKind::Unknown(s) if s.starts_with("TypeRef:")),
                );
                if !is_static_member_def {
                    self.generate_global_var(name, ty, *has_init, &node.children);
                }
            }
            ClangNodeKind::ModuleImportDecl {
                module_name,
                is_header_unit,
            } => {
                // C++20 module import → comment for now (pending full module support)
                // In the future, this could map to:
                // - `use module_name::*;` for regular modules
                // - `include!("header.rs");` for header units
                if *is_header_unit {
                    self.writeln(&format!(
                        "// C++20 header unit import: import <{}>",
                        module_name
                    ));
                } else {
                    // Convert module path separators (. or ::) to Rust path
                    let rust_path = module_name.replace('.', "::");
                    self.writeln(&format!("// C++20 module import: import {}", module_name));
                    // Generate a use statement as a placeholder
                    // When modules are fully implemented, this will become functional
                    if !rust_path.is_empty() {
                        self.writeln(&format!(
                            "// use {}::*; // (pending module implementation)",
                            sanitize_identifier(&rust_path)
                        ));
                    }
                }
            }
            ClangNodeKind::NamespaceDecl { name } => {
                // Generate Rust module for namespace
                if let Some(ns_name) = name {
                    // Skip anonymous namespaces, standard library namespaces, or problematic ones
                    // pmr namespace has memory_resource with polymorphic dispatch issues
                    if ns_name.starts_with("__") || ns_name == "std" || ns_name == "pmr" {
                        // Still track the namespace for deduplication, but don't create module
                        self.current_namespace.push(ns_name.clone());
                        for child in &node.children {
                            self.generate_top_level(child);
                        }
                        self.current_namespace.pop();
                    } else {
                        // Build full module key for deduplication
                        let module_key = if self.current_namespace.is_empty() {
                            ns_name.clone()
                        } else {
                            format!("{}::{}", self.current_namespace.join("::"), ns_name)
                        };

                        // Check if this is the first occurrence of this module
                        let is_first = !self.generated_modules.contains(&module_key);
                        if is_first {
                            self.generated_modules.insert(module_key.clone());
                        }

                        // For duplicate namespaces, skip - we generate merged contents on first occurrence
                        if !is_first {
                            return;
                        }

                        self.writeln(&format!("pub mod {} {{", sanitize_identifier(ns_name)));
                        self.indent += 1;
                        self.module_depth += 1; // Track actual Rust module depth

                        // Track current namespace for relative path computation
                        self.current_namespace.push(ns_name.clone());

                        // Use merged namespace contents from all occurrences
                        // This handles C++ namespace reopening (same namespace declared multiple times)
                        if let Some(merged_indices) =
                            self.merged_namespace_children.get(&module_key).cloned()
                        {
                            for idx in merged_indices {
                                if let Some(child) = self.collected_nodes.get(idx).cloned() {
                                    self.generate_top_level(&child);
                                }
                            }
                        } else {
                            // Fallback: use direct children if not in merged map
                            for child in &node.children {
                                self.generate_top_level(child);
                            }
                        }

                        self.current_namespace.pop();

                        // Add stub functions for specific libc++ internal namespaces
                        if ns_name == "_LIBCPP_ABI_NAMESPACE" {
                            self.writeln("/// libc++ constant evaluation check (always returns false at runtime)");
                            self.writeln("#[inline]");
                            self.writeln(
                                "pub fn __libcpp_is_constant_evaluated() -> bool { false }",
                            );
                            self.writeln("");
                            self.writeln("/// swap function stub");
                            self.writeln("#[inline]");
                            self.writeln(
                                "pub fn swap<T>(a: &mut T, b: &mut T) { std::mem::swap(a, b); }",
                            );
                            self.writeln("");
                            self.writeln("/// move function stub  ");
                            self.writeln("#[inline]");
                            self.writeln("pub fn r#move<T>(v: T) -> T { v }");
                        }

                        self.module_depth -= 1;
                        self.indent -= 1;
                        self.writeln("}");
                        self.writeln("");
                    }
                } else {
                    // Anonymous namespace - generate private module with synthetic name
                    // This mirrors C++ semantics where anonymous namespaces have internal linkage
                    let anon_name = format!("__anon_{}", self.anon_namespace_counter);
                    self.anon_namespace_counter += 1;

                    self.writeln("/// Anonymous namespace (internal linkage)");
                    self.writeln(&format!("mod {} {{", anon_name));
                    self.indent += 1;
                    self.module_depth += 1;

                    // Track the synthetic namespace name for path resolution
                    self.current_namespace.push(anon_name.clone());
                    for child in &node.children {
                        self.generate_top_level(child);
                    }
                    self.current_namespace.pop();

                    self.module_depth -= 1;
                    self.indent -= 1;
                    self.writeln("}");

                    // Auto-use the contents so they're accessible in parent scope
                    self.writeln(&format!("use {}::*;", anon_name));
                    self.writeln("");
                }
            }
            ClangNodeKind::ClassTemplateDecl {
                name: template_name,
                template_params,
                ..
            } => {
                // Store template definition for later instantiation
                // Children include TemplateTypeParmDecl (template params) and FieldDecl/CXXMethodDecl (members)
                self.template_definitions.insert(
                    template_name.clone(),
                    (template_params.clone(), node.children.clone()),
                );

                // Process children of class template to find implicit instantiations
                for child in &node.children {
                    match &child.kind {
                        // Template instantiations appear as RecordDecl children with
                        // type names containing template arguments (e.g., "MyVec<int>")
                        ClangNodeKind::RecordDecl {
                            name: child_name,
                            is_class,
                            is_definition,
                            ..
                        } => {
                            // Only process instantiations (names with <...>) that are definitions
                            if *is_definition
                                && child_name.contains('<')
                                && child_name.contains('>')
                            {
                                self.generate_struct(child_name, *is_class, &child.children);
                            }
                        }
                        _ => {
                            // Recursively process other children (might contain nested instantiations)
                            self.generate_top_level(child);
                        }
                    }
                }
            }
            ClangNodeKind::ClassTemplatePartialSpecDecl { .. } => {
                // Partial specializations are like regular structs with the specialized types
                // The name will include the specialization pattern (e.g., "Pair<T, T>")
                // For now, process children to find any instantiations
                for child in &node.children {
                    if let ClangNodeKind::RecordDecl {
                        name: child_name,
                        is_class,
                        is_definition,
                        ..
                    } = &child.kind
                    {
                        // Only generate for definitions
                        if *is_definition && child_name.contains('<') && child_name.contains('>') {
                            self.generate_struct(child_name, *is_class, &child.children);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Get the appropriate return type string for a function, considering coroutine info.
    /// For async coroutines with value type, uses the extracted type.
    /// For generators, could use impl Iterator<Item=T> (future enhancement).
    fn get_coroutine_return_type(
        &self,
        return_type: &CppType,
        coroutine_info: &Option<CoroutineInfo>,
    ) -> String {
        if let Some(info) = coroutine_info {
            // If we extracted a value type from the coroutine return type, use it
            if let Some(ref value_type) = info.value_type {
                match info.kind {
                    CoroutineKind::Async | CoroutineKind::Task => {
                        // async fn returns the inner type directly
                        if *value_type == CppType::Void {
                            return String::new();
                        }
                        return format!(
                            " -> {}",
                            Self::sanitize_return_type(&value_type.to_rust_type_str())
                        );
                    }
                    CoroutineKind::Generator => {
                        // Generators should return impl Iterator<Item=T>
                        // Note: Rust generators are unstable, so this is forward-looking
                        return format!(
                            " -> impl Iterator<Item={}>",
                            Self::sanitize_return_type(&value_type.to_rust_type_str())
                        );
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
            format!(
                " -> {}",
                Self::sanitize_return_type(&return_type.to_rust_type_str())
            )
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
        self.writeln(&format!(
            "/// State machine struct for generator `{}`",
            func_name
        ));
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
            self.writeln(&format!(
                "{} => {{ self.__state = {}; Some({}) }}",
                i,
                i + 1,
                yield_val
            ));
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
    fn generate_function(
        &mut self,
        name: &str,
        mangled_name: &str,
        return_type: &CppType,
        params: &[(String, CppType)],
        is_variadic: bool,
        is_coroutine: bool,
        coroutine_info: &Option<CoroutineInfo>,
        children: &[ClangNode],
    ) {
        // Skip functions from problematic STL internal namespaces
        // pmr namespace functions use memory_resource which has polymorphic dispatch issues
        if mangled_name.contains("pmr") || mangled_name.contains("memory_resource") {
            return;
        }

        // Skip functions that reference skipped types
        // Check if any parameter or return type contains skipped type names
        let has_skipped_type = |ty: &CppType| {
            let type_str = ty.to_rust_type_str();
            type_str.contains("_Bit_iterator")
                || type_str.contains("_Bit_const_iterator")
                || type_str.contains("__normal_iterator")
                || type_str.contains("__wrap_iter")
                || type_str.contains("memory_resource")
        };
        if has_skipped_type(return_type) || params.iter().any(|(_, t)| has_skipped_type(t)) {
            return;
        }

        // Skip functions with variadic template parameters (C++ parameter packs)
        // These contain patterns like `_Tp &&...` or `_Args...` which can't be expressed in Rust
        let has_variadic_pack = |ty: &CppType| {
            let type_str = ty.to_rust_type_str();
            type_str.contains("&&...") || type_str.contains("...")
        };
        if params.iter().any(|(_, t)| has_variadic_pack(t)) {
            return;
        }

        // Skip functions with decltype return types (can't be expressed in Rust)
        let return_type_str = return_type.to_rust_type_str();
        if return_type_str.contains("decltype") {
            return;
        }

        // Skip functions with unresolved template type parameters in return type
        // These are template definitions that haven't been fully instantiated
        if return_type_str.contains("_Tp")
            || return_type_str.contains("_Args")
            || return_type_str.contains("type_parameter_")
        {
            return;
        }

        // Skip functions that return bare c_void (placeholder for unresolved types like std::string)
        // Also skip functions with c_void parameters (except pointer/ref to c_void which is valid)
        if return_type_str == "std::ffi::c_void" {
            return;
        }
        if params.iter().any(|(_, t)| {
            let ts = t.to_rust_type_str();
            ts == "std::ffi::c_void"
        }) {
            return;
        }

        // Special handling for C++ main function
        let is_main = name == "main" && params.is_empty();
        // Use sanitized name for duplicate tracking to avoid suffix issues with operators
        // e.g., "operator&" becomes "op_bitand", so we track "op_bitand" not "operator&"
        let sanitized_base_name = if is_main {
            "cpp_main".to_string()
        } else {
            sanitize_identifier(name)
        };

        // Handle function overloading by appending suffix for duplicates
        let count = self
            .generated_functions
            .entry(sanitized_base_name.clone())
            .or_insert(0);
        let func_name = if *count == 0 {
            *count += 1;
            sanitized_base_name
        } else {
            *count += 1;
            format!("{}_{}", sanitized_base_name, *count - 1)
        };

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
            self.writeln(&format!(
                "/// Coroutine: {} ({})",
                kind_str, info.return_type_spelling
            ));
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
                || matches!(param_type, CppType::Array { size: None, .. })
            {
                self.ptr_vars.insert(param_name.clone());
            }
            // Only track sized arrays as arrays
            if matches!(param_type, CppType::Array { size: Some(_), .. }) {
                self.arr_vars.insert(param_name.clone());
            }
        }

        // Collect parameters that are assigned to within the function body
        // C++ allows modifying by-value params, but Rust requires `mut`
        let assigned_params = Self::collect_assigned_params_from_children(children, params);

        // Function signature - convert polymorphic pointers to trait objects
        // Deduplicate parameter names (C++ allows unnamed params, Rust doesn't)
        let mut param_name_counts: HashMap<String, usize> = HashMap::new();
        let params_str = params
            .iter()
            .map(|(n, t)| {
                let type_str = self.convert_type_for_polymorphism(t);
                let mut param_name = sanitize_identifier(n);
                // If this parameter name has been seen before, add a suffix
                let count = param_name_counts.entry(param_name.clone()).or_insert(0);
                if *count > 0 {
                    param_name = format!("{}_{}", param_name, *count);
                }
                *param_name_counts.get_mut(&sanitize_identifier(n)).unwrap() += 1;
                // Add `mut` if this parameter is assigned to in the body
                let mut_prefix = if assigned_params.contains(n) {
                    "mut "
                } else {
                    ""
                };
                format!("{}{}: {}", mut_prefix, param_name, type_str)
            })
            .collect::<Vec<_>>()
            .join(", ");

        // Determine return type based on coroutine info
        let ret_str = self.get_coroutine_return_type(return_type, coroutine_info);

        // Check if this is a generator
        let is_generator = is_coroutine
            && matches!(
                coroutine_info.as_ref().map(|i| i.kind),
                Some(CoroutineKind::Generator)
            );

        // Determine if this should be an async function
        let is_async = is_coroutine
            && matches!(
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
            self.generate_generator_struct(&func_name, &item_type, &yields);

            // Generate the function that returns the generator
            let struct_name = format!("{}Generator", to_pascal_case(&func_name));
            self.writeln(&format!(
                "pub fn {}({}){} {{",
                func_name, // Already sanitized above
                params_str,
                ret_str
            ));
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

            // Variadic functions require extern "C" linkage and unsafe keyword
            let (async_keyword, extern_c) = if is_variadic {
                ("", "unsafe extern \"C\" ")
            } else if is_async {
                ("async ", "")
            } else {
                ("", "")
            };
            self.writeln(&format!(
                "pub {}{}fn {}({}){} {{",
                async_keyword,
                extern_c,
                func_name, // Already sanitized above
                params_with_variadic,
                ret_str
            ));
            self.indent += 1;

            // Track return type for return statement handling
            let old_return_type = self.current_return_type.take();
            self.current_return_type = Some(return_type.clone());

            // Find the compound statement (function body)
            for child in children {
                if let ClangNodeKind::CompoundStmt = &child.kind {
                    self.generate_block_contents(&child.children, return_type);
                }
            }

            self.current_return_type = old_return_type;
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
            if let ClangNodeKind::FieldDecl {
                name: field_name,
                ty,
                access,
                is_static,
                bit_field_width,
            } = &child.kind
            {
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

        // Track anonymous bit field count for unique naming
        let mut anon_count = 0;

        for group in &groups {
            let storage_type = group.storage_type();
            let storage_field = format!("_bitfield_{}", group.group_index);

            for field in &group.fields {
                let vis = access_to_visibility(field.access);
                // Handle anonymous bit fields: give them unique names
                let field_name = if field.field_name.is_empty() {
                    anon_count += 1;
                    format!("_unnamed_{}", anon_count)
                } else {
                    sanitize_identifier(&field.field_name)
                };
                let ret_type = field.original_type.to_rust_type_str();

                // Calculate mask for this field's width
                let mask = (1u64 << field.width) - 1;

                // Getter: extract bits and cast to original type
                self.writeln(&format!("/// Getter for bit field `{}`", field.field_name));
                self.writeln(&format!(
                    "{}fn {}(&self) -> {} {{",
                    vis, field_name, ret_type
                ));
                self.indent += 1;
                if field.offset == 0 {
                    self.writeln(&format!(
                        "(self.{} & 0x{:X}) as {}",
                        storage_field, mask, ret_type
                    ));
                } else {
                    self.writeln(&format!(
                        "((self.{} >> {}) & 0x{:X}) as {}",
                        storage_field, field.offset, mask, ret_type
                    ));
                }
                self.indent -= 1;
                self.writeln("}");
                self.writeln("");

                // Setter: clear bits and set new value
                self.writeln(&format!("/// Setter for bit field `{}`", field.field_name));
                self.writeln(&format!(
                    "{}fn set_{}(&mut self, v: {}) {{",
                    vis, field_name, ret_type
                ));
                self.indent += 1;
                if field.offset == 0 {
                    self.writeln(&format!(
                        "self.{} = (self.{} & !0x{:X}) | ((v as {}) & 0x{:X});",
                        storage_field, storage_field, mask, storage_type, mask
                    ));
                } else {
                    let shifted_mask = mask << field.offset;
                    self.writeln(&format!(
                        "self.{} = (self.{} & !0x{:X}) | (((v as {}) & 0x{:X}) << {});",
                        storage_field,
                        storage_field,
                        shifted_mask,
                        storage_type,
                        mask,
                        field.offset
                    ));
                }
                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }
        }
    }

    /// Generate synthesized arithmetic operators (op_add, op_sub) for iterators
    /// If a struct has op_add_assign but no op_add, we synthesize op_add.
    /// This handles C++ binary operators that are friend functions, not members.
    /// Note: Only synthesize for types that look like iterators (have op_inc/op_dec)
    fn generate_synthesized_arithmetic_operators(&mut self) {
        // Only synthesize for iterator-like types (have increment/decrement operators)
        let has_inc = self.current_struct_methods.contains_key("op_inc");
        let has_dec = self.current_struct_methods.contains_key("op_dec");

        if !has_inc && !has_dec {
            // Not an iterator-like type, don't synthesize
            return;
        }

        // Check what methods exist in current_struct_methods
        let has_add_assign = self.current_struct_methods.contains_key("op_add_assign");
        let has_add = self.current_struct_methods.contains_key("op_add");
        let has_sub_assign = self.current_struct_methods.contains_key("op_sub_assign");
        let has_sub = self.current_struct_methods.contains_key("op_sub");

        // Synthesize op_add if op_add_assign exists but op_add doesn't
        if has_add_assign && !has_add {
            self.writeln("");
            self.writeln("/// Synthesized operator+ (C++ friend function)");
            self.writeln("pub fn op_add(&self, __n: isize) -> Self {");
            self.indent += 1;
            self.writeln("let mut result = self.clone();");
            self.writeln("result.op_add_assign(__n);");
            self.writeln("result");
            self.indent -= 1;
            self.writeln("}");
        }

        // Synthesize op_sub if op_sub_assign exists but op_sub doesn't
        if has_sub_assign && !has_sub {
            self.writeln("");
            self.writeln("/// Synthesized operator- (C++ friend function)");
            self.writeln("pub fn op_sub(&self, __n: isize) -> Self {");
            self.indent += 1;
            self.writeln("let mut result = self.clone();");
            self.writeln("result.op_sub_assign(__n);");
            self.writeln("result");
            self.indent -= 1;
            self.writeln("}");
        }

        // Synthesize op_deref if op_index exists but op_deref doesn't
        // This handles C++ iterators with operator[] that calls operator*
        // e.g., _Bit_iterator::operator[] returns *(*this + __i)
        let has_index = self.current_struct_methods.contains_key("op_index");
        let has_deref = self.current_struct_methods.contains_key("op_deref");

        if has_index && !has_deref {
            self.writeln("");
            self.writeln("/// Synthesized operator* (C++ dereference)");
            self.writeln("/// Returns reference - actual type depends on container");
            self.writeln("pub fn op_deref(&self) -> &std::ffi::c_void {");
            self.indent += 1;
            self.writeln("// Stub: actual implementation depends on container type");
            self.writeln("unsafe { &*(std::ptr::null::<std::ffi::c_void>()) }");
            self.indent -= 1;
            self.writeln("}");
        }
    }

    /// Generate struct definition.
    fn generate_struct(&mut self, name: &str, is_class: bool, children: &[ClangNode]) {
        // For struct DEFINITIONS, use sanitize_identifier() instead of to_rust_type_str()
        // to_rust_type_str() maps some types to primitives (e.g., exception -> c_void)
        // which is wrong for struct definitions - we want the actual struct name
        let rust_name = sanitize_identifier(name);

        // Skip template DEFINITIONS that have unresolved type parameters.
        // Template definitions use names like "vector<_Tp, _Alloc>" or contain type-parameter-X-X.
        // We should only generate structs for actual instantiations like "vector<int>".
        // Clang presents template definitions with dependent type parameter names.
        if name.contains("_Tp")
            || name.contains("_Alloc")
            || name.contains("type-parameter-")
            || name.contains("type_parameter_")
            || (name.contains('<') && (name.contains("_T>") || name.contains("_T,")))
        {
            // This is a template definition, not an instantiation - skip it
            // The actual instantiation (e.g., std::vector<int>) will generate its own struct
            return;
        }

        // Skip deep STL internal types that cause compilation issues
        // These aren't needed for basic container usage and have complex template dependencies
        if name.contains("numeric_limits<ranges::__detail::")  // Return c_void for template types
            || name.contains("hash<float>")  // Hash specialization has wrong arg count
            || name.contains("hash<double>") // Hash specialization has wrong arg count
            || name.contains("hash<long double>")
            || name.contains("memory_resource")  // Polymorphic dispatch issues
            || name.contains("__wrap_iter")  // Iterator wrapper with template issues
            || name.contains("__normal_iterator")  // Iterator wrapper
            || name.contains("allocator_traits<std::allocator<void>")  // Returns &c_void.clone()
            || name.contains("allocator_traits<allocator<void>")  // Returns &c_void.clone()
            || name.contains("__uninitialized_copy")  // Template metaprogramming helper
            || name.contains("_Bit_iterator")  // Bit iterator has op_index returning c_void
            || name.contains("_Bit_const_iterator")
        {
            return;
        }

        // Skip if already generated (handles duplicate template instantiations)
        if self.generated_structs.contains(&rust_name) {
            return;
        }

        self.generated_structs.insert(rust_name.clone());

        // Check if there's an explicit copy constructor - if so, we'll generate Clone impl later
        // Otherwise, derive Clone along with Default
        let has_explicit_copy_ctor = children.iter().any(|child| {
            matches!(
                &child.kind,
                ClangNodeKind::ConstructorDecl {
                    ctor_kind: ConstructorKind::Copy,
                    is_definition: true,
                    ..
                }
            )
        });

        // Check if there's any field that would prevent deriving Default:
        // - Arrays larger than 32 elements (Rust's Default is only impl'd for arrays up to [T; 32])
        // - Fields of type c_void which doesn't implement Default
        let has_non_default_field = children.iter().any(|child| {
            if let ClangNodeKind::FieldDecl { ty, is_static, .. } = &child.kind {
                if *is_static {
                    return false;
                }
                // Check for large arrays (Default only impl'd up to [T; 32])
                if let CppType::Array { size: Some(n), .. } = ty {
                    if *n > 32 {
                        return true;
                    }
                }
                // Check for c_void fields (c_void doesn't implement Default)
                let type_str = ty.to_rust_type_str();
                if type_str == "std::ffi::c_void" || type_str.ends_with("c_void") {
                    return true;
                }
                // Check for array of c_void
                if let CppType::Array { element, .. } = ty {
                    let elem_str = element.to_rust_type_str();
                    if elem_str == "std::ffi::c_void" || elem_str.ends_with("c_void") {
                        return true;
                    }
                }
                false
            } else {
                false
            }
        });

        let kind = if is_class { "class" } else { "struct" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));
        self.writeln("#[repr(C)]");
        // Derive Clone for trivially copyable types (no explicit copy ctor)
        // For types with explicit copy ctor, we generate Clone impl separately
        // Skip Default derive if struct has fields that don't impl Default (large arrays, c_void)
        if has_non_default_field {
            // Can't derive Default - the struct needs a manual Default impl
            if has_explicit_copy_ctor {
                // Neither Default nor Clone can be derived
            } else {
                self.writeln("#[derive(Clone)]");
            }
        } else if has_explicit_copy_ctor {
            self.writeln("#[derive(Default)]");
        } else {
            self.writeln("#[derive(Default, Clone)]");
        }
        self.writeln(&format!("pub struct {} {{", rust_name));
        self.indent += 1;

        // Add vtable pointer for ROOT polymorphic classes (those without a polymorphic base)
        // Derived classes inherit the vtable pointer through __base
        if let Some(vtable_info) = self.vtables.get(name).cloned() {
            if vtable_info.base_class.is_none() {
                // This is a root polymorphic class - add vtable pointer as first field
                self.writeln(&format!("pub __vtable: *const {}_vtable,", rust_name));
            }
        }

        // First, embed non-virtual base classes as fields (supports multiple inheritance)
        // Base classes must come first to maintain C++ memory layout
        let mut base_fields = Vec::new();
        let mut base_idx = 0;
        for child in children {
            if let ClangNodeKind::CXXBaseSpecifier {
                base_type,
                access,
                is_virtual,
                ..
            } = &child.kind
            {
                // Only include public/protected bases (private inheritance is more complex)
                if !matches!(access, crate::ast::AccessSpecifier::Private) {
                    if *is_virtual {
                        continue;
                    }
                    let base_name = base_type.to_rust_type_str();
                    // Use __base for first base (backward compatible), __base1/__base2/etc for MI
                    let field_name = if base_idx == 0 {
                        "__base".to_string()
                    } else {
                        format!("__base{}", base_idx)
                    };
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
            self.bit_field_groups
                .insert(name.to_string(), bit_groups.clone());
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
            if let ClangNodeKind::FieldDecl {
                name: fname,
                ty,
                is_static,
                access,
                bit_field_width,
            } = &child.kind
            {
                if *is_static || bit_field_width.is_some() {
                    continue; // Static fields handled separately, bit fields handled above
                }
                let sanitized_name = if fname.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(fname)
                };
                let vis = access_to_visibility(*access);
                self.writeln(&format!(
                    "{}{}: {},",
                    vis,
                    sanitized_name,
                    ty.to_rust_type_str_for_field()
                ));
                fields.push((sanitized_name, ty.clone()));
            } else if let ClangNodeKind::RecordDecl {
                name: anon_name, ..
            } = &child.kind
            {
                // Flatten anonymous struct fields into parent
                if anon_name.starts_with("(anonymous") || anon_name.starts_with("__anon_") {
                    for anon_child in &child.children {
                        if let ClangNodeKind::FieldDecl {
                            name: fname,
                            ty,
                            is_static,
                            access,
                            bit_field_width,
                        } = &anon_child.kind
                        {
                            if *is_static || bit_field_width.is_some() {
                                continue;
                            }
                            let sanitized_name = if fname.is_empty() {
                                "_field".to_string()
                            } else {
                                sanitize_identifier(fname)
                            };
                            let vis = access_to_visibility(*access);
                            self.writeln(&format!(
                                "{}{}: {},",
                                vis,
                                sanitized_name,
                                ty.to_rust_type_str_for_field()
                            ));
                            fields.push((sanitized_name, ty.clone()));
                        }
                    }
                }
            } else if let ClangNodeKind::UnionDecl {
                name: anon_name, ..
            } = &child.kind
            {
                // Flatten anonymous union fields into parent
                // In C++, anonymous unions allow direct access to their members from the parent
                if anon_name.starts_with("(anonymous") || anon_name.starts_with("__anon_union_") {
                    for anon_child in &child.children {
                        if let ClangNodeKind::FieldDecl {
                            name: fname,
                            ty,
                            is_static,
                            access,
                            bit_field_width,
                        } = &anon_child.kind
                        {
                            if *is_static || bit_field_width.is_some() {
                                continue;
                            }
                            let sanitized_name = if fname.is_empty() {
                                "_field".to_string()
                            } else {
                                sanitize_identifier(fname)
                            };
                            let vis = access_to_visibility(*access);
                            self.writeln(&format!(
                                "{}{}: {},",
                                vis,
                                sanitized_name,
                                ty.to_rust_type_str_for_field()
                            ));
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

        // Generate manual Default impl for structs that can't derive Default
        // (due to large arrays or c_void fields)
        if has_non_default_field && !has_explicit_copy_ctor {
            self.writeln(&format!("impl Default for {} {{", rust_name));
            self.indent += 1;
            self.writeln("fn default() -> Self { unsafe { std::mem::zeroed() } }");
            self.indent -= 1;
            self.writeln("}");
        }

        // Generate static member variables as globals
        for child in children {
            if let ClangNodeKind::FieldDecl {
                name: field_name,
                ty,
                is_static: true,
                ..
            } = &child.kind
            {
                // Use sanitize_static_member_name for uppercase global names
                // to avoid r# prefix issues with keywords like "in"
                let sanitized_field = sanitize_static_member_name(field_name);
                let sanitized_struct = sanitize_static_member_name(name);
                let rust_ty = ty.to_rust_type_str();
                let global_name = format!(
                    "{}_{}",
                    sanitized_struct.to_uppercase(),
                    sanitized_field.to_uppercase()
                );
                self.writeln("");
                self.writeln(&format!("/// Static member `{}::{}`", name, field_name));
                self.writeln(&format!(
                    "static mut {}: {} = {};",
                    global_name,
                    rust_ty,
                    Self::default_value_for_type(ty)
                ));
                // Register the static member for later lookup
                self.static_members
                    .insert((name.to_string(), field_name.clone()), global_name);
            }
        }

        // Check if there's an explicit default constructor (0 params)
        let has_default_ctor = children.iter().any(|c| {
            matches!(&c.kind, ClangNodeKind::ConstructorDecl { params, is_definition: true, .. } if params.is_empty())
        });

        // Generate impl block for methods
        let methods: Vec<_> = children
            .iter()
            .filter(|c| {
                matches!(
                    &c.kind,
                    ClangNodeKind::CXXMethodDecl {
                        is_definition: true,
                        ..
                    } | ClangNodeKind::ConstructorDecl {
                        is_definition: true,
                        ..
                    }
                )
            })
            .collect();

        // Check if we have bit fields that need accessor methods
        let has_bit_fields = self.bit_field_groups.contains_key(name);

        // Always generate impl block if we need new_0, have other methods, or have bit fields
        if !methods.is_empty() || !has_default_ctor || has_bit_fields {
            self.writeln("");
            self.writeln(&format!("impl {} {{", rust_name));
            self.indent += 1;

            // Clear method counter for this struct's impl block
            self.current_struct_methods.clear();

            // Generate default new_0() if no explicit default constructor
            if !has_default_ctor {
                // Track new_0 so overloaded constructors don't collide
                self.current_struct_methods.insert("new_0".to_string(), 1);
                self.writeln("pub fn new_0() -> Self {");
                self.indent += 1;

                // Check if this is a polymorphic class that needs vtable initialization
                if let Some(vtable_info) = self.vtables.get(name).cloned() {
                    let sanitized = sanitize_identifier(name);
                    // Abstract classes don't have vtable instances, use Default
                    if vtable_info.is_abstract {
                        self.writeln("Default::default()");
                    } else if vtable_info.base_class.is_none() {
                        // Root polymorphic class - set vtable directly
                        self.writeln("Self {");
                        self.indent += 1;
                        self.writeln(&format!("__vtable: &{}_VTABLE,", sanitized.to_uppercase()));
                        self.writeln("..Default::default()");
                        self.indent -= 1;
                        self.writeln("}");
                    } else {
                        // Derived polymorphic class - set vtable through base chain
                        let vtable_path = self.compute_vtable_access_path(name);
                        self.writeln("let mut __self = Self::default();");
                        self.writeln(&format!(
                            "__self.{}.__vtable = &{}_VTABLE;",
                            vtable_path,
                            sanitized.to_uppercase()
                        ));
                        self.writeln("__self");
                    }
                } else {
                    self.writeln("Default::default()");
                }

                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }

            for method in methods {
                self.generate_method(method, name);
            }

            // Generate bit field accessor methods
            self.generate_bit_field_accessors(name);

            // Generate synthesized arithmetic operators for iterators
            // If a struct has op_add_assign but no op_add, synthesize op_add
            self.generate_synthesized_arithmetic_operators();

            // Add stub constructors for exception classes that need string/const char* constructors
            // These are called by derived classes but may not have definitions in headers
            if name == "logic_error" || name == "runtime_error" {
                // Check if new_1 was generated (has definition)
                let has_new_1 = self
                    .current_struct_methods
                    .get("new_1")
                    .copied()
                    .unwrap_or(0)
                    > 0;
                if !has_new_1 {
                    self.writeln("");
                    self.writeln(
                        "/// Stub constructor for string argument (libc++ exception class)",
                    );
                    self.writeln("pub fn new_1(_s: &std::ffi::c_void) -> Self {");
                    self.indent += 1;
                    self.writeln("Default::default()");
                    self.indent -= 1;
                    self.writeln("}");
                }
                // Check if new_1_1 was generated
                let has_new_1_1 = self
                    .current_struct_methods
                    .get("new_1_1")
                    .copied()
                    .unwrap_or(0)
                    > 0;
                if !has_new_1_1 {
                    self.writeln("");
                    self.writeln(
                        "/// Stub constructor for const char* argument (libc++ exception class)",
                    );
                    self.writeln("pub fn new_1_1(_s: *const i8) -> Self {");
                    self.indent += 1;
                    self.writeln("Default::default()");
                    self.indent -= 1;
                    self.writeln("}");
                }
            }

            self.indent -= 1;
            self.writeln("}");
        }

        // Generate Drop impl if there's a destructor
        for child in children {
            if let ClangNodeKind::DestructorDecl {
                is_definition: true,
                ..
            } = &child.kind
            {
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

        // Generate Clone impl if there's an explicit copy constructor
        // (otherwise Clone is derived via #[derive(Default, Clone)] above)
        if has_explicit_copy_ctor {
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
        }

        // Note: Trait generation removed - now using vtable-based dispatch
        // See Task 25.7 for vtable dispatch implementation

        self.writeln("");
    }

    /// Generate an enum definition.
    fn generate_enum(
        &mut self,
        name: &str,
        is_scoped: bool,
        underlying_type: &CppType,
        children: &[ClangNode],
    ) {
        // Skip enums with dependent types (template parameters)
        let repr_type = underlying_type.to_rust_type_str();
        if repr_type == "_dependent_type"
            || repr_type == "integral_constant__Tp____v"
            || repr_type.starts_with("type_parameter_")
            || repr_type.contains("_parameter_")
        {
            return;
        }

        // Skip unnamed enums that have problematic names (e.g., "(unnamed enum at ...)")
        // These are typically internal implementation details in C++ headers
        if name.starts_with("(unnamed") || name.contains(" at ") {
            // For unnamed enums with constants, generate the constants as standalone constants
            for child in children {
                if let ClangNodeKind::EnumConstantDecl {
                    name: const_name,
                    value,
                } = &child.kind
                {
                    if let Some(v) = value {
                        self.writeln(&format!(
                            "pub const {}: {} = {};",
                            sanitize_identifier(const_name),
                            repr_type,
                            v
                        ));
                    }
                }
            }
            if children
                .iter()
                .any(|c| matches!(&c.kind, ClangNodeKind::EnumConstantDecl { .. }))
            {
                self.writeln("");
            }
            return;
        }

        // Sanitize the name to handle Rust keywords and special characters
        let safe_name = sanitize_identifier(name);

        // Skip if already generated (handles duplicate definitions from template instantiation or reopened namespaces)
        if self.generated_structs.contains(name) {
            return;
        }
        self.generated_structs.insert(name.to_string());

        let kind = if is_scoped { "enum class" } else { "enum" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));

        // Generate as Rust enum
        // Use a valid primitive type for repr - fall back to i32 if the type is not a standard primitive
        let repr_type = match underlying_type.to_rust_type_str().as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "usize" => underlying_type.to_rust_type_str(),
            _ => "i32".to_string(), // Default to i32 for non-primitive underlying types
        };

        // Check if this is an empty enum (no variants)
        let has_variants = children
            .iter()
            .any(|c| matches!(&c.kind, ClangNodeKind::EnumConstantDecl { .. }));

        if has_variants {
            // First pass: collect all variants and detect duplicates
            let mut seen_values: HashMap<i64, String> = HashMap::new();
            let mut duplicates: Vec<(String, i64, String)> = Vec::new(); // (alias_name, value, original_name)

            for child in children {
                if let ClangNodeKind::EnumConstantDecl {
                    name: const_name,
                    value,
                } = &child.kind
                {
                    let safe_const_name = sanitize_identifier(const_name);
                    if let Some(v) = value {
                        if let Some(original) = seen_values.get(v) {
                            // Duplicate value - save for const alias generation
                            duplicates.push((safe_const_name, *v, original.clone()));
                        } else {
                            seen_values.insert(*v, safe_const_name);
                        }
                    }
                }
            }

            self.writeln(&format!("#[repr({})]", repr_type));
            self.writeln("#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]");
            self.writeln(&format!("pub enum {} {{", safe_name));
            self.indent += 1;

            let mut first = true;
            for child in children {
                if let ClangNodeKind::EnumConstantDecl {
                    name: const_name,
                    value,
                } = &child.kind
                {
                    // Sanitize enum constant names (e.g., "unsized" is a Rust reserved keyword)
                    let safe_const_name = sanitize_identifier(const_name);

                    // Skip if this is a duplicate value alias
                    if duplicates
                        .iter()
                        .any(|(alias, _, _)| alias == &safe_const_name)
                    {
                        continue;
                    }

                    if first {
                        // First variant is the default
                        self.writeln("#[default]");
                        first = false;
                    }
                    if let Some(v) = value {
                        self.writeln(&format!("{} = {},", safe_const_name, v));
                    } else {
                        self.writeln(&format!("{},", safe_const_name));
                    }
                }
            }

            self.indent -= 1;
            self.writeln("}");

            // Generate const aliases for duplicate values
            for (alias_name, _value, original_name) in &duplicates {
                self.writeln(&format!(
                    "pub const {}: {} = {}::{};",
                    alias_name.to_uppercase(),
                    safe_name,
                    safe_name,
                    original_name
                ));
            }
        } else {
            // Empty enum - generate as a zero-sized struct instead (Rust doesn't support repr on empty enums)
            self.writeln("#[repr(transparent)]");
            self.writeln("#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]");
            self.writeln(&format!("pub struct {}({});", safe_name, repr_type));
        }
        self.writeln("");
    }

    /// Generate a Rust union from a C++ union declaration.
    fn generate_union(&mut self, name: &str, children: &[ClangNode]) {
        // For union DEFINITIONS, use sanitize_identifier() instead of to_rust_type_str()
        // to_rust_type_str() maps some types to primitives (e.g., type -> void)
        // which is wrong for union definitions - we want the actual union name
        // sanitize_identifier also properly escapes Rust keywords with r#
        let rust_name = sanitize_identifier(name);

        // Skip if already generated as struct/union
        if self.generated_structs.contains(&rust_name) {
            return;
        }
        // Skip if already generated as type alias (avoid symbol collision)
        if self.generated_aliases.contains(&rust_name) {
            return;
        }
        self.generated_structs.insert(rust_name.clone());

        // Check if any field contains c_void (requires ManuallyDrop, which breaks Copy)
        let has_cvoid_field = children.iter().any(|child| {
            if let ClangNodeKind::FieldDecl { ty, is_static, .. } = &child.kind {
                if *is_static {
                    return false;
                }
                let type_str = ty.to_rust_type_str();
                type_str == "std::ffi::c_void" || type_str.contains("c_void")
            } else {
                false
            }
        });

        self.writeln(&format!("/// C++ union `{}`", name));
        self.writeln("#[repr(C)]");
        // Can't derive Copy/Clone if any field needs ManuallyDrop (c_void doesn't impl Copy/Clone)
        if !has_cvoid_field {
            self.writeln("#[derive(Copy, Clone)]");
        }
        self.writeln(&format!("pub union {} {{", rust_name));
        self.indent += 1;

        let mut fields = Vec::new();
        for child in children {
            if let ClangNodeKind::FieldDecl {
                name: field_name,
                ty,
                is_static,
                access,
                ..
            } = &child.kind
            {
                if *is_static {
                    continue;
                }
                let sanitized_name = if field_name.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(field_name)
                };
                let vis = access_to_visibility(*access);
                let type_str = ty.to_rust_type_str();
                // Wrap non-Copy types in ManuallyDrop for union compatibility
                // c_void is used as placeholder for template types and doesn't impl Copy
                let wrapped_type = if type_str == "std::ffi::c_void" || type_str.contains("c_void")
                {
                    format!("std::mem::ManuallyDrop<{}>", type_str)
                } else {
                    type_str
                };
                self.writeln(&format!("{}{}: {},", vis, sanitized_name, wrapped_type));
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

        // Generate Clone impl if we have c_void fields (can't derive it)
        if has_cvoid_field {
            self.writeln(&format!("impl Clone for {} {{", rust_name));
            self.indent += 1;
            self.writeln("fn clone(&self) -> Self {");
            self.indent += 1;
            // Use unsafe memcpy to clone the union bytes
            self.writeln("unsafe {");
            self.indent += 1;
            self.writeln("let mut copy: Self = std::mem::zeroed();");
            self.writeln("std::ptr::copy_nonoverlapping(self, &mut copy, 1);");
            self.writeln("copy");
            self.indent -= 1;
            self.writeln("}");
            self.indent -= 1;
            self.writeln("}");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
        }
    }

    /// Generate a type alias for typedef or using declarations.
    fn generate_type_alias(&mut self, name: &str, underlying_type: &CppType) {
        // Sanitize the name to handle Rust keywords (e.g., "type" -> "r#type")
        let safe_name = sanitize_identifier(name);

        // Skip if this alias was already generated (common in template metaprogramming)
        if self.generated_aliases.contains(&safe_name) {
            return;
        }
        self.generated_aliases.insert(safe_name.clone());

        // Convert the underlying C++ type to Rust
        let rust_type = underlying_type.to_rust_type_str();
        self.writeln(&format!("/// C++ typedef/using `{}`", name));
        self.writeln(&format!("pub type {} = {};", safe_name, rust_type));
        self.writeln("");
    }

    /// Generate a global variable declaration.
    fn generate_global_var(
        &mut self,
        name: &str,
        ty: &CppType,
        _has_init: bool,
        children: &[ClangNode],
    ) {
        // Sanitize the name to handle special characters and keywords
        let base_name = sanitize_identifier(name);

        // Prefix global variables with __gv_ to prevent parameter shadowing
        // Rust doesn't allow function parameters to shadow statics, so we need unique names
        let safe_name = format!("__gv_{}", base_name);

        // Skip if already generated (handles duplicates from template instantiation)
        if self.global_vars.contains(&safe_name) {
            return;
        }

        // Skip template non-type parameters and dependent types
        // These are placeholder types from templates that shouldn't become global variables
        let rust_type = ty.to_rust_type_str();
        if rust_type == "_dependent_type"
            || rust_type == "integral_constant__Tp____v"
            || rust_type.starts_with("type_parameter_")
            || rust_type.contains("_parameter_")
        {
            return;
        }
        // Track this as a global variable (needs unsafe access and deduplication)
        // Store the mapping from original name to prefixed name for reference resolution
        self.global_vars.insert(safe_name.clone());
        self.global_var_mapping
            .insert(base_name.clone(), safe_name.clone());
        self.writeln(&format!("/// C++ global variable `{}`", name));

        // Get initial value if present
        // Handle different cases:
        // - Arrays without initializers have IntegerLiteral (size) as first child
        // - Arrays with initializers have InitListExpr as first child
        // - Static member definitions have TypeRef as first child (skip it)
        // - Regular variables have their initializer as first child
        let init_value = if !children.is_empty() {
            // Find the actual initializer, skipping TypeRef for qualified definitions
            let init_idx = if matches!(&children[0].kind, ClangNodeKind::Unknown(s) if s.starts_with("TypeRef:"))
            {
                // Skip TypeRef child for qualified definitions like "int Counter::count = 0"
                if children.len() > 1 {
                    Some(1)
                } else {
                    None
                }
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
                    // Skip literal suffixes - Rust will infer type from variable declaration
                    self.skip_literal_suffix = true;
                    let init_str = self.expr_to_string(init_node);
                    self.skip_literal_suffix = false;

                    // Check if the expression contains unresolved _unnamed references
                    // This happens with unresolved template parameters in numeric_limits, etc.
                    // Fall back to default value in these cases
                    if init_str.contains("_unnamed") {
                        Self::default_value_for_static(ty)
                    } else if matches!(ty, CppType::Bool) {
                        // Handle bool type with integer initializer (C++ allows 0/1 for bool)
                        match init_str.as_str() {
                            "0" | "0i32" => "false".to_string(),
                            "1" | "1i32" => "true".to_string(),
                            _ => init_str,
                        }
                    } else if matches!(ty, CppType::Named(_)) {
                        // For struct types, convert 0 to zeroed memory initialization
                        match init_str.as_str() {
                            "0" | "0i32" => "unsafe { std::mem::zeroed() }".to_string(),
                            _ => init_str,
                        }
                    } else {
                        init_str
                    }
                }
            } else {
                Self::default_value_for_static(ty)
            }
        } else {
            // No children: use default value
            Self::default_value_for_static(ty)
        };

        self.writeln(&format!(
            "static mut {}: {} = {};",
            safe_name, rust_type, init_value
        ));
        self.writeln("");
    }

    /// Generate a const-safe default value for static variables.
    fn default_value_for_static(ty: &CppType) -> String {
        match ty {
            CppType::Int { .. }
            | CppType::Short { .. }
            | CppType::Long { .. }
            | CppType::LongLong { .. }
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

    /// Generate a vtable struct for a polymorphic class.
    /// The vtable contains function pointers for all virtual methods.
    fn generate_vtable_struct(&mut self, class_name: &str, vtable_info: &ClassVTableInfo) {
        let sanitized_name = sanitize_identifier(class_name);
        let vtable_name = format!("{}_vtable", sanitized_name);

        // Skip if vtable struct is already generated (e.g., from stubs)
        if self.generated_structs.contains(&vtable_name) {
            return;
        }
        self.generated_structs.insert(vtable_name.clone());

        self.writeln("");
        self.writeln(&format!(
            "/// VTable for polymorphic class `{}`",
            class_name
        ));
        self.writeln("#[repr(C)]");
        self.writeln(&format!("pub struct {} {{", vtable_name));
        self.indent += 1;

        // RTTI fields for dynamic_cast support
        self.writeln("/// Type ID (hash of class name) for runtime type checking");
        self.writeln("pub __type_id: u64,");
        self.writeln("/// Number of entries in __base_type_ids array");
        self.writeln("pub __base_count: usize,");
        self.writeln(
            "/// Array of base class type IDs (includes self, ordered from derived to base)",
        );
        self.writeln("pub __base_type_ids: &'static [u64],");

        // Track method names to handle overloaded methods
        let mut method_name_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // Generate function pointer field for each virtual method
        for entry in &vtable_info.entries {
            let base_method_name = sanitize_identifier(&entry.name);
            // Handle overloaded methods by adding suffix for duplicates
            let count = method_name_counts
                .entry(base_method_name.clone())
                .or_insert(0);
            let method_name = if *count == 0 {
                *count += 1;
                base_method_name
            } else {
                *count += 1;
                format!("{}_{}", base_method_name, *count - 1)
            };
            let return_type = Self::sanitize_return_type(&entry.return_type.to_rust_type_str());

            // Build parameter list: first param is self pointer, then explicit params
            let self_ptr = if entry.is_const {
                format!("*const {}", sanitized_name)
            } else {
                format!("*mut {}", sanitized_name)
            };

            let param_types: Vec<String> = entry
                .params
                .iter()
                .map(|(_, ptype)| ptype.to_rust_type_str())
                .collect();

            let all_params = if param_types.is_empty() {
                self_ptr
            } else {
                format!("{}, {}", self_ptr, param_types.join(", "))
            };

            if return_type == "()" {
                self.writeln(&format!("pub {}: unsafe fn({}),", method_name, all_params));
            } else {
                self.writeln(&format!(
                    "pub {}: unsafe fn({}) -> {},",
                    method_name, all_params, return_type
                ));
            }
        }

        // Add destructor entry (always present for polymorphic classes)
        self.writeln(&format!(
            "pub __destructor: unsafe fn(*mut {}),",
            sanitized_name
        ));

        self.indent -= 1;
        self.writeln("}");
    }

    /// Convert a type to Rust for polymorphic pointers.
    /// Uses raw pointers for vtable-based dispatch.
    fn convert_type_for_polymorphism(&self, ty: &CppType) -> String {
        match ty {
            CppType::Pointer { pointee, is_const } => {
                // Check if pointee is a polymorphic class
                if let CppType::Named(class_name) = pointee.as_ref() {
                    if self.polymorphic_classes.contains(class_name) {
                        // Use raw pointer for vtable-based dispatch
                        let sanitized = sanitize_identifier(class_name);
                        return if *is_const {
                            format!("*const {}", sanitized)
                        } else {
                            format!("*mut {}", sanitized)
                        };
                    }
                }
                // Not polymorphic, use regular pointer type
                ty.to_rust_type_str()
            }
            _ => ty.to_rust_type_str(),
        }
    }

    /// Collect parameter names that are assigned to within a function/method body.
    /// C++ allows modifying pass-by-value parameters, but Rust requires `mut`.
    fn collect_assigned_params(node: &ClangNode, params: &[(String, CppType)]) -> HashSet<String> {
        let param_names: HashSet<String> = params.iter().map(|(n, _)| n.clone()).collect();
        let mut assigned = HashSet::new();
        Self::find_param_assignments(node, &param_names, &mut assigned);
        assigned
    }

    /// Like collect_assigned_params but works on a slice of children nodes (for top-level functions).
    fn collect_assigned_params_from_children(
        children: &[ClangNode],
        params: &[(String, CppType)],
    ) -> HashSet<String> {
        let param_names: HashSet<String> = params.iter().map(|(n, _)| n.clone()).collect();
        let mut assigned = HashSet::new();
        for child in children {
            Self::find_param_assignments(child, &param_names, &mut assigned);
        }
        assigned
    }

    /// Recursively find assignments to parameters.
    fn find_param_assignments(
        node: &ClangNode,
        param_names: &HashSet<String>,
        assigned: &mut HashSet<String>,
    ) {
        // Check for assignment operators
        if let ClangNodeKind::BinaryOperator { op, .. } = &node.kind {
            let is_assignment = matches!(
                op,
                BinaryOp::Assign
                    | BinaryOp::AddAssign
                    | BinaryOp::SubAssign
                    | BinaryOp::MulAssign
                    | BinaryOp::DivAssign
                    | BinaryOp::RemAssign
                    | BinaryOp::AndAssign
                    | BinaryOp::OrAssign
                    | BinaryOp::XorAssign
                    | BinaryOp::ShlAssign
                    | BinaryOp::ShrAssign
            );
            if is_assignment && !node.children.is_empty() {
                // Check if left side is a DeclRefExpr to a parameter
                if let Some(name) = Self::get_declref_name(&node.children[0]) {
                    if param_names.contains(&name) {
                        assigned.insert(name);
                    }
                }
            }
        }

        // Check for increment/decrement operators
        if let ClangNodeKind::UnaryOperator { op, .. } = &node.kind {
            match op {
                UnaryOp::PreInc | UnaryOp::PostInc | UnaryOp::PreDec | UnaryOp::PostDec => {
                    if !node.children.is_empty() {
                        if let Some(name) = Self::get_declref_name(&node.children[0]) {
                            if param_names.contains(&name) {
                                assigned.insert(name);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Recurse into children
        for child in &node.children {
            Self::find_param_assignments(child, param_names, assigned);
        }
    }

    /// Get the name from a DeclRefExpr (possibly wrapped in casts).
    fn get_declref_name(node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, .. } => Some(name.clone()),
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                if !node.children.is_empty() {
                    Self::get_declref_name(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Extract member assignments from a constructor body.
    /// Looks for patterns like `this->field = value;` or `field = value;`
    fn extract_member_assignments(
        node: &ClangNode,
        initializers: &mut Vec<(String, String)>,
        codegen: &AstCodeGen,
    ) {
        for child in &node.children {
            // Look for ExprStmt containing BinaryOperator with Assign
            if let ClangNodeKind::ExprStmt = &child.kind {
                if !child.children.is_empty() {
                    Self::extract_assignment(&child.children[0], initializers, codegen);
                }
            } else if let ClangNodeKind::BinaryOperator {
                op: BinaryOp::Assign,
                ..
            } = &child.kind
            {
                Self::extract_assignment(child, initializers, codegen);
            }
            // Recursively check compound statements
            if let ClangNodeKind::CompoundStmt = &child.kind {
                Self::extract_member_assignments(child, initializers, codegen);
            }
        }
    }

    /// Extract a single member assignment from a BinaryOperator node.
    fn extract_assignment(
        node: &ClangNode,
        initializers: &mut Vec<(String, String)>,
        codegen: &AstCodeGen,
    ) {
        if let ClangNodeKind::BinaryOperator {
            op: BinaryOp::Assign,
            ..
        } = &node.kind
        {
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
            ClangNodeKind::ArraySubscriptExpr { .. } => {
                // For array subscript (e.g., data[i]), get member name from the base (data)
                if !node.children.is_empty() {
                    Self::get_member_name(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if a method's body only returns *this (self)
    /// Used to fix return types when c_void is a placeholder
    fn method_returns_this_only(node: &ClangNode) -> bool {
        // Find CompoundStmt (method body)
        for child in &node.children {
            if let ClangNodeKind::CompoundStmt = &child.kind {
                // Check if the only meaningful statement is "return *this" or similar
                return Self::body_returns_this(&child.children);
            }
        }
        false
    }

    /// Check if a list of statements ultimately returns *this
    fn body_returns_this(stmts: &[ClangNode]) -> bool {
        // Must have at least one statement
        if stmts.is_empty() {
            return false;
        }

        // The last (or only) statement that matters should be a return of *this
        for stmt in stmts {
            match &stmt.kind {
                ClangNodeKind::ReturnStmt => {
                    // Check if it returns *this
                    if !stmt.children.is_empty() {
                        return Self::expr_is_this(&stmt.children[0]);
                    }
                    return false;
                }
                ClangNodeKind::ExprStmt => {
                    // Skip other expressions, continue to check return
                    continue;
                }
                _ => {
                    // Any other statement type (like if/while/etc) - don't assume
                    continue;
                }
            }
        }
        false
    }

    /// Check if an expression is *this
    fn expr_is_this(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::UnaryOperator {
                op: UnaryOp::Deref, ..
            } => {
                // *this pattern
                if !node.children.is_empty() {
                    if let ClangNodeKind::CXXThisExpr { .. } = &node.children[0].kind {
                        return true;
                    }
                    // Also check through implicit casts
                    return Self::expr_is_this(&node.children[0]);
                }
                false
            }
            ClangNodeKind::CXXThisExpr { .. } => {
                // Just 'this' (returning pointer to self)
                true
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                // Check through casts
                if !node.children.is_empty() {
                    return Self::expr_is_this(&node.children[0]);
                }
                false
            }
            ClangNodeKind::CallExpr { .. } => {
                // Copy constructor call or other call with *this as argument
                if !node.children.is_empty() {
                    return Self::expr_is_this(&node.children[0]);
                }
                false
            }
            ClangNodeKind::Unknown(_) => {
                // Handle unknown wrapper nodes (like MaterializeTemporaryExpr, ExprWithCleanups)
                if !node.children.is_empty() {
                    return Self::expr_is_this(&node.children[0]);
                }
                false
            }
            _ => false,
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
            ClangNodeKind::BinaryOperator {
                op: BinaryOp::Assign,
                ..
            } => {
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
            ClangNodeKind::ArraySubscriptExpr { .. } => {
                // For array subscript (e.g., data[i]), check the base (data)
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
    fn extract_constructor_args(&mut self, node: &ClangNode) -> Vec<String> {
        let mut args = Vec::new();
        // Skip literal suffixes - Rust will infer types from constructor parameters
        let prev_skip = self.skip_literal_suffix;
        self.skip_literal_suffix = true;
        match &node.kind {
            ClangNodeKind::CallExpr { .. } => {
                // Arguments are children of the call expression
                for child in &node.children {
                    // Skip type references and function references
                    match &child.kind {
                        ClangNodeKind::Unknown(s) if s == "TypeRef" => continue,
                        ClangNodeKind::DeclRefExpr { .. }
                        | ClangNodeKind::IntegerLiteral { .. }
                        | ClangNodeKind::FloatingLiteral { .. }
                        | ClangNodeKind::BoolLiteral(_)
                        | ClangNodeKind::ImplicitCastExpr { .. }
                        | ClangNodeKind::BinaryOperator { .. }
                        | ClangNodeKind::UnaryOperator { .. } => {
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
                    self.skip_literal_suffix = prev_skip;
                    return self.extract_constructor_args(&node.children[0]);
                }
            }
            _ => {}
        }
        self.skip_literal_suffix = prev_skip;
        args
    }

    /// Check if a node is a pointer dereference (possibly wrapped in casts).
    fn is_pointer_deref(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::UnaryOperator {
                op: UnaryOp::Deref, ..
            } => true,
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
            // Also look through MemberExpr - e.g., `c->data[idx].val` where we need to
            // detect the pointer subscript `c->data[idx]` in the base of `.val`
            ClangNodeKind::MemberExpr { is_arrow, .. } => {
                if *is_arrow {
                    // Arrow access itself involves pointer dereference, but check base too
                    !node.children.is_empty() && self.is_pointer_subscript(&node.children[0])
                } else {
                    // For dot access like `.val`, check if the base involves pointer subscript
                    !node.children.is_empty() && self.is_pointer_subscript(&node.children[0])
                }
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
            ClangNodeKind::DeclRefExpr {
                ty,
                namespace_path,
                name,
                ..
            } => {
                // Static members accessed via Class::member have namespace_path with class name
                if !namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                    return true;
                }
                // Also check if this is a static member of the current class (accessed without Class:: prefix)
                if namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                    if let Some(ref current_class) = self.current_class {
                        if self
                            .static_members
                            .contains_key(&(current_class.clone(), name.clone()))
                        {
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
            ClangNodeKind::DeclRefExpr { name, .. } => self.ptr_vars.contains(name),
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
                let sanitized = sanitize_identifier(name);
                self.global_var_mapping.contains_key(&sanitized)
            }
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                // Look through casts and unknown wrappers
                !node.children.is_empty() && self.is_global_var_expr(&node.children[0])
            }
            _ => false,
        }
    }

    /// Get the raw variable name from a DeclRefExpr (unwrapping casts).
    /// If the variable is a global variable, returns the prefixed name (__gv_...).
    fn get_raw_var_name(&self, node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, .. } => {
                let sanitized = sanitize_identifier(name);
                // Check if this is a global variable and return the prefixed name
                if let Some(prefixed) = self.global_var_mapping.get(&sanitized) {
                    Some(prefixed.clone())
                } else {
                    Some(sanitized)
                }
            }
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

    /// Get the original type of an expression, looking through implicit casts.
    /// This returns the type of the innermost expression before any implicit conversions.
    /// For example, for an ImplicitCastExpr<UncheckedDerivedToBase> from _Bit_iterator to _Bit_iterator_base,
    /// this returns the original _Bit_iterator type, not the casted _Bit_iterator_base type.
    fn get_original_expr_type(node: &ClangNode) -> Option<CppType> {
        match &node.kind {
            // For ImplicitCastExpr, look through to get the original type
            ClangNodeKind::ImplicitCastExpr { .. } => {
                if !node.children.is_empty() {
                    Self::get_original_expr_type(&node.children[0])
                } else {
                    None
                }
            }
            // For wrapper nodes, look through
            ClangNodeKind::Unknown(_) | ClangNodeKind::ParenExpr { .. } => {
                if !node.children.is_empty() {
                    Self::get_original_expr_type(&node.children[0])
                } else {
                    None
                }
            }
            // For other nodes, return the actual type
            _ => Self::get_expr_type(node),
        }
    }

    /// Extract the class name from a type, handling const qualifiers, references, and pointers.
    /// For example, "const Point" -> "Point", Reference { pointee: Named("Point") } -> "Point"
    fn extract_class_name(ty: &Option<CppType>) -> Option<String> {
        ty.as_ref().and_then(Self::extract_class_name_from_type)
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

    /// Strip namespace prefix and template arguments from a class name.
    /// Used for comparing class names when detecting inherited member access.
    /// e.g., "std::ctype<char>" -> "ctype", "std::_Bit_reference" -> "_Bit_reference"
    fn strip_namespace_and_template(s: &str) -> String {
        // First strip namespace prefix
        let unqual = if let Some(pos) = s.rfind("::") {
            &s[pos + 2..]
        } else {
            s
        };
        // Then strip template arguments (e.g., ctype<char> -> ctype)
        if let Some(pos) = unqual.find('<') {
            unqual[..pos].to_string()
        } else {
            unqual.to_string()
        }
    }

    /// Get the base access path for a member declared in a specific base class.
    fn get_base_access_for_class(&self, current_class: &str, declaring_class: &str) -> BaseAccess {
        // Strip namespace prefix from current_class for lookup
        // The class_bases map uses unqualified names, but current_class may be qualified (e.g., std::_Bit_iterator)
        let current_class_unqual = if let Some(pos) = current_class.rfind("::") {
            &current_class[pos + 2..]
        } else {
            current_class
        };

        if let Some(vbases) = self
            .virtual_bases
            .get(current_class)
            .or_else(|| self.virtual_bases.get(current_class_unqual))
        {
            if vbases.iter().any(|b| b == declaring_class) {
                return BaseAccess::VirtualPtr(self.virtual_base_field_name(declaring_class));
            }
        }

        // Try both qualified and unqualified names for class_bases lookup
        let base_classes = self
            .class_bases
            .get(current_class)
            .or_else(|| self.class_bases.get(current_class_unqual));
        if let Some(base_classes) = base_classes {
            let mut non_virtual_idx = 0;
            for base in base_classes {
                if base.name == declaring_class {
                    if base.is_virtual {
                        return BaseAccess::VirtualPtr(
                            self.virtual_base_field_name(declaring_class),
                        );
                    }
                    let field = if non_virtual_idx == 0 {
                        "__base".to_string()
                    } else {
                        format!("__base{}", non_virtual_idx)
                    };
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
                        let first_base = if non_virtual_base_idx == 0 {
                            "__base".to_string()
                        } else {
                            format!("__base{}", non_virtual_base_idx)
                        };
                        return BaseAccess::FieldChain(format!("{}.__base", first_base));
                    }
                }
            }
            // Has base classes but declaring_class wasn't found - fallback to __base
            return BaseAccess::DirectField("__base".to_string());
        }

        // No base class info for current_class - this means it's a template or stub type
        // that wasn't fully parsed. Return empty access to indicate no base field needed.
        // The calling code should check for empty field names and skip base access.
        BaseAccess::DirectField(String::new())
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
            ClangNodeKind::MemberExpr { ty, .. } => {
                // For method calls, ty may be a Function type (for regular methods)
                // or a special "<bound member function type>" string in Named
                if let CppType::Function { params, .. } = ty {
                    Some(params.clone())
                } else if let CppType::Named(name) = ty {
                    // Parse "<bound member function type>" - contains param types
                    // Format: "type (Class::*)(param1, param2, ...) const"
                    // For now, try to extract from the type string
                    Self::parse_member_function_params(name)
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

    /// Parse parameter types from a bound member function type string.
    /// The format is typically "<bound member function type>" but might also be
    /// "type (Class::*)(param1, param2, ...) const" style.
    fn parse_member_function_params(type_str: &str) -> Option<Vec<CppType>> {
        // Most common case: "<bound member function type>" doesn't contain actual type info
        // We need a different approach - check the function signature from the class
        if type_str.contains("bound member function type") {
            return None;
        }

        // Try to parse "(param1, param2, ...)" from the string
        if let Some(start) = type_str.find(")(") {
            if let Some(end) = type_str[start + 2..].find(')') {
                let params_str = &type_str[start + 2..start + 2 + end];
                if params_str.is_empty() {
                    return Some(vec![]);
                }
                // Split by comma and parse each param type
                let params: Vec<CppType> = params_str
                    .split(',')
                    .map(|s| {
                        let s = s.trim();
                        // Check for reference types
                        if s.ends_with('&') {
                            let inner = s.trim_end_matches('&').trim();
                            let is_const = inner.starts_with("const ");
                            let inner_type = if is_const {
                                inner.strip_prefix("const ").unwrap_or(inner).trim()
                            } else {
                                inner
                            };
                            CppType::Reference {
                                referent: Box::new(CppType::Named(inner_type.to_string())),
                                is_const,
                                is_rvalue: false,
                            }
                        } else {
                            CppType::Named(s.to_string())
                        }
                    })
                    .collect();
                return Some(params);
            }
        }

        None
    }

    /// Check if a MemberExpr (possibly wrapped) is a virtual base method call.
    /// Returns Some((base_expr, vbase_field, method_name)) if it is.
    fn get_virtual_base_method_call_info(
        &self,
        node: &ClangNode,
    ) -> Option<(String, String, String)> {
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

        if let ClangNodeKind::MemberExpr {
            member_name,
            declaring_class,
            is_static,
            ..
        } = &member_node.kind
        {
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
    /// Uses const-compatible initialization for use in static variables.
    fn default_value_for_type(ty: &CppType) -> String {
        match ty {
            CppType::Int { .. }
            | CppType::Long { .. }
            | CppType::Short { .. }
            | CppType::Char { .. }
            | CppType::LongLong { .. } => "0".to_string(),
            CppType::Float => "0.0f32".to_string(),
            CppType::Double => "0.0f64".to_string(),
            CppType::Bool => "false".to_string(),
            CppType::Pointer { .. } => "std::ptr::null_mut()".to_string(),
            CppType::Array { element, size } => {
                // For arrays of non-primitive types, use zeroed() for the whole array
                // since [zeroed(); N] requires Copy but zeroed() for [T; N] works directly
                if let Some(n) = size {
                    match element.as_ref() {
                        CppType::Int { .. }
                        | CppType::Long { .. }
                        | CppType::Short { .. }
                        | CppType::Char { .. }
                        | CppType::LongLong { .. } => {
                            format!("[0; {}]", n)
                        }
                        CppType::Float => format!("[0.0f32; {}]", n),
                        CppType::Double => format!("[0.0f64; {}]", n),
                        CppType::Bool => format!("[false; {}]", n),
                        CppType::Pointer { .. } => {
                            format!("[std::ptr::null_mut(); {}]", n)
                        }
                        // For struct arrays and other non-Copy types, zero the entire array
                        _ => "unsafe { std::mem::zeroed() }".to_string(),
                    }
                } else {
                    "[]".to_string()
                }
            }
            // For named types (structs) and references, use zeroed memory which is const-compatible
            CppType::Named(_) | CppType::Reference { .. } => {
                "unsafe { std::mem::zeroed() }".to_string()
            }
            _ => "unsafe { std::mem::zeroed() }".to_string(),
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
                        let right = if i + 1 < node.children.len() {
                            i + 1
                        } else {
                            i
                        };
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
            if let ClangNodeKind::MemberExpr {
                member_name,
                is_arrow,
                ..
            } = &child.kind
            {
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
            if let ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } = &child.kind
            {
                if !child.children.is_empty() {
                    return self.get_explicit_destructor_call_inner(&child.children[0]);
                }
            }
        }
        None
    }

    /// Helper for get_explicit_destructor_call that checks inner nodes.
    fn get_explicit_destructor_call_inner(&self, node: &ClangNode) -> Option<String> {
        if let ClangNodeKind::MemberExpr {
            member_name,
            is_arrow,
            ..
        } = &node.kind
        {
            if member_name.starts_with('~') && !node.children.is_empty() {
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
        None
    }

    /// Check if a node is a dereference of a pointer (like *ptr or (*ptr)).
    /// Returns the pointer expression if so.
    fn get_deref_pointer(node: &ClangNode) -> Option<&ClangNode> {
        match &node.kind {
            ClangNodeKind::UnaryOperator {
                op: UnaryOp::Deref, ..
            } => {
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
            ClangNodeKind::MemberExpr { ty, .. } => {
                // MemberExpr with "<bound member function type>" is a method reference
                // which is used as a function in member call expressions (e.g., v.size())
                if let CppType::Named(name) = ty {
                    name.contains("bound member function type")
                } else {
                    false
                }
            }
            ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } => {
                // Look through wrapper nodes
                node.children.iter().any(Self::is_function_reference)
            }
            _ => false,
        }
    }

    /// Strip `Some(...)` wrapper from a string if present.
    /// Used for function call callees where FunctionToPointerDecay shouldn't wrap.
    fn strip_some_wrapper(s: &str) -> String {
        if s.starts_with("Some(") && s.ends_with(")") {
            // Extract inner part
            s[5..s.len() - 1].to_string()
        } else {
            s.to_string()
        }
    }

    /// Check if a node is a function pointer variable (not a direct function reference).
    /// Returns true if the node has type Pointer { pointee: Function { .. } }
    /// or a Named type that is a typedef to a function pointer
    fn is_function_pointer_variable(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { ty, .. } => Self::is_function_pointer_type_or_typedef(ty),
            ClangNodeKind::Unknown(_) | ClangNodeKind::ImplicitCastExpr { .. } => {
                // Look through wrapper nodes (but not FunctionToPointerDecay)
                node.children.iter().any(Self::is_function_pointer_variable)
            }
            _ => false,
        }
    }

    /// Check if a type is a function pointer or a typedef that resolves to one
    fn is_function_pointer_type_or_typedef(ty: &CppType) -> bool {
        match ty {
            CppType::Pointer { pointee, .. } => {
                matches!(pointee.as_ref(), CppType::Function { .. })
            }
            CppType::Named(name) => {
                // Check for common function pointer typedef patterns
                // In C++, typedef void (*Handler)(int) creates a named type
                // We also need to handle typedefs from our own generation
                // where we generate Option<fn(...)> for function pointers
                // These will typically be all uppercase or PascalCase names
                // that aren't primitive types
                !matches!(
                    name.as_str(),
                    "bool"
                        | "char"
                        | "int"
                        | "long"
                        | "short"
                        | "float"
                        | "double"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "f32"
                        | "f64"
                        | "isize"
                        | "usize"
                        | "size_t"
                        | "ptrdiff_t"
                        | "intptr_t"
                        | "uintptr_t"
                ) && (
                    // Check if name ends with common function pointer typedef conventions
                    name.ends_with("Fn") ||
                    name.ends_with("Func") ||
                    name.ends_with("Handler") ||
                    name.ends_with("Callback") ||
                    name.ends_with("Ptr") ||
                    name.ends_with("Op") ||
                    // Or is a PascalCase name that could be a function pointer typedef
                    name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                )
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
                node.children.iter().any(Self::is_nullptr_literal)
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
            ClangNodeKind::DeclRefExpr {
                name,
                namespace_path,
                ..
            } => {
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
                    if (op_name == "operator<<" || op_name == "operator>>")
                        && !node.children.is_empty()
                        && left_idx < node.children.len()
                    {
                        return Self::get_io_stream_type(&node.children[left_idx]);
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
            ClangNodeKind::DeclRefExpr {
                name,
                namespace_path,
                ..
            } => {
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
                node.children.iter().any(Self::contains_typeid_expr)
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
                        if let Some(stream_type) =
                            Self::get_io_stream_type(&node.children[left_idx])
                        {
                            // Base case: stream << arg
                            return Some((stream_type, vec![&node.children[right_idx]]));
                        }
                        // Recursive case: (stream << ...) << arg
                        // Check if left operand is another operator<< on a stream
                        if let Some((stream_type, mut args)) =
                            self.collect_stream_output_args(&node.children[left_idx])
                        {
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
        let has_newline = args
            .last()
            .is_some_and(|arg| Self::is_stream_manipulator(arg) == Some("newline"));

        // Filter out endl/flush manipulators, collect format args
        let format_args: Vec<String> = args
            .iter()
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
                format!(
                    "writeln!({}, \"{}\", {}).unwrap()",
                    stream_expr, format_str, args_str
                )
            } else {
                format!(
                    "write!({}, \"{}\", {}).unwrap()",
                    stream_expr, format_str, args_str
                )
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
                        if let Some(stream_type) =
                            Self::get_io_stream_type(&node.children[left_idx])
                        {
                            if stream_type == "stdin" {
                                // Base case: stream >> arg
                                return Some((stream_type, vec![&node.children[right_idx]]));
                            }
                        }
                        // Recursive case: (stream >> ...) >> arg
                        if let Some((stream_type, mut args)) =
                            self.collect_stream_input_args(&node.children[left_idx])
                        {
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
        let var_reads: Vec<String> = args
            .iter()
            .map(|arg| {
                let var_name = self.expr_to_string(arg);
                let var_type = Self::get_expr_type(arg);

                // Generate appropriate parse call based on type
                let parse_expr = match var_type {
                    Some(CppType::Int { signed: true }) => {
                        "__parts.next().unwrap().parse::<i32>().unwrap()".to_string()
                    }
                    Some(CppType::Int { signed: false }) => {
                        "__parts.next().unwrap().parse::<u32>().unwrap()".to_string()
                    }
                    Some(CppType::Long { signed: true })
                    | Some(CppType::LongLong { signed: true }) => {
                        "__parts.next().unwrap().parse::<i64>().unwrap()".to_string()
                    }
                    Some(CppType::Long { signed: false })
                    | Some(CppType::LongLong { signed: false }) => {
                        "__parts.next().unwrap().parse::<u64>().unwrap()".to_string()
                    }
                    Some(CppType::Short { signed: true }) => {
                        "__parts.next().unwrap().parse::<i16>().unwrap()".to_string()
                    }
                    Some(CppType::Short { signed: false }) => {
                        "__parts.next().unwrap().parse::<u16>().unwrap()".to_string()
                    }
                    Some(CppType::Float) => {
                        "__parts.next().unwrap().parse::<f32>().unwrap()".to_string()
                    }
                    Some(CppType::Double) => {
                        "__parts.next().unwrap().parse::<f64>().unwrap()".to_string()
                    }
                    Some(CppType::Char { signed: true }) => {
                        "__parts.next().unwrap().chars().next().unwrap() as i8".to_string()
                    }
                    Some(CppType::Char { signed: false }) => {
                        "__parts.next().unwrap().chars().next().unwrap() as u8".to_string()
                    }
                    Some(CppType::Bool) => {
                        "__parts.next().unwrap().parse::<bool>().unwrap()".to_string()
                    }
                    Some(CppType::Named(ref name)) if name == "std::string" || name == "string" => {
                        "__parts.next().unwrap().to_string()".to_string()
                    }
                    _ => "__parts.next().unwrap().to_string()".to_string(),
                };

                format!("{} = {}", var_name, parse_expr)
            })
            .collect();

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
            ClangNodeKind::CXXMethodDecl {
                name,
                return_type,
                params,
                is_static,
                is_const,
                ..
            } => {
                // If the C++ method is marked const, use &self
                // Otherwise, use &mut self (non-const methods can potentially mutate)
                let returns_mut_ref = matches!(
                    return_type,
                    CppType::Reference {
                        is_const: false,
                        ..
                    }
                );
                // Iterator operators always modify self (increment/decrement)
                let is_iterator_mutating_op = matches!(name.as_str(), "operator++" | "operator--");
                // Non-const methods should use &mut self
                let is_mutable_method = !*is_const || returns_mut_ref || is_iterator_mutating_op;

                let self_param = if *is_static {
                    "".to_string()
                } else if is_mutable_method {
                    "&mut self, ".to_string()
                } else {
                    "&self, ".to_string()
                };

                // Collect parameters that are assigned to within the method body
                // C++ allows modifying by-value params, but Rust requires `mut`
                let assigned_params = Self::collect_assigned_params(node, params);

                // Deduplicate parameter names (C++ allows unnamed params, Rust doesn't)
                let mut param_name_counts: HashMap<String, usize> = HashMap::new();
                let params_str = params
                    .iter()
                    .map(|(n, t)| {
                        let mut param_name = sanitize_identifier(n);
                        // If this parameter name has been seen before, add a suffix
                        let count = param_name_counts.entry(param_name.clone()).or_insert(0);
                        if *count > 0 {
                            param_name = format!("{}_{}", param_name, *count);
                        }
                        *param_name_counts.get_mut(&sanitize_identifier(n)).unwrap() += 1;
                        // Add `mut` if this parameter is assigned to in the body
                        let mut_prefix = if assigned_params.contains(n) {
                            "mut "
                        } else {
                            ""
                        };
                        format!("{}{}: {}", mut_prefix, param_name, t.to_rust_type_str())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                // Determine return type, fixing c_void placeholders for methods returning *this
                let rust_return_type = return_type.to_rust_type_str();
                // Check if this is an iterator operator that should return Self
                let is_iterator_value_return_op =
                    matches!(name.as_str(), "operator++" | "operator--" | "_M_const_cast");
                // Compound assignment operators should return &mut Self
                let is_iterator_ref_return_op = matches!(
                    name.as_str(),
                    "operator+="
                        | "operator-="
                        | "operator*="
                        | "operator/="
                        | "operator%="
                        | "operator&="
                        | "operator|="
                        | "operator^="
                        | "operator<<="
                        | "operator>>="
                );
                let ret_str = if *return_type == CppType::Void {
                    String::new()
                } else if (rust_return_type.contains("c_void") || rust_return_type == "*mut ()")
                    && is_iterator_ref_return_op
                {
                    // Compound assignment operators return &mut Self
                    " -> &mut Self".to_string()
                } else if (rust_return_type.contains("c_void") || rust_return_type == "*mut ()")
                    && (Self::method_returns_this_only(node) || is_iterator_value_return_op)
                {
                    // Method returns *this or is an iterator operator - use Self
                    // Post-increment (params.len() == 1) returns by value
                    // Pre-increment (params.len() == 0) returns by mutable reference
                    if params.is_empty() && (returns_mut_ref || is_mutable_method) {
                        " -> &mut Self".to_string()
                    } else {
                        " -> Self".to_string()
                    }
                } else {
                    format!(" -> {}", Self::sanitize_return_type(&rust_return_type))
                };

                // Special handling for operators that have const/non-const overloads
                // Skip the const version of operator* - only generate the mutable one
                // Note: operator-> always returns a pointer (not reference), so we don't skip it
                let skip_method = name == "operator*" && params.is_empty() && !is_mutable_method;

                if skip_method {
                    self.current_class = old_class;
                    return;
                }

                let base_method_name = if name == "operator*" && params.is_empty() {
                    // Unary dereference operator (mutable version only)
                    "op_deref".to_string()
                } else if name == "operator->" {
                    // Arrow operator (mutable version only)
                    "op_arrow".to_string()
                } else {
                    sanitize_identifier(name)
                };

                // Handle method overloading by appending suffix for duplicates
                let count = self
                    .current_struct_methods
                    .entry(base_method_name.clone())
                    .or_insert(0);
                let method_name = if *count == 0 {
                    *count += 1;
                    base_method_name
                } else {
                    *count += 1;
                    format!("{}_{}", base_method_name, *count - 1)
                };

                self.writeln(&format!(
                    "pub fn {}({}{}){} {{",
                    method_name, self_param, params_str, ret_str
                ));
                self.indent += 1;

                // Track return type for reference return handling
                let old_return_type = self.current_return_type.take();
                self.current_return_type = Some(return_type.clone());

                // Track reference, pointer, and array parameters for proper dereferencing
                let saved_ref_vars = self.ref_vars.clone();
                let saved_ptr_vars = self.ptr_vars.clone();
                let saved_arr_vars = self.arr_vars.clone();
                self.ref_vars.clear();
                self.ptr_vars.clear();
                self.arr_vars.clear();
                for (param_name, param_type) in params {
                    if matches!(param_type, CppType::Reference { .. }) {
                        self.ref_vars.insert(param_name.clone());
                    }
                    if matches!(param_type, CppType::Pointer { .. })
                        || matches!(param_type, CppType::Array { size: None, .. })
                    {
                        self.ptr_vars.insert(param_name.clone());
                    }
                    if matches!(param_type, CppType::Array { .. }) {
                        self.arr_vars.insert(param_name.clone());
                    }
                }

                // Find body
                for child in &node.children {
                    if let ClangNodeKind::CompoundStmt = &child.kind {
                        self.generate_block_contents(&child.children, return_type);
                    }
                }

                // Restore saved state
                self.ref_vars = saved_ref_vars;
                self.ptr_vars = saved_ptr_vars;
                self.arr_vars = saved_arr_vars;

                self.current_return_type = old_return_type;
                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }
            ClangNodeKind::ConstructorDecl { params, .. } => {
                // Base name uses new_N format where N is param count
                let base_fn_name = format!("new_{}", params.len());

                // Handle constructor overloading (same param count, different types)
                let count = self
                    .current_struct_methods
                    .entry(base_fn_name.clone())
                    .or_insert(0);
                let fn_name = if *count == 0 {
                    *count += 1;
                    base_fn_name.clone()
                } else {
                    *count += 1;
                    format!("{}_{}", base_fn_name, *count - 1)
                };
                let internal_name = format!("__new_without_vbases_{}", params.len());

                // Record constructor signature for base class initializer generation
                let param_types: Vec<CppType> = params.iter().map(|(_, t)| t.clone()).collect();
                self.constructor_signatures
                    .entry(struct_name.to_string())
                    .or_default()
                    .push((fn_name.clone(), param_types));

                // Deduplicate parameter names (C++ allows unnamed params, Rust doesn't)
                let mut param_name_counts: HashMap<String, usize> = HashMap::new();
                let mut deduped_params: Vec<String> = Vec::new();
                let params_str = params
                    .iter()
                    .map(|(n, t)| {
                        let mut param_name = sanitize_identifier(n);
                        let count = param_name_counts.entry(param_name.clone()).or_insert(0);
                        if *count > 0 {
                            param_name = format!("{}_{}", param_name, *count);
                        }
                        *param_name_counts.get_mut(&sanitize_identifier(n)).unwrap() += 1;
                        deduped_params.push(param_name.clone());
                        format!("{}: {}", param_name, t.to_rust_type_str())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let params_names = deduped_params.join(", ");

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
                let base_classes = self
                    .current_class
                    .as_ref()
                    .and_then(|c| self.class_bases.get(c))
                    .cloned()
                    .unwrap_or_default();

                let mut i = 0;
                while i < node.children.len() {
                    if let ClangNodeKind::MemberRef { name } = &node.children[i].kind {
                        // Next sibling should be the initializer expression
                        let init_val = if i + 1 < node.children.len() {
                            i += 1;
                            // Skip literal suffixes - Rust will infer the type from struct field
                            self.skip_literal_suffix = true;
                            let val = self.expr_to_string(&node.children[i]);
                            self.skip_literal_suffix = false;
                            val
                        } else {
                            "Default::default()".to_string()
                        };
                        initializers.push((name.clone(), init_val));
                    } else if let ClangNodeKind::Unknown(s) = &node.children[i].kind {
                        // Check for TypeRef:ClassName pattern indicating base class initializer
                        if let Some(base_class_cpp) = s.strip_prefix("TypeRef:") {
                            // Convert C++ type name to Rust struct name
                            // Strip namespace prefix to match struct definition naming
                            // (struct _Bit_iterator_base is defined without std:: prefix)
                            let base_class_unqual =
                                if let Some(last_colon_pos) = base_class_cpp.rfind("::") {
                                    &base_class_cpp[last_colon_pos + 2..]
                                } else {
                                    base_class_cpp
                                };
                            let base_class = sanitize_identifier(base_class_unqual);
                            // Next sibling should be constructor call
                            if i + 1 < node.children.len() {
                                i += 1;
                                // Check if next is a CallExpr
                                if matches!(&node.children[i].kind, ClangNodeKind::CallExpr { .. })
                                {
                                    // Extract constructor arguments
                                    let args = self.extract_constructor_args(&node.children[i]);

                                    // Look up constructor signature to correct 0 -> null_mut() for pointer params
                                    let ctor_name_lookup = format!("new_{}", args.len());
                                    let corrected_args: Vec<String> = if let Some(ctors) =
                                        self.constructor_signatures.get(&base_class)
                                    {
                                        // Find the matching constructor by name
                                        if let Some((_, param_types)) =
                                            ctors.iter().find(|(name, _)| *name == ctor_name_lookup)
                                        {
                                            args.iter()
                                                .zip(param_types.iter())
                                                .map(|(arg, ty)| {
                                                    correct_initializer_for_type(arg, ty)
                                                })
                                                .collect()
                                        } else {
                                            args.clone()
                                        }
                                    } else {
                                        args.clone()
                                    };

                                    let ctor_call = format!(
                                        "{}::new_{}({})",
                                        base_class,
                                        args.len(),
                                        corrected_args.join(", ")
                                    );

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
                                            let base_has_vbases =
                                                self.class_has_virtual_bases(&info.name);
                                            let ctor_name = if base_has_vbases {
                                                format!(
                                                    "{}::__new_without_vbases_{}",
                                                    info.name,
                                                    corrected_args.len()
                                                )
                                            } else {
                                                format!(
                                                    "{}::new_{}",
                                                    info.name,
                                                    corrected_args.len()
                                                )
                                            };
                                            let ctor_call = format!(
                                                "{}({})",
                                                ctor_name,
                                                corrected_args.join(", ")
                                            );
                                            let field_name = if non_virtual_idx == 0 {
                                                "__base".to_string()
                                            } else {
                                                format!("__base{}", non_virtual_idx)
                                            };
                                            base_inits.push((field_name, ctor_call));
                                        }
                                    } else {
                                        // Check if this is a transitive virtual base (not a direct base)
                                        let is_transitive_vbase = self
                                            .current_class
                                            .as_ref()
                                            .and_then(|c| self.virtual_bases.get(c))
                                            .map(|vbases| vbases.contains(&base_class))
                                            .unwrap_or(false);

                                        if is_transitive_vbase {
                                            // This is a virtual base initializer (e.g., A(v) in D::D() : A(v), B(v), C(v))
                                            virtual_base_inits
                                                .push((base_class.to_string(), ctor_call));
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
                        Self::extract_member_assignments(
                            &node.children[i],
                            &mut initializers,
                            self,
                        );
                        // Store compound stmt for later - non-member statements will be generated after Self {} literal
                        ctor_compound_stmt = Some(i);
                    }
                    i += 1;
                }

                let class_has_vbases = self.class_has_virtual_bases(struct_name);

                if class_has_vbases {
                    // Internal constructor that does not allocate virtual bases
                    self.writeln(&format!(
                        "pub(crate) fn {}({}) -> Self {{",
                        internal_name, params_str
                    ));
                    self.indent += 1;
                    self.writeln("Self {");
                    self.indent += 1;

                    let mut initialized_vbase: std::collections::HashSet<String> =
                        std::collections::HashSet::new();

                    for (field_name, base_call) in &base_inits {
                        self.writeln(&format!("{}: {},", field_name, base_call));
                        initialized_vbase.insert(field_name.clone());
                    }

                    // Initialize vtable pointer for ROOT polymorphic classes
                    if let Some(vtable_info) = self.vtables.get(struct_name).cloned() {
                        if vtable_info.base_class.is_none() {
                            let sanitized = sanitize_identifier(struct_name);
                            self.writeln(&format!(
                                "__vtable: &{}_VTABLE,",
                                sanitized.to_uppercase()
                            ));
                            initialized_vbase.insert("__vtable".to_string());
                        }
                    }

                    let vbases_internal = self
                        .virtual_bases
                        .get(struct_name)
                        .cloned()
                        .unwrap_or_default();
                    for vb in &vbases_internal {
                        let field = self.virtual_base_field_name(vb);
                        let storage = self.virtual_base_storage_field_name(vb);
                        self.writeln(&format!("{}: std::ptr::null_mut(),", field));
                        self.writeln(&format!("{}: None,", storage));
                        initialized_vbase.insert(field);
                        initialized_vbase.insert(storage);
                    }
                    // Get field info for type-aware initialization
                    let all_fields_vbase = self
                        .class_fields
                        .get(struct_name)
                        .cloned()
                        .unwrap_or_default();
                    for (field, value) in &initializers {
                        let sanitized = sanitize_identifier(field);
                        // Correct initializer value based on field type (e.g., 0 -> null_mut() for pointers)
                        let corrected = all_fields_vbase
                            .iter()
                            .find(|(name, _)| name == &sanitized)
                            .map(|(_, ty)| correct_initializer_for_type(value, ty))
                            .unwrap_or_else(|| value.clone());
                        self.writeln(&format!("{}: {},", sanitized, corrected));
                        initialized_vbase.insert(sanitized);
                    }

                    // Generate default values for uninitialized fields
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
                    self.writeln(&format!(
                        "let mut __self = Self::{}({});",
                        internal_name, params_names
                    ));

                    let vbases_public = self
                        .virtual_bases
                        .get(struct_name)
                        .cloned()
                        .unwrap_or_default();
                    for vb in &vbases_public {
                        let ctor = if let Some((_, call)) =
                            virtual_base_inits.iter().find(|(name, _)| name == vb)
                        {
                            call.clone()
                        } else {
                            format!("{}::new_0()", vb)
                        };
                        let vb_field = self.virtual_base_field_name(vb);
                        let vb_storage = self.virtual_base_storage_field_name(vb);
                        let temp_name = format!("__vb_{}", vb_field.trim_start_matches("__vbase_"));
                        self.writeln(&format!("let mut {} = Box::new({});", temp_name, ctor));
                        self.writeln(&format!(
                            "let {}_ptr = {}.as_mut() as *mut {};",
                            temp_name, temp_name, vb
                        ));
                        self.writeln(&format!("__self.{} = {}_ptr;", vb_field, temp_name));
                        self.writeln(&format!("__self.{} = Some({});", vb_storage, temp_name));
                    }

                    // Propagate virtual base pointers into embedded bases that need them
                    let mut non_virtual_idx = 0;
                    for base in &base_classes {
                        if !base.is_virtual {
                            if self.class_has_virtual_bases(&base.name) {
                                let base_field = if non_virtual_idx == 0 {
                                    "__base".to_string()
                                } else {
                                    format!("__base{}", non_virtual_idx)
                                };
                                let base_vbases = self
                                    .virtual_bases
                                    .get(&base.name)
                                    .cloned()
                                    .unwrap_or_default();
                                for vb in &base_vbases {
                                    let vb_field = self.virtual_base_field_name(vb);
                                    self.writeln(&format!(
                                        "__self.{}.{} = __self.{};",
                                        base_field, vb_field, vb_field
                                    ));
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

                    // Check if this is a derived polymorphic class that needs vtable set after construction
                    // Abstract classes don't have vtable instances, so skip vtable assignment
                    let is_derived_polymorphic = self
                        .vtables
                        .get(struct_name)
                        .map(|v| v.base_class.is_some() && !v.is_abstract)
                        .unwrap_or(false);

                    // Use __self pattern if we need to do post-construction work
                    let needs_self_pattern = has_non_member_stmts || is_derived_polymorphic;

                    self.writeln(&format!("pub fn {}({}) -> Self {{", fn_name, params_str));
                    self.indent += 1;

                    if needs_self_pattern {
                        // Need to run statements after construction, so use let + return pattern
                        self.writeln("let mut __self = Self {");
                    } else {
                        self.writeln("Self {");
                    }
                    self.indent += 1;

                    // Collect initialized field names
                    let mut initialized: std::collections::HashSet<String> =
                        std::collections::HashSet::new();

                    // Generate base class initializers
                    for (field_name, base_call) in &base_inits {
                        self.writeln(&format!("{}: {},", field_name, base_call));
                        initialized.insert(field_name.clone());
                    }

                    // Initialize vtable pointer for ROOT polymorphic classes
                    // (Derived classes get vtable pointer through __base)
                    if let Some(vtable_info) = self.vtables.get(struct_name).cloned() {
                        if vtable_info.base_class.is_none() {
                            // This is a root polymorphic class - set vtable pointer
                            let sanitized = sanitize_identifier(struct_name);
                            self.writeln(&format!(
                                "__vtable: &{}_VTABLE,",
                                sanitized.to_uppercase()
                            ));
                            initialized.insert("__vtable".to_string());
                        }
                    }

                    // Get field info for type-aware initialization
                    let all_fields = self
                        .class_fields
                        .get(struct_name)
                        .cloned()
                        .unwrap_or_default();
                    // Generate field initializers
                    for (field, value) in &initializers {
                        let sanitized = sanitize_identifier(field);
                        // Correct initializer value based on field type (e.g., 0 -> null_mut() for pointers)
                        let corrected = all_fields
                            .iter()
                            .find(|(name, _)| name == &sanitized)
                            .map(|(_, ty)| correct_initializer_for_type(value, ty))
                            .unwrap_or_else(|| value.clone());
                        self.writeln(&format!("{}: {},", sanitized, corrected));
                        initialized.insert(sanitized);
                    }

                    // Generate default values for uninitialized fields
                    // This avoids using ..Default::default() which can cause issues with Drop
                    for (field_name, field_type) in &all_fields {
                        if !initialized.contains(field_name) {
                            let default_val = default_value_for_type(field_type);
                            self.writeln(&format!("{}: {},", field_name, default_val));
                        }
                    }

                    self.indent -= 1;

                    if needs_self_pattern {
                        self.writeln("};");

                        // Set vtable pointer for derived polymorphic classes
                        // The base constructor set base's vtable, we need to override it
                        if is_derived_polymorphic {
                            let sanitized = sanitize_identifier(struct_name);
                            // Find the path to __vtable through inheritance chain
                            // For deep inheritance, this could be __base.__base.__vtable etc.
                            let vtable_path = self.compute_vtable_access_path(struct_name);
                            self.writeln(&format!(
                                "__self.{}.__vtable = &{}_VTABLE;",
                                vtable_path,
                                sanitized.to_uppercase()
                            ));
                        }

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
                        // For arrays, prefer InitListExpr over IntegerLiteral (which is the array size)
                        let initializer = if is_array {
                            // For arrays, look specifically for InitListExpr
                            child.children.iter().find(|c| {
                                matches!(&c.kind, ClangNodeKind::InitListExpr { .. })
                            }).or_else(|| {
                                // Fall back to other expressions (CXXConstructExpr, etc.)
                                child.children.iter().find(|c| {
                                    !matches!(&c.kind, ClangNodeKind::Unknown(s) if s == "TypeRef")
                                        && !matches!(&c.kind, ClangNodeKind::Unknown(s) if s.contains("Type"))
                                        && !matches!(&c.kind, ClangNodeKind::IntegerLiteral { .. }) // Skip array size literal
                                        && !matches!(&c.kind, ClangNodeKind::ParmVarDecl { .. })
                                })
                            })
                        } else {
                            child.children.iter().find(|c| {
                                !matches!(&c.kind, ClangNodeKind::Unknown(s) if s == "TypeRef")
                                    && !matches!(&c.kind, ClangNodeKind::Unknown(s) if s.contains("Type"))
                                    && !matches!(&c.kind, ClangNodeKind::Unknown(s) if s == "NamespaceRef")
                                    && !matches!(&c.kind, ClangNodeKind::Unknown(s) if s == "TemplateRef")
                                    && !matches!(&c.kind, ClangNodeKind::ParmVarDecl { .. })
                            })
                        };

                        // Check if we have a real initializer
                        let has_real_init = initializer.is_some();

                        let init = if has_real_init {
                            let init_node = initializer.unwrap();
                            // Special case: function pointer initialized with nullptr → None
                            if Self::is_function_pointer_type(ty)
                                && Self::is_nullptr_literal(init_node)
                            {
                                " = None".to_string()
                            } else {
                                // Skip type suffixes for literals when we have explicit type annotation
                                self.skip_literal_suffix = true;
                                let expr = self.expr_to_string(init_node);
                                self.skip_literal_suffix = false;
                                // If expression is unsupported or errored, fall back to default
                                // Common error patterns: "unsupported", "/* call error */"
                                if expr.contains("unsupported") || expr.contains("/* call error */")
                                {
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
                                    let value_node = Self::find_variant_init_value(init_node)
                                        .unwrap_or(init_node);
                                    let value_expr = self.expr_to_string(value_node);
                                    // Try to determine the initializer type
                                    if let Some(init_type) = Self::get_expr_type(value_node) {
                                        if let Some(idx) =
                                            Self::find_variant_index(&variant_args, &init_type)
                                        {
                                            format!(" = {}::V{}({})", enum_name, idx, value_expr)
                                        } else {
                                            // Couldn't match type to variant, use V0 as fallback
                                            format!(" = {}::V0({})", enum_name, value_expr)
                                        }
                                    } else {
                                        // Couldn't determine init type, use V0 as fallback
                                        format!(" = {}::V0({})", enum_name, value_expr)
                                    }
                                } else if let CppType::Named(_) = ty {
                                    // Check if this is a Named type with "0" initializer,
                                    // which indicates a CXXConstructExpr that couldn't be parsed
                                    let rust_type = ty.to_rust_type_str();
                                    // Only generate constructor for actual struct types, not primitives
                                    // that might have been mapped from C++ types
                                    let is_primitive = matches!(
                                        rust_type.as_str(),
                                        "usize"
                                            | "isize"
                                            | "i8"
                                            | "i16"
                                            | "i32"
                                            | "i64"
                                            | "i128"
                                            | "u8"
                                            | "u16"
                                            | "u32"
                                            | "u64"
                                            | "u128"
                                            | "f32"
                                            | "f64"
                                            | "bool"
                                            | "()"
                                            | "char"
                                    ) || rust_type.starts_with('*')
                                        || rust_type.starts_with('&');
                                    if (expr == "0" || expr == "_unnamed") && !is_primitive {
                                        // Use unsafe zeroed for:
                                        // - "0" placeholder from unresolved CXXConstructExpr
                                        // - "_unnamed" placeholder from unresolved expression
                                        // - template types (contain __) since they may not have new_0 or Default impl
                                        if rust_type.contains("__") || expr == "_unnamed" {
                                            " = unsafe { std::mem::zeroed() }".to_string()
                                        } else {
                                            format!(" = {}::new_0()", rust_type)
                                        }
                                    } else {
                                        format!(" = {}", expr)
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

                        // Fix c_void placeholder types for variables initialized with self/*this
                        let rust_type = ty.to_rust_type_str();
                        let (final_type, final_init) = if rust_type.contains("c_void")
                            && has_real_init
                            && Self::expr_is_this(initializer.unwrap())
                        {
                            // Variable is initialized with *this, use Self and clone
                            ("Self".to_string(), " = self.clone()".to_string())
                        } else {
                            (rust_type, init)
                        };

                        self.writeln(&format!(
                            "let {}{}: {}{};",
                            mut_kw,
                            sanitize_identifier(name),
                            final_type,
                            final_init
                        ));
                    }
                }
            }
            ClangNodeKind::ReturnStmt => {
                if node.children.is_empty() {
                    self.writeln("return;");
                } else {
                    // Skip literal suffixes - Rust will infer type from return type
                    let prev_skip = self.skip_literal_suffix;
                    self.skip_literal_suffix = true;
                    let expr = self.expr_to_string(&node.children[0]);
                    self.skip_literal_suffix = prev_skip;
                    // Check if we need to add &mut for reference return types
                    let expr = if let Some(CppType::Reference { is_const, .. }) =
                        &self.current_return_type
                    {
                        // Don't add & or &mut if returning 'self' (from *this in C++)
                        // because Rust's &mut self already provides the reference
                        if expr == "self" || expr == "__self" {
                            expr
                        } else if expr.contains(".op_assign(")
                            || expr.contains(".op_add_assign(")
                            || expr.contains(".op_sub_assign(")
                            || expr.contains(".op_mul_assign(")
                            || expr.contains(".op_div_assign(")
                            || expr.contains(".op_rem_assign(")
                        {
                            // Assignment operator overloads already return &mut Self
                            // Don't add another &mut
                            expr
                        } else if expr.starts_with("unsafe { ") && expr.ends_with(" }") {
                            // If expression is an unsafe block like "unsafe { *ptr }",
                            // put the & or &mut inside: "unsafe { &mut *ptr }"
                            let inner = &expr[9..expr.len() - 2]; // Extract content between "unsafe { " and " }"
                            let prefix = if *is_const { "&" } else { "&mut " };
                            format!("unsafe {{ {}{} }}", prefix, inner)
                        } else if *is_const {
                            format!("&{}", expr)
                        } else {
                            format!("&mut {}", expr)
                        }
                    } else if (expr == "self" || expr == "__self")
                        && Self::expr_is_this(&node.children[0])
                    {
                        // Returning *this by value - need to clone since self is a reference
                        format!("{}.clone()", expr)
                    } else if expr == "0"
                        && matches!(
                            self.current_return_type,
                            Some(CppType::Pointer { .. })
                        )
                    {
                        // In C++, returning 0 or NULL for a pointer type means return null pointer
                        "std::ptr::null()".to_string()
                    } else {
                        // Check if we need to add a cast for primitive integer return types
                        // This handles cases like `return *__c;` where __c is u32 but return type is i32
                        let expr_type = Self::get_expr_type(&node.children[0]);
                        let int_primitives = ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "isize", "usize"];

                        let ret_rust_type = self
                            .current_return_type
                            .as_ref()
                            .map(|t| t.to_rust_type_str());
                        let expr_rust_type = expr_type.as_ref().map(|t| t.to_rust_type_str());

                        let ret_is_int =
                            ret_rust_type.as_ref().map_or(false, |t| int_primitives.contains(&t.as_str()));
                        let expr_is_int =
                            expr_rust_type.as_ref().map_or(false, |t| int_primitives.contains(&t.as_str()));

                        // Add cast if both are integer primitives but different types
                        // Also handle case where expr type is unknown but return type is int and expr is a deref
                        let needs_explicit_cast = ret_is_int && expr_is_int && ret_rust_type != expr_rust_type;

                        // Handle case where expression type is unknown or known but not detected as int
                        // We're returning from an int function and the expression is a simple dereference
                        // The expr might be "*__c" but also handle "(*__c)" and similar patterns
                        let is_deref_expr = expr.starts_with('*') || expr.starts_with("(*");
                        let is_comparison_expr =
                            expr.contains("==") || expr.contains("!=") || expr.contains('<') || expr.contains('>');

                        // Unconditional cast for deref expressions returning integers
                        // This handles wint_t (u32) -> wchar_t (i32) and similar conversions
                        let needs_deref_cast = ret_is_int
                            && is_deref_expr
                            && !expr.contains(" as ")
                            && !is_comparison_expr;

                        // Handle int-to-bool conversion (C++ truthy semantics)
                        let ret_is_bool = ret_rust_type.as_ref().map_or(false, |t| t == "bool");
                        let needs_int_to_bool = ret_is_bool && expr_is_int;

                        if needs_int_to_bool {
                            // Convert integer to bool: non-zero = true
                            format!("({}) != 0", expr)
                        } else if needs_explicit_cast || needs_deref_cast {
                            if let Some(rust_type) = ret_rust_type {
                                format!("{} as {}", expr, rust_type)
                            } else {
                                expr
                            }
                        } else {
                            expr
                        }
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
                    self.writeln(
                        "match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {",
                    );
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
                // Skip "_unnamed" placeholder expressions (from unresolved AST nodes)
                if expr == "_unnamed" {
                    self.writeln("// unresolved expression");
                } else if is_tail_expr {
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
            // In C++, pointers and integers can be used in boolean context
            // Pointers: non-null = true; Integers: non-zero = true
            // In Rust, we need explicit checks
            let cond_type = Self::get_expr_type(&node.children[0]);
            let cond = if matches!(cond_type, Some(CppType::Pointer { .. })) {
                format!("!{}.is_null()", cond)
            } else if matches!(
                cond_type,
                Some(CppType::Int { .. })
                    | Some(CppType::Short { .. })
                    | Some(CppType::Long { .. })
                    | Some(CppType::LongLong { .. })
                    | Some(CppType::Char { .. })
            ) {
                // Integer in boolean context: non-zero = true
                format!("({}) != 0", cond)
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

    /// Find a DeclStmt that might be wrapped in ImplicitCastExpr or Unknown nodes.
    /// This is needed for while loop conditions like: while (int x = expr)
    fn find_decl_stmt_in_condition(node: &ClangNode) -> Option<&ClangNode> {
        match &node.kind {
            ClangNodeKind::DeclStmt => Some(node),
            ClangNodeKind::ImplicitCastExpr { .. }
            | ClangNodeKind::Unknown(_)
            | ClangNodeKind::ParenExpr { .. } => {
                // Look through wrapper nodes
                for child in &node.children {
                    if let Some(decl) = Self::find_decl_stmt_in_condition(child) {
                        return Some(decl);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Generate a while statement.
    fn generate_while_stmt(&mut self, node: &ClangNode) {
        // Children: condition, body
        if node.children.len() >= 2 {
            let cond_node = &node.children[0];

            // Try to find a DeclStmt - it might be direct or wrapped in ImplicitCastExpr/ExprWithCleanups
            let decl_stmt_node = Self::find_decl_stmt_in_condition(cond_node);

            // Check if the condition is a VarDecl directly (no DeclStmt wrapper)
            // This happens with: while (int x = expr) where the VarDecl is a direct child of WhileStmt
            if let ClangNodeKind::VarDecl { name, ty, .. } = &cond_node.kind {
                let var_name = sanitize_identifier(name);
                let rust_type = ty.to_rust_type_str();
                let init = if !cond_node.children.is_empty() {
                    self.expr_to_string(&cond_node.children[0])
                } else {
                    "Default::default()".to_string()
                };

                // Generate loop with declaration and break check
                self.writeln("loop {");
                self.indent += 1;

                // Declare the variable
                self.writeln(&format!("let {}: {} = {};", var_name, rust_type, init));

                // Generate break condition based on type
                let break_cond = match ty {
                    CppType::Pointer { .. } => format!("if {}.is_null() {{ break; }}", var_name),
                    CppType::Bool => format!("if !{} {{ break; }}", var_name),
                    _ => format!("if {} == 0 {{ break; }}", var_name),
                };
                self.writeln(&break_cond);

                // Generate body
                self.generate_stmt(&node.children[1], false);

                self.indent -= 1;
                self.writeln("}");
                return;
            }

            // Check if the condition is a DeclStmt (variable declaration in while condition)
            // Example: while (unsigned char __c = *__ptr++) { ... }
            // This needs special handling: loop { let __c = *__ptr++; if __c == 0 { break; } ... }
            if let Some(decl_node) = decl_stmt_node {
                if let Some(var_child) = decl_node.children.first() {
                    if let ClangNodeKind::VarDecl { name, ty, .. } = &var_child.kind {
                        let var_name = sanitize_identifier(name);
                        let rust_type = ty.to_rust_type_str();
                        let init = if !var_child.children.is_empty() {
                            self.expr_to_string(&var_child.children[0])
                        } else {
                            "Default::default()".to_string()
                        };

                        // Generate loop with declaration and break check
                        self.writeln("loop {");
                        self.indent += 1;

                        // Declare the variable
                        self.writeln(&format!("let {}: {} = {};", var_name, rust_type, init));

                        // Generate break condition based on type
                        // For integer types: check if zero
                        // For pointers: check if null
                        // For bool: check if false
                        let break_cond = match ty {
                            CppType::Pointer { .. } => {
                                format!("if {}.is_null() {{ break; }}", var_name)
                            }
                            CppType::Bool => format!("if !{} {{ break; }}", var_name),
                            _ => format!("if {} == 0 {{ break; }}", var_name),
                        };
                        self.writeln(&break_cond);

                        // Generate body
                        self.generate_stmt(&node.children[1], false);

                        self.indent -= 1;
                        self.writeln("}");
                        return;
                    }
                }
            }

            // Standard while loop without declaration in condition
            let cond = self.expr_to_string(cond_node);
            // In C++, pointers can be used in boolean context (non-null = true)
            let cond_type = Self::get_expr_type(cond_node);
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
                            if i == 0
                                && matches!(&case_child.kind, ClangNodeKind::IntegerLiteral { .. })
                            {
                                continue; // Skip the case value literal
                            }
                            // Check for nested CaseStmt (fallthrough)
                            if let ClangNodeKind::CaseStmt { value: nested_val } = &case_child.kind
                            {
                                current_values.push(*nested_val);
                                // Process nested case's children
                                for (j, nested_child) in case_child.children.iter().enumerate() {
                                    if j == 0
                                        && matches!(
                                            &nested_child.kind,
                                            ClangNodeKind::IntegerLiteral { .. }
                                        )
                                    {
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
        let has_default = node.children.get(1).is_some_and(|c| {
            if let ClangNodeKind::CompoundStmt = &c.kind {
                c.children
                    .iter()
                    .any(|ch| matches!(&ch.kind, ClangNodeKind::DefaultStmt))
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
        let pattern = values
            .iter()
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
            self.writeln(&format!(
                "for {} in {}{} {{",
                sanitize_identifier(var_name),
                sanitize_identifier(&range_name),
                iter_suffix
            ));
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
                        UnaryOp::Deref => {
                            // Check if operand is a reference variable (tracked in ref_vars)
                            // In Rust, dereferencing a reference for method calls is automatic
                            // So *ref_var.method() should just be ref_var.method()
                            if let ClangNodeKind::DeclRefExpr { name, .. } =
                                &node.children[0].kind
                            {
                                if self.ref_vars.contains(name) {
                                    // Skip the dereference - Rust auto-derefs for method calls
                                    return operand;
                                }
                            }
                            format!("*{}", operand)
                        }
                        UnaryOp::Minus => {
                            // C++ allows -bool which converts bool to int then negates
                            // In Rust, we convert to logical NOT for boolean types
                            let operand_ty = Self::get_expr_type(&node.children[0]);
                            if matches!(operand_ty, Some(CppType::Bool)) {
                                format!("!{}", operand)
                            } else {
                                format!("-{}", operand)
                            }
                        }
                        UnaryOp::Plus => operand,
                        UnaryOp::LNot => {
                            // C++ logical NOT (!x) converts to bool first
                            // For non-bool types, `!x` means `x == 0` in C++
                            let operand_ty = Self::get_expr_type(&node.children[0]);
                            if matches!(operand_ty, Some(CppType::Bool)) {
                                format!("!{}", operand)
                            } else if matches!(operand_ty, Some(CppType::Pointer { .. })) {
                                // For pointer types, use is_null()
                                format!("{}.is_null()", operand)
                            } else {
                                // For non-bool non-pointer types, use == 0 comparison
                                format!("(({}) == 0)", operand)
                            }
                        }
                        UnaryOp::Not => format!("!{}", operand),
                        UnaryOp::AddrOf => {
                            // Check if this is a pointer to a polymorphic class
                            if let CppType::Pointer { pointee, is_const } = ty {
                                if let CppType::Named(class_name) = pointee.as_ref() {
                                    if self.polymorphic_classes.contains(class_name) {
                                        // For polymorphic types, use raw pointer for vtable dispatch
                                        let sanitized = sanitize_identifier(class_name);
                                        return if *is_const {
                                            format!("&{} as *const {}", operand, sanitized)
                                        } else {
                                            format!("&mut {} as *mut {}", operand, sanitized)
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
                        UnaryOp::PreInc => {
                            // For pointer types, use .add(1)
                            if matches!(ty, CppType::Pointer { .. }) {
                                format!(
                                    "{{ {} = unsafe {{ {}.add(1) }}; {} }}",
                                    operand, operand, operand
                                )
                            } else {
                                format!("{{ {} += 1; {} }}", operand, operand)
                            }
                        }
                        UnaryOp::PreDec => {
                            // For pointer types, use .sub(1)
                            if matches!(ty, CppType::Pointer { .. }) {
                                format!(
                                    "{{ {} = unsafe {{ {}.sub(1) }}; {} }}",
                                    operand, operand, operand
                                )
                            } else {
                                format!("{{ {} -= 1; {} }}", operand, operand)
                            }
                        }
                        UnaryOp::PostInc => {
                            // For pointer types, use .add(1)
                            if matches!(ty, CppType::Pointer { .. }) {
                                format!(
                                    "{{ let __v = {}; {} = unsafe {{ {}.add(1) }}; __v }}",
                                    operand, operand, operand
                                )
                            } else {
                                format!("{{ let __v = {}; {} += 1; __v }}", operand, operand)
                            }
                        }
                        UnaryOp::PostDec => {
                            // For pointer types, use .sub(1)
                            if matches!(ty, CppType::Pointer { .. }) {
                                format!(
                                    "{{ let __v = {}; {} = unsafe {{ {}.sub(1) }}; __v }}",
                                    operand, operand, operand
                                )
                            } else {
                                format!("{{ let __v = {}; {} -= 1; __v }}", operand, operand)
                            }
                        }
                    }
                } else {
                    "/* unary op error */".to_string()
                }
            }
            ClangNodeKind::ImplicitCastExpr { cast_kind, ty } => {
                // Handle implicit casts - some need explicit conversion in Rust
                if !node.children.is_empty() {
                    let child = &node.children[0];
                    let inner = self.expr_to_string_raw(child);
                    // Check if inner is a binary expression - needs parens for cast to apply to whole expr
                    let needs_parens = matches!(child.kind, ClangNodeKind::BinaryOperator { .. });
                    match cast_kind {
                        CastKind::IntegralCast => {
                            // Need explicit cast for integral conversions
                            let rust_type = ty.to_rust_type_str();
                            // Check if this is a cast to a non-primitive type (struct)
                            // Non-primitive types can't use `as` for conversion
                            let is_primitive = matches!(
                                ty,
                                CppType::Int { .. }
                                    | CppType::Short { .. }
                                    | CppType::Long { .. }
                                    | CppType::LongLong { .. }
                                    | CppType::Char { .. }
                                    | CppType::Float
                                    | CppType::Double
                                    | CppType::Bool
                                    | CppType::Pointer { .. }
                            ) || rust_type.starts_with("i")
                                || rust_type.starts_with("u")
                                || rust_type.starts_with("f")
                                || rust_type == "bool"
                                || rust_type.starts_with("*");
                            // Check if inner is a zero literal (possibly with type suffix)
                            let is_zero_literal =
                                inner == "0" || inner.starts_with("0i") || inner.starts_with("0u");
                            if !is_primitive && is_zero_literal {
                                // Casting 0 to a struct type - use zeroed() instead
                                format!("unsafe {{ std::mem::zeroed::<{}>() }}", rust_type)
                            } else if is_primitive {
                                if needs_parens {
                                    format!("({}) as {}", inner, rust_type)
                                } else {
                                    format!("{} as {}", inner, rust_type)
                                }
                            } else {
                                // Non-zero to non-primitive - can't do proper cast, use zeroed
                                format!("unsafe {{ std::mem::zeroed::<{}>() }}", rust_type)
                            }
                        }
                        CastKind::FloatingCast
                        | CastKind::IntegralToFloating
                        | CastKind::FloatingToIntegral => {
                            // Need explicit cast for floating conversions
                            let rust_type = ty.to_rust_type_str();
                            if needs_parens {
                                format!("({}) as {}", inner, rust_type)
                            } else {
                                format!("{} as {}", inner, rust_type)
                            }
                        }
                        CastKind::FunctionToPointerDecay => {
                            // Function to pointer decay - wrap in Some() for Option<fn(...)> type
                            format!("Some({})", inner)
                        }
                        _ => {
                            // Check for derived-to-base pointer cast for polymorphic types
                            // This requires explicit cast in Rust since we use raw pointers
                            if let CppType::Pointer { pointee, is_const } = ty {
                                if let CppType::Named(target_class) = pointee.as_ref() {
                                    if self.polymorphic_classes.contains(target_class) {
                                        // Check if inner expression has a different pointer type
                                        // Look for patterns like "... as *mut SomeClass" or "... as *const SomeClass"
                                        let sanitized_target = sanitize_identifier(target_class);
                                        let ptr_type = if *is_const {
                                            format!("*const {}", sanitized_target)
                                        } else {
                                            format!("*mut {}", sanitized_target)
                                        };
                                        // If inner already ends with the target pointer type, no need to cast
                                        if !inner.ends_with(&ptr_type) {
                                            // Need to add the cast
                                            return format!("{} as {}", inner, ptr_type);
                                        }
                                    }
                                }
                            }
                            // Most casts pass through (LValueToRValue, ArrayToPointerDecay, etc.)
                            inner
                        }
                    }
                } else {
                    "/* cast error */".to_string()
                }
            }
            ClangNodeKind::DeclRefExpr {
                name,
                namespace_path,
                ty,
                ..
            } => {
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
                        if let Some(global_name) =
                            self.static_members.get(&(class_name.clone(), name.clone()))
                        {
                            return global_name.clone();
                        }
                        // Fallback: generate from convention
                        // Use sanitize_static_member_name to avoid r# prefix issues with uppercase names
                        let global_name = format!(
                            "{}_{}",
                            class_name.to_uppercase(),
                            sanitize_static_member_name(name).to_uppercase()
                        );
                        let is_static_member =
                            self.static_members.values().any(|g| g == &global_name);
                        if is_static_member {
                            return global_name;
                        }
                    }
                    // Check if this is a static member of the current class (accessed without Class:: prefix)
                    if namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                        if let Some(ref current_class) = self.current_class {
                            if let Some(global_name) = self
                                .static_members
                                .get(&(current_class.clone(), name.clone()))
                            {
                                return global_name.clone();
                            }
                        }
                    }

                    // Check if this is a global variable (already in unsafe context, no wrapper needed)
                    // Global variables are prefixed with __gv_ to avoid parameter shadowing
                    if let Some(prefixed_name) = self.global_var_mapping.get(&ident) {
                        return prefixed_name.clone();
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
            ClangNodeKind::EvaluatedExpr {
                int_value,
                float_value,
                ty,
            } => {
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
                        // Parenthesize idx to handle operator precedence (e.g., size_ - 1 as usize)
                        format!("*{}.add(({}) as usize)", arr, idx)
                    } else {
                        // Array indexing
                        // Parenthesize idx to handle operator precedence (e.g., size_ - 1 as usize)
                        format!("{}[({}) as usize]", arr, idx)
                    }
                } else {
                    "/* array subscript error */".to_string()
                }
            }
            ClangNodeKind::MemberExpr {
                member_name,
                is_static,
                is_arrow,
                declaring_class,
                ..
            } => {
                // For static member access, return the global name without unsafe wrapper
                if *is_static {
                    if let Some(class_name) = declaring_class {
                        if let Some(global_name) = self
                            .static_members
                            .get(&(class_name.clone(), member_name.clone()))
                        {
                            return global_name.clone();
                        }
                        // Fallback: generate from convention
                        return format!(
                            "{}_{}",
                            class_name.to_uppercase(),
                            sanitize_static_member_name(member_name).to_uppercase()
                        );
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
                        // For dot access, if base starts with '*' (dereference) or contains 'as' (cast),
                        // we need to parenthesize it to get correct precedence.
                        // In Rust, `.` has higher precedence than `*` and `as`, so:
                        // - `*x.y` means `*(x.y)` - we want `(*x).y`
                        // - `x as T.y` means `x as (T.y)` - we want `(x as T).y`
                        // E.g., `*ptr.add(i).field` should be `(*ptr.add(i)).field`
                        // E.g., `ptr as *const T.field` should be `(ptr as *const T).field`
                        if base.starts_with('*') || base.contains(" as ") {
                            format!("({}).{}", base, member)
                        } else {
                            format!("{}.{}", base, member)
                        }
                    }
                } else {
                    // Implicit this - no children means this->member
                    format!("self.{}", sanitize_identifier(member_name))
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
            ClangNodeKind::EvaluatedExpr {
                int_value,
                float_value,
                ty,
            } => {
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
            ClangNodeKind::CXXNewExpr {
                ty,
                is_array,
                is_placement,
            } => {
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
                        (
                            "/* missing placement ptr */".to_string(),
                            default_value_for_type(ty),
                        )
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
                        format!(
                            "unsafe {{ fragile_delete_array::<{}>({}) }}",
                            elem_type_str, ptr
                        )
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
            ClangNodeKind::DeclRefExpr {
                name,
                namespace_path,
                ty,
                ..
            } => {
                if name == "this" {
                    if self.use_ctor_self {
                        "__self".to_string()
                    } else {
                        "self".to_string()
                    }
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
                        if let Some(global_name) =
                            self.static_members.get(&(class_name.clone(), name.clone()))
                        {
                            return format!("unsafe {{ {} }}", global_name);
                        }
                        // Try fallback: generate from convention if it looks like a static member
                        // (class name followed by member name, no function type)
                        // Use sanitize_static_member_name to avoid r# prefix issues with uppercase names
                        let global_name = format!(
                            "{}_{}",
                            class_name.to_uppercase(),
                            sanitize_static_member_name(name).to_uppercase()
                        );
                        // Check if this global exists in our static_members for any class
                        let is_static_member =
                            self.static_members.values().any(|g| g == &global_name);
                        if is_static_member {
                            return format!("unsafe {{ {} }}", global_name);
                        }
                    }

                    // Check if this is a static member of the current class (accessed without Class:: prefix)
                    if namespace_path.is_empty() && !matches!(ty, CppType::Function { .. }) {
                        if let Some(ref current_class) = self.current_class {
                            if let Some(global_name) = self
                                .static_members
                                .get(&(current_class.clone(), name.clone()))
                            {
                                return format!("unsafe {{ {} }}", global_name);
                            }
                        }
                    }

                    // Check if this is a global variable (needs unsafe access)
                    // Global variables are prefixed with __gv_ to avoid parameter shadowing
                    if let Some(prefixed_name) = self.global_var_mapping.get(&ident) {
                        return format!("unsafe {{ {} }}", prefixed_name);
                    }

                    // Check if this is a function template instantiation call
                    // If so, we need to use the mangled instantiation name
                    // (the instantiation was already collected during collect_template_info)
                    if let CppType::Function {
                        params,
                        return_type,
                        ..
                    } = ty
                    {
                        if let Some(template_info) = self.fn_template_definitions.get(name) {
                            // Build the mangled name using template param extraction
                            let type_args: Vec<String> = template_info
                                .template_params
                                .iter()
                                .enumerate()
                                .map(|(i, param_name)| {
                                    let (template_param_ty, instantiated_ty) =
                                        if i < template_info.params.len() && i < params.len() {
                                            (&template_info.params[i].1, &params[i])
                                        } else if matches!(
                                            &template_info.return_type,
                                            CppType::TemplateParam { .. }
                                        ) {
                                            (&template_info.return_type, return_type.as_ref())
                                        } else if i < params.len() {
                                            return params[i].to_rust_type_str();
                                        } else {
                                            return return_type.to_rust_type_str();
                                        };
                                    extract_template_arg(
                                        template_param_ty,
                                        instantiated_ty,
                                        param_name,
                                    )
                                })
                                .collect();
                            let sanitized_args: Vec<String> = type_args
                                .iter()
                                .map(|a| sanitize_type_for_fn_name(a))
                                .collect();
                            let mangled_name = format!("{}_{}", name, sanitized_args.join("_"));
                            return self.compute_relative_path(namespace_path, &mangled_name);
                        }
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
                if self.use_ctor_self {
                    "__self".to_string()
                } else {
                    "self".to_string()
                }
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
                    let left_is_global_subscript =
                        self.is_global_array_subscript(&node.children[0]);
                    let left_is_global_var = self.is_global_var_expr(&node.children[0]);
                    let left_is_arrow = Self::is_arrow_member_access(&node.children[0]);
                    let needs_unsafe = left_is_deref
                        || left_is_ptr_subscript
                        || left_is_static_member
                        || left_is_global_subscript
                        || left_is_global_var
                        || left_is_arrow;

                    // Check if left side is a pointer type for += / -= (need .add() / .sub())
                    let left_type = Self::get_expr_type(&node.children[0]);
                    let left_is_pointer = matches!(left_type, Some(CppType::Pointer { .. }));

                    // Handle function pointer comparison with nullptr: use .is_none() / .is_some()
                    let left_is_fn_ptr = left_type
                        .as_ref()
                        .is_some_and(Self::is_function_pointer_type);
                    if left_is_fn_ptr
                        && matches!(op, BinaryOp::Eq | BinaryOp::Ne)
                        && Self::is_nullptr_literal(&node.children[1])
                    {
                        let left = self.expr_to_string(&node.children[0]);
                        return if matches!(op, BinaryOp::Eq) {
                            format!("{}.is_none()", left)
                        } else {
                            format!("{}.is_some()", left)
                        };
                    }

                    // Handle pointer subtraction: ptr1 - ptr2 -> unsafe { ptr1.offset_from(ptr2) }
                    // Returns isize (number of elements between pointers)
                    let right_type = Self::get_expr_type(&node.children[1]);
                    let right_is_pointer = matches!(right_type, Some(CppType::Pointer { .. }));
                    if left_is_pointer && right_is_pointer && matches!(op, BinaryOp::Sub) {
                        let left = self.expr_to_string(&node.children[0]);
                        let right = self.expr_to_string(&node.children[1]);
                        return format!("unsafe {{ {}.offset_from({}) }}", left, right);
                    }

                    // Handle pointer arithmetic specially
                    if left_is_pointer && matches!(op, BinaryOp::AddAssign | BinaryOp::SubAssign) {
                        let left = self.expr_to_string(&node.children[0]);
                        let right = self.expr_to_string(&node.children[1]);
                        let method = if matches!(op, BinaryOp::AddAssign) {
                            "add"
                        } else {
                            "sub"
                        };
                        // Wrap complex expressions in parens before casting to usize
                        // ptr.add() is unsafe, so wrap in unsafe block
                        let right_needs_parens = right.contains(' ') || right.contains("as ");
                        if right_needs_parens {
                            format!(
                                "unsafe {{ {} = {}.{}(({}) as usize) }}",
                                left, left, method, right
                            )
                        } else {
                            format!(
                                "unsafe {{ {} = {}.{}({} as usize) }}",
                                left, left, method, right
                            )
                        }
                    } else if matches!(
                        op,
                        BinaryOp::Assign
                            | BinaryOp::AddAssign
                            | BinaryOp::SubAssign
                            | BinaryOp::MulAssign
                            | BinaryOp::DivAssign
                            | BinaryOp::RemAssign
                            | BinaryOp::AndAssign
                            | BinaryOp::OrAssign
                            | BinaryOp::XorAssign
                            | BinaryOp::ShlAssign
                            | BinaryOp::ShrAssign
                    ) && needs_unsafe
                    {
                        // For pointer dereference, subscript, or static member on left side, wrap entire assignment in unsafe
                        // Strip literal suffix on RHS - Rust infers type from LHS
                        let left_raw = self.expr_to_string_raw(&node.children[0]);
                        let right_str =
                            strip_literal_suffix(&self.expr_to_string_raw(&node.children[1]));

                        // Check if left side is float type and right side is integer literal
                        let left_type = Self::get_expr_type(&node.children[0]);
                        let left_is_float =
                            matches!(left_type, Some(CppType::Float | CppType::Double));
                        let right_raw = if left_is_float && is_integer_literal_str(&right_str) {
                            int_literal_to_float(&right_str)
                        } else {
                            right_str
                        };

                        format!("unsafe {{ {} {} {} }}", left_raw, op_str, right_raw)
                    } else if matches!(
                        op,
                        BinaryOp::Assign
                            | BinaryOp::AddAssign
                            | BinaryOp::SubAssign
                            | BinaryOp::MulAssign
                            | BinaryOp::DivAssign
                            | BinaryOp::RemAssign
                            | BinaryOp::AndAssign
                            | BinaryOp::OrAssign
                            | BinaryOp::XorAssign
                            | BinaryOp::ShlAssign
                            | BinaryOp::ShrAssign
                    ) {
                        // For assignment operators, strip literal suffix on RHS - Rust infers from LHS
                        let left = self.expr_to_string(&node.children[0]);
                        let right_str =
                            strip_literal_suffix(&self.expr_to_string(&node.children[1]));

                        // Check if left side is float type and right side is integer literal
                        // Rust requires float literals (e.g., 1.0) when assigning to float
                        let left_type = Self::get_expr_type(&node.children[0]);
                        let left_is_float =
                            matches!(left_type, Some(CppType::Float | CppType::Double));
                        let right = if left_is_float && is_integer_literal_str(&right_str) {
                            int_literal_to_float(&right_str)
                        } else {
                            right_str
                        };

                        format!("{} {} {}", left, op_str, right)
                    } else if matches!(
                        op,
                        BinaryOp::Eq
                            | BinaryOp::Ne
                            | BinaryOp::Lt
                            | BinaryOp::Le
                            | BinaryOp::Gt
                            | BinaryOp::Ge
                    ) {
                        // For comparison operators, strip literal suffixes - Rust infers compatible types
                        let left_str =
                            strip_literal_suffix(&self.expr_to_string(&node.children[0]));
                        let right_str =
                            strip_literal_suffix(&self.expr_to_string(&node.children[1]));

                        // Check if one side is float and the other is an integer literal
                        // Rust requires float literals (e.g., 0.0) when comparing with floats
                        let left_type = Self::get_expr_type(&node.children[0]);
                        let right_type = Self::get_expr_type(&node.children[1]);
                        let left_is_float =
                            matches!(left_type, Some(CppType::Float | CppType::Double));
                        let right_is_float =
                            matches!(right_type, Some(CppType::Float | CppType::Double));

                        let left = if right_is_float && is_integer_literal_str(&left_str) {
                            int_literal_to_float(&left_str)
                        } else {
                            left_str
                        };
                        let right = if left_is_float && is_integer_literal_str(&right_str) {
                            int_literal_to_float(&right_str)
                        } else {
                            right_str
                        };

                        // Wrap left operand in parens if it ends with "as TYPE" to prevent
                        // < being interpreted as generic arguments (e.g., `x as i32 < y`)
                        let left = if left.contains(" as ") && !left.starts_with('(') {
                            format!("({})", left)
                        } else {
                            left
                        };
                        format!("{} {} {}", left, op_str, right)
                    } else if matches!(op, BinaryOp::Add | BinaryOp::Sub) && left_is_pointer {
                        // Pointer + integer or pointer - integer -> ptr.add(n) or ptr.sub(n)
                        // Note: pointer - pointer is handled earlier with offset_from
                        let left_str = self.expr_to_string(&node.children[0]);
                        let right_str =
                            strip_literal_suffix(&self.expr_to_string(&node.children[1]));
                        let method = if matches!(op, BinaryOp::Add) {
                            "add"
                        } else {
                            "sub"
                        };
                        // Wrap complex expressions in parens before casting to usize
                        let right_needs_parens = right_str.contains(' ') || right_str.contains("as ");
                        if right_needs_parens {
                            format!("unsafe {{ {}.{}(({}) as usize) }}", left_str, method, right_str)
                        } else {
                            format!("unsafe {{ {}.{}({} as usize) }}", left_str, method, right_str)
                        }
                    } else if matches!(
                        op,
                        BinaryOp::Add
                            | BinaryOp::Sub
                            | BinaryOp::Mul
                            | BinaryOp::Div
                            | BinaryOp::Rem
                    ) {
                        // For arithmetic operators, strip literal suffixes and handle float/int mixing
                        let left_str =
                            strip_literal_suffix(&self.expr_to_string(&node.children[0]));
                        let right_str =
                            strip_literal_suffix(&self.expr_to_string(&node.children[1]));

                        // Check if one side is float and the other is an integer literal
                        let left_type = Self::get_expr_type(&node.children[0]);
                        let right_type = Self::get_expr_type(&node.children[1]);
                        let left_is_float =
                            matches!(left_type, Some(CppType::Float | CppType::Double));
                        let right_is_float =
                            matches!(right_type, Some(CppType::Float | CppType::Double));

                        let left = if right_is_float && is_integer_literal_str(&left_str) {
                            int_literal_to_float(&left_str)
                        } else {
                            left_str
                        };
                        let right = if left_is_float && is_integer_literal_str(&right_str) {
                            int_literal_to_float(&right_str)
                        } else {
                            right_str
                        };
                        format!("{} {} {}", left, op_str, right)
                    } else if matches!(
                        op,
                        BinaryOp::And
                            | BinaryOp::Or
                            | BinaryOp::Xor
                            | BinaryOp::Shl
                            | BinaryOp::Shr
                    ) {
                        // For bitwise operators, strip literal suffixes to let Rust infer types
                        // This handles cases like `isize / 64i32` -> `isize / 64`
                        let left = strip_literal_suffix(&self.expr_to_string(&node.children[0]));
                        let right = strip_literal_suffix(&self.expr_to_string(&node.children[1]));
                        // For shift operators, if left side contains `as` (a cast), we need to
                        // parenthesize it. Otherwise Rust parses `1 as u64 << X` as `1 as (u64<<X>)`.
                        let left = if matches!(op, BinaryOp::Shl | BinaryOp::Shr)
                            && left.contains(" as ")
                        {
                            format!("({})", left)
                        } else {
                            left
                        };
                        format!("{} {} {}", left, op_str, right)
                    } else {
                        let left = self.expr_to_string(&node.children[0]);
                        let right = self.expr_to_string(&node.children[1]);
                        // For comparison/relational operators, if left side is an unsafe block,
                        // we need to parenthesize it. Rust requires `(unsafe { X }) > Y`,
                        // not `unsafe { X } > Y`.
                        let left = if matches!(
                            op,
                            BinaryOp::Lt
                                | BinaryOp::Le
                                | BinaryOp::Gt
                                | BinaryOp::Ge
                                | BinaryOp::Eq
                                | BinaryOp::Ne
                        ) && left.contains("unsafe {")
                        {
                            format!("({})", left)
                        } else {
                            left
                        };
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
                        UnaryOp::Minus => {
                            // C++ allows -bool which converts bool to int then negates
                            // In Rust, we convert to logical NOT for boolean types
                            let operand_ty = Self::get_expr_type(&node.children[0]);
                            if matches!(operand_ty, Some(CppType::Bool)) {
                                format!("!{}", operand)
                            } else {
                                format!("-{}", operand)
                            }
                        }
                        UnaryOp::Plus => operand,
                        UnaryOp::LNot => {
                            // C++ logical NOT (!x) converts to bool first
                            // For non-bool types, `!x` means `x == 0` in C++
                            let operand_ty = Self::get_expr_type(&node.children[0]);
                            if matches!(operand_ty, Some(CppType::Bool)) {
                                format!("!{}", operand)
                            } else if matches!(operand_ty, Some(CppType::Pointer { .. })) {
                                // For pointer types, use is_null()
                                format!("{}.is_null()", operand)
                            } else {
                                // For non-bool non-pointer types, use == 0 comparison
                                format!("(({}) == 0)", operand)
                            }
                        }
                        UnaryOp::Not => format!("!{}", operand), // bitwise not ~ in C++
                        UnaryOp::AddrOf => {
                            // Check if child is an ArraySubscriptExpr with a pointer base
                            // In C++, &arr[i] where arr is a pointer is equivalent to arr + i
                            // We can generate arr.add(i as usize) directly instead of
                            // &mut unsafe { *arr.add(i as usize) } as *mut T
                            let child = &node.children[0];
                            if let ClangNodeKind::ArraySubscriptExpr { .. } = &child.kind {
                                if child.children.len() >= 2 {
                                    let arr_type = Self::get_expr_type(&child.children[0]);
                                    let is_pointer =
                                        matches!(arr_type, Some(CppType::Pointer { .. }))
                                            || matches!(
                                                arr_type,
                                                Some(CppType::Array { size: None, .. })
                                            )
                                            || self.is_ptr_var_expr(&child.children[0]);

                                    if is_pointer {
                                        let arr = self.expr_to_string(&child.children[0]);
                                        let idx = self.expr_to_string(&child.children[1]);
                                        // Pointer arithmetic requires unsafe block
                                        return format!(
                                            "unsafe {{ {}.add(({}) as usize) }}",
                                            arr, idx
                                        );
                                    }
                                }
                            }

                            // Check if this is a pointer to a polymorphic class
                            if let CppType::Pointer { pointee, is_const } = ty {
                                if let CppType::Named(class_name) = pointee.as_ref() {
                                    if self.polymorphic_classes.contains(class_name) {
                                        // For polymorphic types, use raw pointer for vtable dispatch
                                        let sanitized = sanitize_identifier(class_name);
                                        return if *is_const {
                                            format!("&{} as *const {}", operand, sanitized)
                                        } else {
                                            format!("&mut {} as *mut {}", operand, sanitized)
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
                            } else if let ClangNodeKind::DeclRefExpr { name, .. } =
                                &node.children[0].kind
                            {
                                // Check if operand is a reference variable (tracked in ref_vars)
                                // In Rust, dereferencing a reference for method calls is automatic
                                if self.ref_vars.contains(name) {
                                    // Skip the dereference - Rust auto-derefs
                                    operand
                                } else {
                                    // Raw pointer dereference needs unsafe
                                    format!("unsafe {{ *{} }}", operand)
                                }
                            } else {
                                // Raw pointer dereference needs unsafe
                                format!("unsafe {{ *{} }}", operand)
                            }
                        }
                        UnaryOp::PreInc | UnaryOp::PreDec => {
                            let is_pointer = matches!(ty, CppType::Pointer { .. });
                            // For global variables, wrap entire operation in unsafe
                            if is_global {
                                let raw_name = self
                                    .get_raw_var_name(&node.children[0])
                                    .unwrap_or(operand.clone());
                                if is_pointer {
                                    let method = if matches!(op, UnaryOp::PreInc) {
                                        "add"
                                    } else {
                                        "sub"
                                    };
                                    format!(
                                        "unsafe {{ {} = {}.{}(1); {} }}",
                                        raw_name, raw_name, method, raw_name
                                    )
                                } else {
                                    let op_str = if matches!(op, UnaryOp::PreInc) {
                                        "+="
                                    } else {
                                        "-="
                                    };
                                    format!("unsafe {{ {} {} 1; {} }}", raw_name, op_str, raw_name)
                                }
                            } else if is_pointer {
                                // Pointer arithmetic with .add/.sub is unsafe
                                let method = if matches!(op, UnaryOp::PreInc) {
                                    "add"
                                } else {
                                    "sub"
                                };
                                format!(
                                    "unsafe {{ {} = {}.{}(1); {} }}",
                                    operand, operand, method, operand
                                )
                            } else {
                                let op_str = if matches!(op, UnaryOp::PreInc) {
                                    "+="
                                } else {
                                    "-="
                                };
                                format!("{{ {} {} 1; {} }}", operand, op_str, operand)
                            }
                        }
                        UnaryOp::PostInc | UnaryOp::PostDec => {
                            let is_pointer = matches!(ty, CppType::Pointer { .. });
                            // For global variables, wrap entire operation in unsafe
                            if is_global {
                                let raw_name = self
                                    .get_raw_var_name(&node.children[0])
                                    .unwrap_or(operand.clone());
                                if is_pointer {
                                    let method = if matches!(op, UnaryOp::PostInc) {
                                        "add"
                                    } else {
                                        "sub"
                                    };
                                    format!(
                                        "unsafe {{ let __v = {}; {} = {}.{}(1); __v }}",
                                        raw_name, raw_name, raw_name, method
                                    )
                                } else {
                                    let op_str = if matches!(op, UnaryOp::PostInc) {
                                        "+="
                                    } else {
                                        "-="
                                    };
                                    format!(
                                        "unsafe {{ let __v = {}; {} {} 1; __v }}",
                                        raw_name, raw_name, op_str
                                    )
                                }
                            } else if is_pointer {
                                // Pointer arithmetic with .add/.sub is unsafe
                                let method = if matches!(op, UnaryOp::PostInc) {
                                    "add"
                                } else {
                                    "sub"
                                };
                                format!(
                                    "unsafe {{ let __v = {}; {} = {}.{}(1); __v }}",
                                    operand, operand, operand, method
                                )
                            } else {
                                let op_str = if matches!(op, UnaryOp::PostInc) {
                                    "+="
                                } else {
                                    "-="
                                };
                                format!(
                                    "{{ let __v = {}; {} {} 1; __v }}",
                                    operand, operand, op_str
                                )
                            }
                        }
                    }
                } else {
                    "/* unary op error */".to_string()
                }
            }
            ClangNodeKind::CallExpr { ty } => {
                // Check if this is a virtual method call through a pointer to polymorphic class
                // If so, generate vtable dispatch instead of trait-based dispatch
                if let Some(vtable_call) = self.try_generate_vtable_dispatch(node) {
                    return vtable_call;
                }

                // Check if this is a std::get call on a variant
                if let Some((variant_arg, variant_type, return_type)) = Self::is_std_get_call(node)
                {
                    if let Some(idx) =
                        Self::get_variant_index_from_return_type(&variant_type, return_type)
                    {
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
                if let Some((adaptor, range_node, arg_node)) = Self::is_std_views_adaptor_call(node)
                {
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
                                return format!(
                                    "{}.iter().{}({})",
                                    range_expr, adaptor, count_expr
                                );
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
                if let Some((algo, range_node, arg_node)) = Self::is_std_ranges_algorithm_call(node)
                {
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
                                return format!(
                                    "{}.iter().filter({}).count()",
                                    range_expr, pred_expr
                                );
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
                                let args: Vec<String> = node
                                    .children
                                    .iter()
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
                if let Some((op_name, left_idx, right_idx_opt)) = Self::get_operator_call_info(node)
                {
                    // Special handling for global operator new/delete
                    // These are not method calls but global allocation functions
                    // For operator new/delete, find the actual argument (not the operator reference)
                    if op_name == "operator new" || op_name == "operator new[]" {
                        // ::operator new(size) -> fragile_runtime::fragile_malloc(size)
                        // Find the size argument - it's the child that's not the function reference
                        let size_arg = node
                            .children
                            .iter()
                            .find(|c| !Self::is_function_reference(c))
                            .map(|c| self.expr_to_string(c))
                            .unwrap_or_else(|| "0".to_string());
                        return format!(
                            "unsafe {{ crate::fragile_runtime::fragile_malloc({}) }}",
                            size_arg
                        );
                    }
                    if op_name == "operator delete" || op_name == "operator delete[]" {
                        // ::operator delete(ptr) -> fragile_runtime::fragile_free(ptr)
                        // Find the pointer argument - it's the child that's not the function reference
                        let ptr_arg = node
                            .children
                            .iter()
                            .find(|c| !Self::is_function_reference(c))
                            .map(|c| self.expr_to_string(c))
                            .unwrap_or_else(|| "std::ptr::null_mut()".to_string());
                        return format!("unsafe {{ crate::fragile_runtime::fragile_free({} as *mut std::ffi::c_void) }}", ptr_arg);
                    }

                    // Convert operator name to method name (operator+ -> op_add)
                    let method_name = sanitize_identifier(&op_name);
                    let left_operand = self.expr_to_string(&node.children[left_idx]);

                    if op_name == "operator()" {
                        // Function call operator: callee.op_call(args...)
                        // Collect all children except the callee and the operator() reference
                        let args: Vec<String> = node
                            .children
                            .iter()
                            .enumerate()
                            .filter(|(i, c)| *i != left_idx && !Self::is_function_reference(c))
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
                        let left_is_typeid =
                            matches!(
                                &node.children[left_idx].kind,
                                ClangNodeKind::TypeidExpr { .. }
                            ) || Self::contains_typeid_expr(&node.children[left_idx]);
                        let right_is_typeid =
                            matches!(
                                &node.children[right_idx].kind,
                                ClangNodeKind::TypeidExpr { .. }
                            ) || Self::contains_typeid_expr(&node.children[right_idx]);

                        if left_is_typeid
                            && right_is_typeid
                            && (op_name == "operator==" || op_name == "operator!=")
                        {
                            let rust_op = if op_name == "operator==" { "==" } else { "!=" };
                            return format!("{} {} {}", left_operand, rust_op, right_operand);
                        }

                        let right_type = Self::get_expr_type(&node.children[right_idx]);

                        // Special case: operator= (copy assignment vs converting assignment)
                        // For simple structs without explicit operator=, Clang generates implicit
                        // operator= calls. We should use Rust assignment instead of calling op_assign,
                        // since simple structs derive Clone and don't need op_assign method.
                        // This covers POD types like struct Token { int type; int value; }
                        //
                        // However, if the RHS type differs from LHS type, it's a converting assignment
                        // (e.g., Counter::operator=(int)) and we must call op_assign to perform conversion.
                        if op_name == "operator=" {
                            let left_type = Self::get_expr_type(&node.children[left_idx]);
                            let is_same_type = match (&left_type, &right_type) {
                                (Some(left_ty), Some(right_ty)) => left_ty == right_ty,
                                _ => false,
                            };

                            if is_same_type {
                                // Copy assignment - use Rust assignment with clone() for struct types
                                // For primitives, clone() is optimized away
                                return format!("{} = {}.clone()", left_operand, right_operand);
                            }
                            // Otherwise, fall through to generate op_assign call for converting assignment
                        }
                        // Pass class/struct types by reference, primitives by value
                        // Named types that are typedefs to primitives should be passed by value
                        let needs_ref = match &right_type {
                            Some(CppType::Named(name)) => {
                                // These are typedefs to primitive types - pass by value
                                !matches!(
                                    name.as_str(),
                                    "ptrdiff_t"
                                        | "std::ptrdiff_t"
                                        | "ssize_t"
                                        | "size_t"
                                        | "std::size_t"
                                        | "intptr_t"
                                        | "std::intptr_t"
                                        | "uintptr_t"
                                        | "std::uintptr_t"
                                        | "difference_type"
                                        | "size_type"
                                        | "int8_t"
                                        | "int16_t"
                                        | "int32_t"
                                        | "int64_t"
                                        | "uint8_t"
                                        | "uint16_t"
                                        | "uint32_t"
                                        | "uint64_t"
                                )
                            }
                            _ => false,
                        };
                        // Parenthesize left operand if it contains a cast (to avoid Rust precedence issues)
                        // e.g., `x as T.method()` is parsed as `x as (T.method())` in Rust
                        let left_paren = if left_operand.contains(" as ") {
                            format!("({})", left_operand)
                        } else {
                            left_operand.clone()
                        };
                        if needs_ref {
                            format!("{}.{}(&{})", left_paren, method_name, right_operand)
                        } else {
                            format!("{}.{}({})", left_paren, method_name, right_operand)
                        }
                    } else {
                        // Other unary operators: operand.op_X()
                        // Parenthesize if it contains a cast
                        let left_paren = if left_operand.contains(" as ") {
                            format!("({})", left_operand)
                        } else {
                            left_operand.clone()
                        };
                        format!("{}.{}()", left_paren, method_name)
                    }
                } else if let CppType::Named(cpp_struct_name) = ty {
                    // Convert C++ type name to valid Rust identifier
                    let struct_name = CppType::Named(cpp_struct_name.clone()).to_rust_type_str();

                    // Check if this is a function call (not a constructor)
                    // A function call has a DeclRefExpr child with Function type
                    let is_function_call = node.children.iter().any(Self::is_function_reference);

                    if is_function_call && !node.children.is_empty() {
                        // Regular function call that returns a struct
                        let func = self.expr_to_string(&node.children[0]);
                        // Strip Some() wrapper if present - callee shouldn't be wrapped
                        // (FunctionToPointerDecay on callee is just a C++ technicality)
                        let func = Self::strip_some_wrapper(&func);
                        let args: Vec<String> = node.children[1..]
                            .iter()
                            .map(|c| self.expr_to_string(c))
                            .collect();
                        format!("{}({})", func, args.join(", "))
                    } else {
                        // Constructor call: all children are arguments (but skip TypeRef nodes)
                        // First, filter to get only argument nodes
                        let arg_nodes: Vec<&ClangNode> = node
                            .children
                            .iter()
                            .filter(|c| {
                                // Skip TypeRef nodes (they're type references, not arguments)
                                if let ClangNodeKind::Unknown(s) = &c.kind {
                                    if s.starts_with("TypeRef:") || s == "TypeRef" {
                                        return false;
                                    }
                                }
                                true
                            })
                            .collect();

                        // Check if this is a copy constructor call (single arg of same type)
                        let is_copy_ctor = arg_nodes.len() == 1 && {
                            let arg_type = Self::get_expr_type(arg_nodes[0]);
                            let arg_class = Self::extract_class_name(&arg_type);
                            arg_class
                                .map(|name| name == *cpp_struct_name)
                                .unwrap_or(false)
                        };

                        if is_copy_ctor {
                            // For copy constructor (T(x) where x:T), use .clone() since
                            // all generated structs derive Clone (either implicitly via derive
                            // or explicitly via Clone impl that calls new_1)
                            let arg_str = self.expr_to_string(arg_nodes[0]);
                            format!("{}.clone()", arg_str)
                        } else {
                            // Regular constructor - convert args and call new_N
                            let args: Vec<String> =
                                arg_nodes.iter().map(|c| self.expr_to_string(c)).collect();
                            let num_args = args.len();

                            // Check if the type maps to a pointer, primitive, or non-struct type
                            // that can't have a constructor (e.g., `*mut std::ffi::c_void`)
                            let is_non_struct = struct_name.starts_with('*')
                                || struct_name.starts_with('&')
                                || struct_name == "std::ffi::c_void"
                                || struct_name == "()"
                                || struct_name == "bool"
                                || struct_name == "i8"
                                || struct_name == "i16"
                                || struct_name == "i32"
                                || struct_name == "i64"
                                || struct_name == "i128"
                                || struct_name == "u8"
                                || struct_name == "u16"
                                || struct_name == "u32"
                                || struct_name == "u64"
                                || struct_name == "u128"
                                || struct_name == "f32"
                                || struct_name == "f64"
                                || struct_name == "isize"
                                || struct_name == "usize"
                                || struct_name == "char";

                            if is_non_struct {
                                // For non-struct types, just use the first argument as-is
                                // (copy "constructor" becomes identity, default "constructor" becomes Default)
                                if num_args == 0 {
                                    "Default::default()".to_string()
                                } else if num_args == 1 {
                                    args[0].clone()
                                } else {
                                    // Multiple args for non-struct type - shouldn't happen but handle gracefully
                                    args[0].clone()
                                }
                            } else {
                                // Always use StructName::new_N(args) to ensure custom constructor bodies run
                                format!("{}::new_{}({})", struct_name, num_args, args.join(", "))
                            }
                        }
                    }
                } else if !node.children.is_empty() {
                    // Check if this is a virtual base method call
                    if let Some((base, vbase_field, method)) =
                        self.get_virtual_base_method_call_info(&node.children[0])
                    {
                        let args: Vec<String> = node.children[1..]
                            .iter()
                            .map(|c| self.expr_to_string(c))
                            .collect();
                        return format!(
                            "unsafe {{ (*{}.{}).{}({}) }}",
                            base,
                            vbase_field,
                            method,
                            args.join(", ")
                        );
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

                    let args: Vec<String> = node.children[1..]
                        .iter()
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
                                        || matches!(&types[i], CppType::Array { size: None, .. })
                                    {
                                        let arg_type = Self::get_expr_type(c);
                                        let is_array =
                                            matches!(arg_type, Some(CppType::Array { .. }));
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

                            // Fallback: For method calls (MemberExpr as callee), if the argument is
                            // a class/struct type, pass by reference. This handles cases where param_types
                            // couldn't be extracted (e.g., "<bound member function type>").
                            let is_method_call = matches!(
                                &node.children[0].kind,
                                ClangNodeKind::MemberExpr { .. }
                            ) || matches!(
                                &node.children[0].kind,
                                ClangNodeKind::ImplicitCastExpr { .. }
                                    if node.children[0].children.iter().any(|child| {
                                        matches!(&child.kind, ClangNodeKind::MemberExpr { .. })
                                    })
                            );

                            if is_method_call && param_types.is_none() {
                                let arg_type = Self::get_expr_type(c);
                                // Check if the argument is a class/struct type that should be passed by reference
                                let needs_ref = match &arg_type {
                                    Some(CppType::Named(name)) => {
                                        // These are typedefs to primitive types - pass by value
                                        !matches!(
                                            name.as_str(),
                                            "ptrdiff_t"
                                                | "std::ptrdiff_t"
                                                | "ssize_t"
                                                | "size_t"
                                                | "std::size_t"
                                                | "intptr_t"
                                                | "std::intptr_t"
                                                | "uintptr_t"
                                                | "std::uintptr_t"
                                                | "difference_type"
                                                | "size_type"
                                                | "int8_t"
                                                | "int16_t"
                                                | "int32_t"
                                                | "int64_t"
                                                | "uint8_t"
                                                | "uint16_t"
                                                | "uint32_t"
                                                | "uint64_t"
                                        )
                                    }
                                    _ => false,
                                };
                                if needs_ref {
                                    let arg_str = self.expr_to_string(c);
                                    return format!("&{}", arg_str);
                                }
                            }

                            self.expr_to_string(c)
                        })
                        .collect();

                    // Check if this is a compiler builtin function call
                    if let Some((rust_code, needs_unsafe)) =
                        Self::map_builtin_function(&func, &args)
                    {
                        return if needs_unsafe {
                            format!("unsafe {{ {} }}", rust_code)
                        } else {
                            rust_code
                        };
                    }

                    // Check if this is a C library function that should be mapped to fragile-runtime
                    let func = if let Some(runtime_func) = Self::map_runtime_function_name(&func) {
                        runtime_func.to_string()
                    } else {
                        func
                    };

                    // Check if the function expression is wrapped in unsafe (from arrow member access)
                    // If so, put the function call inside the unsafe block
                    if func.starts_with("unsafe { ") && func.ends_with(" }") {
                        let inner = &func[9..func.len() - 2]; // Extract "(*...).method" from "unsafe { (*...).method }"
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
            ClangNodeKind::MemberExpr {
                member_name,
                is_arrow,
                declaring_class,
                is_static,
                ..
            } => {
                // Check for static member access first
                if *is_static {
                    // Look up the global variable name for this static member
                    if let Some(class_name) = declaring_class {
                        if let Some(global_name) = self
                            .static_members
                            .get(&(class_name.clone(), member_name.clone()))
                        {
                            return format!("unsafe {{ {} }}", global_name);
                        }
                    }
                    // Fallback: generate global name from convention
                    if let Some(class_name) = declaring_class {
                        let global_name = format!(
                            "{}_{}",
                            class_name.to_uppercase(),
                            sanitize_static_member_name(member_name).to_uppercase()
                        );
                        return format!("unsafe {{ {} }}", global_name);
                    }
                }

                if !node.children.is_empty() {
                    // Check if the child is a TypeRef (qualified call like Base::foo())
                    // In this case, use implicit "self" and access through base class
                    let is_type_ref = matches!(
                        &node.children[0].kind,
                        ClangNodeKind::Unknown(s) if s.starts_with("TypeRef:")
                    );
                    // For qualified calls like Base::foo(), we need to access the base class member
                    // Extract the base class name from TypeRef if present
                    let qualified_base_class = if is_type_ref {
                        if let ClangNodeKind::Unknown(s) = &node.children[0].kind {
                            // Extract class name from "TypeRef:ClassName"
                            s.strip_prefix("TypeRef:").map(|s| s.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let base = if is_type_ref {
                        // Qualified call: Base::foo() means call base class method on self
                        // We need to access through __base field for inherited methods
                        let self_name = if self.use_ctor_self {
                            "__self".to_string()
                        } else {
                            "self".to_string()
                        };
                        // Get the base access path for the qualified class
                        if let Some(ref qual_class) = qualified_base_class {
                            // Look up the base class in current class's hierarchy
                            if let Some(ref current_class) = self.current_class {
                                let base_access =
                                    self.get_base_access_for_class(current_class, qual_class);
                                match base_access {
                                    BaseAccess::DirectField(field) if !field.is_empty() => {
                                        format!("{}.{}", self_name, field)
                                    }
                                    BaseAccess::FieldChain(chain) if !chain.is_empty() => {
                                        format!("{}.{}", self_name, chain)
                                    }
                                    BaseAccess::VirtualPtr(field) => {
                                        format!("unsafe {{ (*{}.{}) }}", self_name, field)
                                    }
                                    _ => self_name,
                                }
                            } else {
                                self_name
                            }
                        } else {
                            self_name
                        }
                    } else {
                        // For member access, check if base is a reference variable
                        // Rust auto-derefs for `.` access, so we don't need explicit `*`
                        // This prevents generating `*__str.method()` which parses as `*(__str.method())`
                        if let Some(ref_ident) = self.get_ref_var_ident(&node.children[0]) {
                            ref_ident
                        } else {
                            self.expr_to_string(&node.children[0])
                        }
                    };
                    // Check if this is accessing an inherited member
                    // Use get_original_expr_type to look through implicit casts (like UncheckedDerivedToBase)
                    // This ensures we get the actual object type, not the casted base class type
                    let base_type = Self::get_original_expr_type(&node.children[0]);

                    // Determine if we need base access and get the correct base field name
                    // Skip base access for anonymous struct members (they are flattened into parent)
                    let (needs_base_access, base_access) = if let Some(decl_class) = declaring_class
                    {
                        // Anonymous struct members are flattened - access directly
                        if decl_class.starts_with("(anonymous") || decl_class.starts_with("__anon_")
                        {
                            (false, BaseAccess::DirectField(String::new()))
                        } else {
                            let base_class_name = Self::extract_class_name(&base_type);
                            if let Some(name) = base_class_name {
                                // Strip namespace prefix and template arguments from BOTH sides for comparison
                                // (e.g., std::ctype<char> -> ctype, std::_Bit_reference -> _Bit_reference)
                                let name_base = Self::strip_namespace_and_template(&name);
                                let decl_class_base =
                                    Self::strip_namespace_and_template(decl_class);
                                // Compare base names (without namespaces or template args)
                                if name_base != decl_class_base {
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
                            // For polymorphic class pointers, use direct method call
                            // The trait implementation will dispatch correctly
                            format!("{}.{}", base, member)
                        } else if needs_base_access {
                            match base_access {
                                BaseAccess::VirtualPtr(field) => {
                                    format!("unsafe {{ (*(*{}).{}).{} }}", base, field, member)
                                }
                                BaseAccess::DirectField(field) | BaseAccess::FieldChain(field) => {
                                    // If field is empty, this is a template/stub type without base class info
                                    if field.is_empty() {
                                        format!("unsafe {{ (*{}).{} }}", base, member)
                                    } else {
                                        // Dereferencing raw pointers requires unsafe
                                        format!("unsafe {{ (*{}).{}.{} }}", base, field, member)
                                    }
                                }
                            }
                        } else {
                            // Dereferencing raw pointers requires unsafe
                            format!("unsafe {{ (*{}).{} }}", base, member)
                        }
                    } else if needs_base_access {
                        match base_access {
                            BaseAccess::VirtualPtr(field) => {
                                format!("unsafe {{ (*{}.{}).{} }}", base, field, member)
                            }
                            BaseAccess::DirectField(field) | BaseAccess::FieldChain(field) => {
                                // If field is empty, this is a template/stub type without base class info
                                // Just access the member directly
                                if field.is_empty() {
                                    format!("{}.{}", base, member)
                                } else {
                                    format!("{}.{}.{}", base, field, member)
                                }
                            }
                        }
                    } else {
                        // Check if base involves pointer subscript - if so, we need to use
                        // raw access and wrap in unsafe to avoid nested unsafe blocks and
                        // move-out-of-raw-pointer issues.
                        // E.g., `cache->entries[i].valid` should become:
                        // `unsafe { (*(*cache).entries.add(i as usize)).valid }`
                        // NOT: `unsafe { *unsafe { (*cache).entries }.add(i) }.valid`
                        let base_has_ptr_subscript = self.is_pointer_subscript(&node.children[0]);
                        if base_has_ptr_subscript && !is_type_ref {
                            let base_raw = self.expr_to_string_raw(&node.children[0]);
                            // If base_raw starts with * or contains 'as', parenthesize for correct precedence
                            if base_raw.starts_with('*') || base_raw.contains(" as ") {
                                format!("unsafe {{ ({}).{} }}", base_raw, member)
                            } else {
                                format!("unsafe {{ {}.{} }}", base_raw, member)
                            }
                        } else {
                            // Parenthesize if base starts with '*' (deref) or contains 'as' (cast)
                            // since Rust's '*' and 'as' have lower precedence than '.'
                            // - `*x.y` means `*(x.y)` in Rust, we want `(*x).y`
                            // - `x as T.y` is invalid, we want `(x as T).y`
                            if base.starts_with('*') || base.contains(" as ") {
                                format!("({}).{}", base, member)
                            } else {
                                format!("{}.{}", base, member)
                            }
                        }
                    }
                } else {
                    // Implicit this - check if member is inherited
                    let member = sanitize_identifier(member_name);
                    let self_name = if self.use_ctor_self { "__self" } else { "self" };
                    let (needs_base_access, base_access) =
                        if let (Some(current), Some(decl_class)) =
                            (&self.current_class, declaring_class)
                        {
                            // Anonymous struct members are flattened - access directly
                            if decl_class.starts_with("(anonymous")
                                || decl_class.starts_with("__anon_")
                            {
                                (false, BaseAccess::DirectField(String::new()))
                            } else {
                                // Strip namespace prefix and template arguments from BOTH sides for comparison
                                // (e.g., std::ctype<char> -> ctype, std::_Bit_reference -> _Bit_reference)
                                let current_base = Self::strip_namespace_and_template(current);
                                let decl_class_base =
                                    Self::strip_namespace_and_template(decl_class);
                                // Compare base names (without namespaces or template args)
                                if current_base != decl_class_base {
                                    let access =
                                        self.get_base_access_for_class(current, decl_class);
                                    (true, access)
                                } else {
                                    (false, BaseAccess::DirectField(String::new()))
                                }
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
                                // If field is empty, this is a template/stub type without base class info
                                if field.is_empty() {
                                    format!("{}.{}", self_name, member)
                                } else {
                                    format!("{}.{}.{}", self_name, field, member)
                                }
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
                        let raw_name = self
                            .get_raw_var_name(&node.children[0])
                            .unwrap_or_else(|| self.expr_to_string(&node.children[0]));
                        // Parenthesize idx to handle operator precedence (e.g., size_ - 1 as usize)
                        format!("unsafe {{ {}[({}) as usize] }}", raw_name, idx)
                    } else if is_pointer {
                        let arr = self.expr_to_string(&node.children[0]);
                        // Parenthesize if arr contains a cast (`as`) since Rust's `as` has lower
                        // precedence than method calls, and `ptr as T.add()` is invalid
                        let arr = if arr.contains(" as ") {
                            format!("({})", arr)
                        } else {
                            arr
                        };
                        // Pointer indexing requires unsafe pointer arithmetic
                        // Parenthesize idx to handle operator precedence (e.g., size_ - 1 as usize)
                        format!("unsafe {{ *{}.add(({}) as usize) }}", arr, idx)
                    } else {
                        let arr = self.expr_to_string(&node.children[0]);
                        // Parenthesize if arr contains a cast (`as`) since Rust's `as` has lower
                        // precedence than indexing, and `ptr as T[idx]` is invalid
                        let arr = if arr.contains(" as ") {
                            format!("({})", arr)
                        } else {
                            arr
                        };
                        // Array indexing - cast index to usize
                        // Parenthesize idx to handle operator precedence (e.g., size_ - 1 as usize)
                        format!("{}[({}) as usize]", arr, idx)
                    }
                } else {
                    "/* array subscript error */".to_string()
                }
            }
            ClangNodeKind::ConditionalOperator { .. } => {
                if node.children.len() >= 3 {
                    let cond_child = &node.children[0];
                    let cond = self.expr_to_string(cond_child);
                    let then_expr = self.expr_to_string(&node.children[1]);
                    let else_expr = self.expr_to_string(&node.children[2]);

                    // Check if condition is a pointer type - needs null check in Rust
                    let cond_type = Self::get_expr_type(cond_child);
                    let cond_str = if matches!(cond_type, Some(CppType::Pointer { .. })) {
                        // Pointer used as boolean: convert to !ptr.is_null()
                        format!("!{}.is_null()", cond)
                    } else {
                        cond
                    };

                    format!(
                        "if {} {{ {} }} else {{ {} }}",
                        cond_str, then_expr, else_expr
                    )
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
                    let child = &node.children[0];
                    let inner = self.expr_to_string(child);
                    // Check if inner is a binary expression - needs parens for cast to apply to whole expr
                    let needs_parens = matches!(child.kind, ClangNodeKind::BinaryOperator { .. });
                    match cast_kind {
                        CastKind::IntegralCast => {
                            // Need explicit cast for integral conversions
                            let rust_type = ty.to_rust_type_str();
                            // Check if this is a cast to a non-primitive type (struct)
                            // Non-primitive types can't use `as` for conversion
                            let is_primitive = matches!(
                                ty,
                                CppType::Int { .. }
                                    | CppType::Short { .. }
                                    | CppType::Long { .. }
                                    | CppType::LongLong { .. }
                                    | CppType::Char { .. }
                                    | CppType::Float
                                    | CppType::Double
                                    | CppType::Bool
                                    | CppType::Pointer { .. }
                            ) || rust_type.starts_with("i")
                                || rust_type.starts_with("u")
                                || rust_type.starts_with("f")
                                || rust_type == "bool"
                                || rust_type.starts_with("*");
                            // Check if inner is a zero literal (possibly with type suffix)
                            let is_zero_literal =
                                inner == "0" || inner.starts_with("0i") || inner.starts_with("0u");
                            if !is_primitive && is_zero_literal {
                                // Casting 0 to a struct type - use zeroed() instead
                                format!("unsafe {{ std::mem::zeroed::<{}>() }}", rust_type)
                            } else if is_primitive {
                                if needs_parens {
                                    format!("({}) as {}", inner, rust_type)
                                } else {
                                    format!("{} as {}", inner, rust_type)
                                }
                            } else {
                                // Non-zero to non-primitive - can't do proper cast, use zeroed
                                format!("unsafe {{ std::mem::zeroed::<{}>() }}", rust_type)
                            }
                        }
                        CastKind::FloatingCast
                        | CastKind::IntegralToFloating
                        | CastKind::FloatingToIntegral => {
                            // Need explicit cast for floating conversions
                            let rust_type = ty.to_rust_type_str();
                            if needs_parens {
                                format!("({}) as {}", inner, rust_type)
                            } else {
                                format!("{} as {}", inner, rust_type)
                            }
                        }
                        CastKind::FunctionToPointerDecay => {
                            // Function to pointer decay - wrap in Some() for Option<fn(...)> type
                            format!("Some({})", inner)
                        }
                        _ => {
                            // Check for derived-to-base pointer cast for polymorphic types
                            // This requires explicit cast in Rust since we use raw pointers
                            if let CppType::Pointer { pointee, is_const } = ty {
                                if let CppType::Named(target_class) = pointee.as_ref() {
                                    if self.polymorphic_classes.contains(target_class) {
                                        // Check if inner expression has a different pointer type
                                        // Look for patterns like "... as *mut SomeClass" or "... as *const SomeClass"
                                        let sanitized_target = sanitize_identifier(target_class);
                                        let ptr_type = if *is_const {
                                            format!("*const {}", sanitized_target)
                                        } else {
                                            format!("*mut {}", sanitized_target)
                                        };
                                        // If inner already ends with the target pointer type, no need to cast
                                        if !inner.ends_with(&ptr_type) {
                                            // Need to add the cast
                                            return format!("{} as {}", inner, ptr_type);
                                        }
                                    }
                                }
                            }
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

                    // Find the actual expression child, skipping TypeRef nodes
                    // CStyleCastExpr typically has [TypeRef, expression] or just [expression]
                    let inner_node = node.children.iter().find(|c| {
                        !matches!(&c.kind, ClangNodeKind::Unknown(s) if s.starts_with("TypeRef"))
                    });
                    let inner = if let Some(inner_child) = inner_node {
                        self.expr_to_string(inner_child)
                    } else {
                        // Fallback to first child
                        self.expr_to_string(&node.children[0])
                    };
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
                    // Strip const/volatile qualifiers from the type name
                    // C++ allows "const Struct { ... }" for constexpr, but Rust doesn't
                    let struct_name = name
                        .trim_start_matches("const ")
                        .trim_start_matches("volatile ")
                        .trim();

                    // Check if this is designated initialization (children have MemberRef)
                    // Designated: { .x = 10, .y = 20 } produces UnexposedExpr(MemberRef, value)
                    // Non-designated: { 10, 20 } produces IntegerLiteral directly
                    let mut field_values: Vec<(String, String)> = Vec::new();
                    let mut has_designators = false;

                    for child in &node.children {
                        // Check if child is UnexposedExpr wrapper with MemberRef designator
                        if matches!(&child.kind, ClangNodeKind::Unknown(s) if s == "UnexposedExpr")
                            && child.children.len() >= 2
                        {
                            if let ClangNodeKind::MemberRef { name: field_name } =
                                &child.children[0].kind
                            {
                                // This is a designated initializer
                                has_designators = true;
                                // The value is the second child (or beyond)
                                let value = self.expr_to_string(&child.children[1]);
                                field_values.push((field_name.clone(), value));
                                continue;
                            }
                        }
                        // Non-designated: just get the value
                        let value = self.expr_to_string(child);
                        field_values.push((String::new(), value));
                    }

                    if has_designators {
                        // All values have field names from designators
                        let inits: Vec<String> = field_values
                            .iter()
                            .map(|(f, v)| format!("{}: {}", f, v))
                            .collect();
                        format!("{} {{ {} }}", struct_name, inits.join(", "))
                    } else {
                        // Try to get field names for this struct (positional)
                        // Try both original name and stripped name for lookup
                        let struct_fields_opt = self
                            .class_fields
                            .get(name)
                            .or_else(|| self.class_fields.get(struct_name));
                        if let Some(struct_fields) = struct_fields_opt {
                            let inits: Vec<String> = field_values
                                .iter()
                                .enumerate()
                                .map(|(i, (_, v))| {
                                    if i < struct_fields.len() {
                                        format!("{}: {}", struct_fields[i].0, v)
                                    } else {
                                        v.clone()
                                    }
                                })
                                .collect();
                            format!("{} {{ {} }}", struct_name, inits.join(", "))
                        } else {
                            // Fallback: can't determine field names
                            let values: Vec<String> =
                                field_values.into_iter().map(|(_, v)| v).collect();
                            format!("{} {{ {} }}", struct_name, values.join(", "))
                        }
                    }
                } else if matches!(ty, CppType::Array { .. }) {
                    // Array type - use array literal syntax
                    let elems: Vec<String> = node
                        .children
                        .iter()
                        .map(|c| self.expr_to_string(c))
                        .collect();
                    format!("[{}]", elems.join(", "))
                } else if node.children.len() == 1 {
                    // Single-element init list for scalar type - just use the element
                    self.expr_to_string(&node.children[0])
                } else {
                    // Multiple elements for non-array type - shouldn't happen but use tuple
                    let elems: Vec<String> = node
                        .children
                        .iter()
                        .map(|c| self.expr_to_string(c))
                        .collect();
                    format!("({})", elems.join(", "))
                }
            }
            ClangNodeKind::LambdaExpr {
                params,
                return_type,
                capture_default,
                captures,
            } => {
                // Generate Rust closure
                // C++: [captures](params) -> ret { body }
                // Rust: |params| -> ret { body } or move |params| { body }
                use crate::ast::CaptureDefault;

                // Determine if we need 'move' keyword
                let needs_move = *capture_default == CaptureDefault::ByCopy
                    || captures.iter().any(|(_, by_ref)| !*by_ref);

                // Generate parameter list with deduplication
                let mut param_name_counts: HashMap<String, usize> = HashMap::new();
                let params_str = params
                    .iter()
                    .map(|(name, ty)| {
                        let mut param_name = sanitize_identifier(name);
                        let count = param_name_counts.entry(param_name.clone()).or_insert(0);
                        if *count > 0 {
                            param_name = format!("{}_{}", param_name, *count);
                        }
                        *param_name_counts
                            .get_mut(&sanitize_identifier(name))
                            .unwrap() += 1;
                        format!("{}: {}", param_name, ty.to_rust_type_str())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                // Generate return type (omit if void)
                let ret_str = if *return_type == CppType::Void {
                    String::new()
                } else {
                    format!(
                        " -> {}",
                        Self::sanitize_return_type(&return_type.to_rust_type_str())
                    )
                };

                // Find the body (CompoundStmt child)
                let body = node
                    .children
                    .iter()
                    .find(|c| matches!(&c.kind, ClangNodeKind::CompoundStmt));

                let body_str = if let Some(body_node) = body {
                    // Check for simple single-return lambdas
                    if body_node.children.len() == 1 {
                        if let ClangNodeKind::ReturnStmt = &body_node.children[0].kind {
                            if !body_node.children[0].children.is_empty() {
                                // Single return with expression - Rust closure can omit return
                                return if needs_move {
                                    format!(
                                        "move |{}|{} {}",
                                        params_str,
                                        ret_str,
                                        self.expr_to_string(&body_node.children[0].children[0])
                                    )
                                } else {
                                    format!(
                                        "|{}|{} {}",
                                        params_str,
                                        ret_str,
                                        self.expr_to_string(&body_node.children[0].children[0])
                                    )
                                };
                            }
                        }
                    }
                    // Multi-statement body - generate block
                    let stmts: Vec<String> = body_node
                        .children
                        .iter()
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
                        // Use to_rust_type_str() instead of Debug formatting to avoid quote issues
                        format!("panic!(\"Threw {}\")", ty.to_rust_type_str())
                    } else {
                        "panic!(\"Exception thrown\")".to_string()
                    }
                } else {
                    // throw; (rethrow) - in Rust, just continue panicking
                    "panic!(\"Rethrow\")".to_string()
                }
            }
            // C++ RTTI expressions
            ClangNodeKind::TypeidExpr {
                is_type_operand,
                operand_ty,
                ..
            } => {
                // typeid(expr) or typeid(Type) → std::any::TypeId::of::<T>()
                if *is_type_operand {
                    // typeid(Type) → TypeId::of::<RustType>()
                    format!(
                        "std::any::TypeId::of::<{}>()",
                        operand_ty.to_rust_type_str()
                    )
                } else if !node.children.is_empty() {
                    // typeid(expr) → for polymorphic types, we'd need runtime RTTI
                    // For now, use the static type from the operand
                    let expr = self.expr_to_string(&node.children[0]);
                    format!(
                        "/* typeid({}) */ std::any::TypeId::of::<{}>()",
                        expr,
                        operand_ty.to_rust_type_str()
                    )
                } else {
                    format!(
                        "std::any::TypeId::of::<{}>()",
                        operand_ty.to_rust_type_str()
                    )
                }
            }
            ClangNodeKind::DynamicCastExpr { target_ty } => {
                // dynamic_cast has different behavior for pointers vs references:
                // - dynamic_cast<T*>(expr) returns nullptr on failure
                // - dynamic_cast<T&>(expr) throws std::bad_cast on failure
                if !node.children.is_empty() {
                    // Find the expression child (skip TypeRef nodes)
                    // DynamicCastExpr children: [TypeRef:TargetType, UnexposedExpr(actual expr)]
                    let expr_node = node.children.iter().find(|child| {
                        !matches!(&child.kind, ClangNodeKind::Unknown(s) if s.starts_with("TypeRef"))
                    });
                    let expr = expr_node
                        .map(|n| self.expr_to_string(n))
                        .unwrap_or_else(|| "()".to_string());
                    let target_str = target_ty.to_rust_type_str();

                    match target_ty {
                        CppType::Reference {
                            referent, is_const, ..
                        } => {
                            // Reference dynamic_cast - throws on failure (std::bad_cast)
                            let inner_type = referent.to_rust_type_str();
                            let sanitized_target = sanitize_identifier(&inner_type);

                            // Check if target is a polymorphic class
                            if self.polymorphic_classes.contains(&inner_type) {
                                // Use RTTI to check type at runtime, panic on failure
                                // Access vtable directly - for dynamic_cast, source is always a base
                                // class pointer with __vtable at the root
                                format!(
                                    "unsafe {{ \
                                        let __target_id = {}_TYPE_ID; \
                                        let __vtable = (*{}).__vtable; \
                                        let __found = (*__vtable).__base_type_ids.contains(&__target_id); \
                                        if !__found {{ panic!(\"std::bad_cast\"); }} \
                                        &*({} as *{} {}) \
                                    }}",
                                    sanitized_target.to_uppercase(),
                                    expr,
                                    expr,
                                    if *is_const { "const" } else { "mut" },
                                    inner_type
                                )
                            } else {
                                // Non-polymorphic, just do static cast
                                format!(
                                    "unsafe {{ *(({} as *const _ as *const {}) as *{} {}) }}",
                                    expr,
                                    inner_type,
                                    if *is_const { "const" } else { "mut" },
                                    inner_type
                                )
                            }
                        }
                        CppType::Pointer { pointee, is_const } => {
                            // Pointer dynamic_cast - returns null on failure
                            let inner_type = pointee.to_rust_type_str();
                            let ptr_prefix = if *is_const { "*const" } else { "*mut" };
                            let sanitized_target = sanitize_identifier(&inner_type);

                            // Check if target is a polymorphic class
                            if self.polymorphic_classes.contains(&inner_type) {
                                // Use RTTI to check type at runtime
                                // Access vtable directly - for dynamic_cast, source is always a base
                                // class pointer with __vtable at the root
                                format!(
                                    "unsafe {{ \
                                        let __ptr = {}; \
                                        if __ptr.is_null() {{ std::ptr::null_mut() }} else {{ \
                                            let __target_id = {}_TYPE_ID; \
                                            let __vtable = (*__ptr).__vtable; \
                                            let __found = (*__vtable).__base_type_ids.contains(&__target_id); \
                                            if __found {{ __ptr as {} {} }} else {{ std::ptr::null_mut() }} \
                                        }} \
                                    }}",
                                    expr,
                                    sanitized_target.to_uppercase(),
                                    ptr_prefix,
                                    inner_type
                                )
                            } else {
                                // Non-polymorphic, just do static cast
                                format!("{} as {} {}", expr, ptr_prefix, inner_type)
                            }
                        }
                        _ => {
                            // Fallback for unexpected types
                            format!("/* dynamic_cast */ {} as {}", expr, target_str)
                        }
                    }
                } else {
                    format!(
                        "/* dynamic_cast to {} without operand */",
                        target_ty.to_rust_type_str()
                    )
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
                // Log diagnostic for unknown node types
                if let ClangNodeKind::Unknown(kind_str) = &node.kind {
                    self.log_diagnostic(
                        "Unknown node",
                        &format!(
                            "kind='{}', has_children={}",
                            kind_str,
                            !node.children.is_empty()
                        ),
                    );
                }

                // Fallback: try children
                if !node.children.is_empty() {
                    self.expr_to_string(&node.children[0])
                } else {
                    // For unsupported expressions, return 0 as a safe fallback
                    // This handles cases like SubstNonTypeTemplateParmExpr that libclang doesn't expose
                    "0".to_string()
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
                        let rust_type = ty.to_rust_type_str();
                        let init = if !child.children.is_empty() {
                            let expr = self.expr_to_string(&child.children[0]);
                            // Check if this is a Named type with "0" initializer, which indicates
                            // a CXXConstructExpr that couldn't be parsed properly
                            // In that case, generate a constructor call instead
                            if let CppType::Named(_) = ty {
                                // Only generate constructor for actual struct types, not primitives
                                let is_primitive = matches!(
                                    rust_type.as_str(),
                                    "usize"
                                        | "isize"
                                        | "i8"
                                        | "i16"
                                        | "i32"
                                        | "i64"
                                        | "i128"
                                        | "u8"
                                        | "u16"
                                        | "u32"
                                        | "u64"
                                        | "u128"
                                        | "f32"
                                        | "f64"
                                        | "bool"
                                        | "()"
                                        | "char"
                                ) || rust_type.starts_with('*')
                                    || rust_type.starts_with('&');
                                if expr == "0" && !is_primitive {
                                    // Use unsafe zeroed for template types (contain __)
                                    if rust_type.contains("__") {
                                        " = unsafe { std::mem::zeroed() }".to_string()
                                    } else {
                                        format!(" = {}::new_0()", rust_type)
                                    }
                                } else {
                                    format!(" = {}", expr)
                                }
                            } else {
                                format!(" = {}", expr)
                            }
                        } else {
                            String::new()
                        };
                        return format!(
                            "let mut {}: {}{};",
                            sanitize_identifier(name),
                            rust_type,
                            init
                        );
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
            "operator bool" => "op_bool".to_string(),
            "operator int" => "op_int".to_string(),
            "operator long" => "op_long".to_string(),
            "operator double" => "op_double".to_string(),
            "operator float" => "op_float".to_string(),
            _ => {
                // Handle user-defined literal operators like operator""sv
                // These generate invalid Rust identifiers with quotes
                if name.contains("\"\"") {
                    // Extract suffix after quotes: operator""sv -> op_literal_sv
                    if let Some(suffix) = name.strip_prefix("operator\"\"") {
                        format!("op_literal_{}", sanitize_identifier(suffix.trim()))
                    } else {
                        "op_literal".to_string()
                    }
                } else if let Some(type_part) = name.strip_prefix("operator ") {
                    // Handle other conversion operators like "operator SomeType"
                    format!("op_{}", sanitize_identifier(type_part))
                } else {
                    name.replace("operator", "op_")
                }
            }
        }
    } else {
        name.to_string()
    };

    // Replace invalid characters
    result = result
        .replace("::", "_")
        .replace(['<', '>'], "_")
        .replace(' ', "")
        .replace(
            [
                '%', '=', '&', '|', '!', '*', '/', '+', '-', '[', ']', '(', ')', ',', ';', '.',
                ':', '^', '~', '"', '\'', '#', '@', '$', '?', '\\',
            ],
            "_",
        );

    // Handle keywords
    if RUST_KEYWORDS.contains(&result.as_str()) {
        // "Self" cannot be used with r# prefix - it's a special keyword
        // Also "self" is problematic in certain contexts
        if result == "Self" {
            result = "Self_".to_string();
        } else if result == "self" {
            result = "self_".to_string();
        } else {
            result = format!("r#{}", result);
        }
    }

    // Handle empty names
    if result.is_empty() {
        result = "_unnamed".to_string();
    }

    result
}

/// Sanitize identifier for use in static member names (CLASS_MEMBER format).
/// Unlike sanitize_identifier, this doesn't apply r# prefix since the result
/// will be uppercased and combined with a class name prefix.
fn sanitize_static_member_name(name: &str) -> String {
    let mut result = name.to_string();

    // Replace invalid characters
    result = result
        .replace("::", "_")
        .replace(['<', '>'], "_")
        .replace(' ', "")
        .replace(
            [
                '%', '=', '&', '|', '!', '*', '/', '+', '-', '[', ']', '(', ')', ',', ';', '.',
                ':', '^', '~', '"', '\'', '#', '@', '$', '?', '\\',
            ],
            "_",
        );

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
        BinaryOp::And => "&",   // Bitwise AND
        BinaryOp::Or => "|",    // Bitwise OR
        BinaryOp::Xor => "^",   // Bitwise XOR
        BinaryOp::LAnd => "&&", // Logical AND
        BinaryOp::LOr => "||",  // Logical OR
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
        BinaryOp::Spaceship => "cmp", // Handled specially - placeholder
    }
}

/// Extract the template argument by comparing the template pattern with the instantiated type.
/// For example, if pattern is `T*` and instantiated is `int*`, returns "i32".
/// If pattern is `T` and instantiated is `int`, returns "i32".
fn extract_template_arg(pattern: &CppType, instantiated: &CppType, _param_name: &str) -> String {
    match (pattern, instantiated) {
        // Direct template parameter: T → instantiated type
        (CppType::TemplateParam { .. }, ty) => ty.to_rust_type_str(),
        // Pointer to template param: T* → extract pointee from instantiated
        (
            CppType::Pointer {
                pointee: p_pattern, ..
            },
            CppType::Pointer {
                pointee: inst_pointee,
                ..
            },
        ) => extract_template_arg(p_pattern, inst_pointee, _param_name),
        // Reference to template param: T& → extract referent from instantiated
        (
            CppType::Reference {
                referent: r_pattern,
                ..
            },
            CppType::Reference {
                referent: inst_referent,
                ..
            },
        ) => extract_template_arg(r_pattern, inst_referent, _param_name),
        // Array of template param: T[N] → extract element from instantiated
        (
            CppType::Array {
                element: e_pattern, ..
            },
            CppType::Array {
                element: inst_element,
                ..
            },
        ) => extract_template_arg(e_pattern, inst_element, _param_name),
        // Pattern doesn't match structure - use instantiated type directly
        _ => instantiated.to_rust_type_str(),
    }
}

/// Sanitize a type name for use in function names (e.g., template instantiation mangling).
/// Converts "*mut i32" to "ptr_mut_i32", "i32" stays "i32", etc.
fn sanitize_type_for_fn_name(ty: &str) -> String {
    ty.replace("*mut ", "ptr_mut_")
        .replace("*const ", "ptr_const_")
        .replace('*', "ptr_")
        .replace("::", "_")
        .replace("->", "_ret_") // Handle function return type arrow before stripping '>'
        .replace([' ', '<'], "_")
        .replace('>', "")
        .replace(',', "_")
        .replace('&', "ref_")
        .replace(['[', ']', ';', '(', ')', '"'], "_") // Handle quotes in extern "C" linkage specifiers
}

/// Get default value for a type.
fn default_value_for_type(ty: &CppType) -> String {
    match ty {
        CppType::Void => "()".to_string(),
        CppType::Bool => "false".to_string(),
        CppType::Char { .. }
        | CppType::Short { .. }
        | CppType::Int { .. }
        | CppType::Long { .. }
        | CppType::LongLong { .. } => "0".to_string(),
        CppType::Float => "0.0f32".to_string(),
        CppType::Double => "0.0f64".to_string(),
        CppType::Pointer { .. } => "std::ptr::null_mut()".to_string(),
        CppType::Reference { .. } => "std::ptr::null_mut()".to_string(),
        CppType::Named(_) => "unsafe { std::mem::zeroed() }".to_string(),
        CppType::Array { element, size } => {
            // For arrays of non-primitive types, use zeroed() for the whole array
            // since [elem_default; N] requires Copy but zeroed() for [T; N] works directly
            if let Some(n) = size {
                match element.as_ref() {
                    CppType::Char { .. }
                    | CppType::Short { .. }
                    | CppType::Int { .. }
                    | CppType::Long { .. }
                    | CppType::LongLong { .. } => format!("[0; {}]", n),
                    CppType::Float => format!("[0.0f32; {}]", n),
                    CppType::Double => format!("[0.0f64; {}]", n),
                    CppType::Bool => format!("[false; {}]", n),
                    CppType::Pointer { .. } => format!("[std::ptr::null_mut(); {}]", n),
                    // For struct arrays and other non-Copy types, zero the entire array
                    _ => "unsafe { std::mem::zeroed() }".to_string(),
                }
            } else {
                "unsafe { std::mem::zeroed() }".to_string()
            }
        }
        _ => "unsafe { std::mem::zeroed() }".to_string(),
    }
}

/// Correct a field initializer value based on the field's type.
/// Converts literal `0` to `std::ptr::null_mut()` for pointer fields.
fn correct_initializer_for_type(value: &str, ty: &CppType) -> String {
    // If value is `0` and the type is a pointer, use null_mut()
    if matches!(ty, CppType::Pointer { .. }) && value == "0" {
        "std::ptr::null_mut()".to_string()
    } else {
        value.to_string()
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
                                make_node(
                                    ClangNodeKind::DeclRefExpr {
                                        name: "a".to_string(),
                                        ty: CppType::Int { signed: true },
                                        namespace_path: vec![],
                                    },
                                    vec![],
                                ),
                                make_node(
                                    ClangNodeKind::DeclRefExpr {
                                        name: "b".to_string(),
                                        ty: CppType::Int { signed: true },
                                        namespace_path: vec![],
                                    },
                                    vec![],
                                ),
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
                            make_node(
                                ClangNodeKind::BinaryOperator {
                                    op: BinaryOp::Gt,
                                    ty: CppType::Bool,
                                },
                                vec![
                                    make_node(
                                        ClangNodeKind::DeclRefExpr {
                                            name: "a".to_string(),
                                            ty: CppType::Int { signed: true },
                                            namespace_path: vec![],
                                        },
                                        vec![],
                                    ),
                                    make_node(
                                        ClangNodeKind::DeclRefExpr {
                                            name: "b".to_string(),
                                            ty: CppType::Int { signed: true },
                                            namespace_path: vec![],
                                        },
                                        vec![],
                                    ),
                                ],
                            ),
                            // Then: return a
                            make_node(
                                ClangNodeKind::ReturnStmt,
                                vec![make_node(
                                    ClangNodeKind::DeclRefExpr {
                                        name: "a".to_string(),
                                        ty: CppType::Int { signed: true },
                                        namespace_path: vec![],
                                    },
                                    vec![],
                                )],
                            ),
                            // Else: return b
                            make_node(
                                ClangNodeKind::ReturnStmt,
                                vec![make_node(
                                    ClangNodeKind::DeclRefExpr {
                                        name: "b".to_string(),
                                        ty: CppType::Int { signed: true },
                                        namespace_path: vec![],
                                    },
                                    vec![],
                                )],
                            ),
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
                        ClangNodeKind::CoreturnStmt {
                            value_ty: Some(CppType::Int { signed: true }),
                        },
                        vec![make_node(
                            ClangNodeKind::IntegerLiteral {
                                value: 42,
                                cpp_type: Some(CppType::Int { signed: true }),
                            },
                            vec![],
                        )],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Should generate async fn with i32 return type (not Task<int>)
        assert!(
            code.contains("pub async fn compute() -> i32"),
            "Expected 'pub async fn compute() -> i32', got:\n{}",
            code
        );
        // Should have coroutine comment
        assert!(
            code.contains("/// Coroutine: async (Task<int>)"),
            "Expected coroutine comment, got:\n{}",
            code
        );
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
                                ClangNodeKind::IntegerLiteral {
                                    value: 1,
                                    cpp_type: Some(CppType::Int { signed: true }),
                                },
                                vec![],
                            )],
                        ),
                        make_node(
                            ClangNodeKind::CoyieldExpr {
                                value_ty: CppType::Int { signed: true },
                                result_ty: CppType::Void,
                            },
                            vec![make_node(
                                ClangNodeKind::IntegerLiteral {
                                    value: 2,
                                    cpp_type: Some(CppType::Int { signed: true }),
                                },
                                vec![],
                            )],
                        ),
                    ],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Generators should NOT be async
        assert!(
            !code.contains("async fn range"),
            "Generator should not be async, got:\n{}",
            code
        );
        // Should return impl Iterator<Item=i32>
        assert!(
            code.contains("impl Iterator<Item=i32>"),
            "Expected 'impl Iterator<Item=i32>', got:\n{}",
            code
        );
        // Should have coroutine comment
        assert!(
            code.contains("/// Coroutine: generator (Generator<int>)"),
            "Expected coroutine comment, got:\n{}",
            code
        );
        // Should generate state machine struct
        assert!(
            code.contains("pub struct RangeGenerator"),
            "Expected 'pub struct RangeGenerator', got:\n{}",
            code
        );
        assert!(
            code.contains("__state: i32"),
            "Expected '__state: i32' field, got:\n{}",
            code
        );
        // Should implement Iterator
        assert!(
            code.contains("impl Iterator for RangeGenerator"),
            "Expected Iterator impl, got:\n{}",
            code
        );
        assert!(
            code.contains("type Item = i32"),
            "Expected 'type Item = i32', got:\n{}",
            code
        );
        assert!(
            code.contains("fn next(&mut self)"),
            "Expected 'fn next(&mut self)', got:\n{}",
            code
        );
        // Should have state machine match arms
        assert!(
            code.contains("match self.__state"),
            "Expected match on __state, got:\n{}",
            code
        );
        assert!(
            code.contains("Some(1i32)"),
            "Expected 'Some(1i32)' for first yield, got:\n{}",
            code
        );
        assert!(
            code.contains("Some(2i32)"),
            "Expected 'Some(2i32)' for second yield, got:\n{}",
            code
        );
        // Function should return generator instance
        assert!(
            code.contains("RangeGenerator { __state: 0 }"),
            "Expected generator instance creation, got:\n{}",
            code
        );
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
                vec![make_node(ClangNodeKind::CompoundStmt, vec![])],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Should fallback to using the original return type
        assert!(
            code.contains("CustomCoroutine"),
            "Expected 'CustomCoroutine' in return type, got:\n{}",
            code
        );
        // Should have coroutine comment
        assert!(
            code.contains("/// Coroutine: custom"),
            "Expected coroutine comment, got:\n{}",
            code
        );
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
                            ClangNodeKind::IntegerLiteral {
                                value: 0,
                                cpp_type: Some(CppType::Int { signed: true }),
                            },
                            vec![],
                        )],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Should NOT be async
        assert!(
            !code.contains("async fn regular"),
            "Regular function should not be async, got:\n{}",
            code
        );
        // Should be just a regular pub fn
        assert!(
            code.contains("pub fn regular() -> i32"),
            "Expected 'pub fn regular() -> i32', got:\n{}",
            code
        );
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
                    params: vec![(
                        "fmt".to_string(),
                        CppType::Pointer {
                            pointee: Box::new(CppType::Char { signed: true }),
                            is_const: true,
                        },
                    )],
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
                            ClangNodeKind::IntegerLiteral {
                                value: 0,
                                cpp_type: Some(CppType::Int { signed: true }),
                            },
                            vec![],
                        )],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        // Should have unsafe extern "C" and variadic signature
        // Rust requires unsafe for variadic extern "C" functions
        assert!(
            code.contains("unsafe extern \"C\""),
            "Variadic function should have unsafe extern \"C\", got:\n{}",
            code
        );
        assert!(
            code.contains("..."),
            "Variadic function should have ... in signature, got:\n{}",
            code
        );
        assert!(
            code.contains("pub unsafe extern \"C\" fn my_printf(fmt: *const i8, ...)"),
            "Expected 'pub unsafe extern \"C\" fn my_printf(fmt: *const i8, ...)', got:\n{}",
            code
        );
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
                    is_definition: true,
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
        assert!(
            code.contains("_bitfield_0: u16"),
            "Expected bit field storage '_bitfield_0: u16', got:\n{}",
            code
        );
        // Should NOT have individual fields a, b, c
        assert!(
            !code.contains("pub a:"),
            "Should not have individual 'a' field, got:\n{}",
            code
        );
        assert!(
            !code.contains("pub b:"),
            "Should not have individual 'b' field, got:\n{}",
            code
        );
        assert!(
            !code.contains("pub c:"),
            "Should not have individual 'c' field, got:\n{}",
            code
        );
        // Should have getter/setter for each bit field
        assert!(
            code.contains("pub fn a(&self)"),
            "Expected getter 'fn a(&self)', got:\n{}",
            code
        );
        assert!(
            code.contains("pub fn set_a(&mut self"),
            "Expected setter 'fn set_a(&mut self)', got:\n{}",
            code
        );
        assert!(
            code.contains("pub fn b(&self)"),
            "Expected getter 'fn b(&self)', got:\n{}",
            code
        );
        assert!(
            code.contains("pub fn set_b(&mut self"),
            "Expected setter 'fn set_b(&mut self)', got:\n{}",
            code
        );
        assert!(
            code.contains("pub fn c(&self)"),
            "Expected getter 'fn c(&self)', got:\n{}",
            code
        );
        assert!(
            code.contains("pub fn set_c(&mut self"),
            "Expected setter 'fn set_c(&mut self)', got:\n{}",
            code
        );
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
                    is_definition: true,
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
        assert!(
            code.contains("_bitfield_0: u8"),
            "Expected bit field storage '_bitfield_0: u8', got:\n{}",
            code
        );
        // Regular fields should still exist
        assert!(
            code.contains("pub x: i32"),
            "Expected regular field 'x: i32', got:\n{}",
            code
        );
        assert!(
            code.contains("pub y: i32"),
            "Expected regular field 'y: i32', got:\n{}",
            code
        );
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
                    is_definition: true,
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
        assert!(
            code.contains("_bitfield_0: u8"),
            "Expected first bit field storage '_bitfield_0: u8', got:\n{}",
            code
        );
        assert!(
            code.contains("_bitfield_1: u8"),
            "Expected second bit field storage '_bitfield_1: u8', got:\n{}",
            code
        );
        assert!(
            code.contains("pub x: i32"),
            "Expected regular field 'x: i32', got:\n{}",
            code
        );
    }
}
