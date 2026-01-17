//! Clang AST parsing and MIR conversion for the Fragile polyglot compiler.
//!
//! This crate provides:
//! - C++ source parsing via libclang
//! - Clang AST traversal and extraction
//! - Conversion from Clang AST to rustc MIR representation
//!
//! # Architecture
//!
//! ```text
//! C++ Source → libclang → Clang AST → MIR Bodies
//! ```

mod parse;
mod convert;
mod ast;
mod types;
mod resolve;
mod deduce;

pub use parse::ClangParser;
pub use convert::MirConverter;
pub use ast::{AccessSpecifier, ClangAst, ClangNode, ClangNodeKind, ConstructorKind, Requirement, TypeTraitKind};
pub use types::{CppType, TypeProperties, TypeTraitResult, TypeTraitEvaluator};
pub use resolve::NameResolver;
pub use deduce::{DeductionError, TypeDeducer};

use miette::Result;
use std::path::Path;

/// Parse a C++ source file and convert to MIR bodies.
///
/// Returns a map of function names to their MIR representations.
/// Automatically applies name resolution to resolve unqualified function calls.
pub fn compile_cpp_file(path: &Path) -> Result<CppModule> {
    let parser = ClangParser::new()?;
    let ast = parser.parse_file(path)?;
    let converter = MirConverter::new();
    let mut module = converter.convert(ast)?;
    module.resolve_names();
    Ok(module)
}

/// A compiled C++ module containing function MIR bodies.
#[derive(Debug)]
pub struct CppModule {
    /// Function definitions with their MIR bodies
    pub functions: Vec<CppFunction>,
    /// Struct/class definitions
    pub structs: Vec<CppStruct>,
    /// Extern declarations (no body)
    pub externs: Vec<CppExtern>,
    /// Function template declarations
    pub function_templates: Vec<CppFunctionTemplate>,
    /// Class template declarations
    pub class_templates: Vec<CppClassTemplate>,
    /// Class template partial specializations
    pub class_partial_specializations: Vec<CppClassTemplatePartialSpec>,
    /// Using namespace directives (for name resolution)
    pub using_directives: Vec<UsingDirective>,
    /// Using declarations (specific name imports)
    pub using_declarations: Vec<UsingDeclaration>,
    /// Concept definitions (C++20)
    pub concepts: Vec<CppConceptDecl>,
    /// Type aliases (using declarations and typedefs)
    pub type_aliases: Vec<CppTypeAlias>,
}

/// A using namespace directive.
#[derive(Debug, Clone)]
pub struct UsingDirective {
    /// The namespace path being imported (e.g., ["std"] or ["rrr", "base"])
    pub namespace: Vec<String>,
    /// Scope where this directive appears (e.g., ["outer"] if inside namespace outer)
    pub scope: Vec<String>,
}

/// A using declaration for a specific name.
#[derive(Debug, Clone)]
pub struct UsingDeclaration {
    /// The fully qualified name being imported (e.g., ["std", "cout"])
    pub qualified_name: Vec<String>,
    /// Scope where this declaration appears
    pub scope: Vec<String>,
}

impl CppModule {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            structs: Vec::new(),
            externs: Vec::new(),
            function_templates: Vec::new(),
            class_templates: Vec::new(),
            class_partial_specializations: Vec::new(),
            using_directives: Vec::new(),
            using_declarations: Vec::new(),
            concepts: Vec::new(),
            type_aliases: Vec::new(),
        }
    }

    /// Apply name resolution to all function calls in MIR bodies.
    ///
    /// This post-processing step resolves unqualified function names to their
    /// fully qualified forms using the stored using directives and declarations.
    pub fn resolve_names(&mut self) {
        use crate::resolve::NameResolver;

        // First pass: collect all resolutions needed (avoiding borrow conflicts)
        let mut function_resolutions: Vec<(usize, Vec<(usize, String)>)> = Vec::new();
        let mut struct_method_resolutions: Vec<(usize, Vec<(usize, usize, String)>)> = Vec::new();
        let mut struct_ctor_resolutions: Vec<(usize, Vec<(usize, usize, String)>)> = Vec::new();
        let mut struct_dtor_resolutions: Vec<(usize, Vec<(usize, String)>)> = Vec::new();

        {
            let resolver = NameResolver::new(self);

            // Collect function resolutions
            for (func_idx, func) in self.functions.iter().enumerate() {
                let scope = &func.namespace;
                let resolutions = Self::collect_mir_resolutions(&func.mir_body, &resolver, scope);
                if !resolutions.is_empty() {
                    function_resolutions.push((func_idx, resolutions));
                }
            }

            // Collect struct method resolutions
            for (st_idx, st) in self.structs.iter().enumerate() {
                let mut scope = st.namespace.clone();
                scope.push(st.name.clone());

                let mut method_res = Vec::new();
                for (method_idx, method) in st.methods.iter().enumerate() {
                    if let Some(ref mir_body) = method.mir_body {
                        for (block_idx, resolved) in
                            Self::collect_mir_resolutions(mir_body, &resolver, &scope)
                        {
                            method_res.push((method_idx, block_idx, resolved));
                        }
                    }
                }
                if !method_res.is_empty() {
                    struct_method_resolutions.push((st_idx, method_res));
                }

                let mut ctor_res = Vec::new();
                for (ctor_idx, ctor) in st.constructors.iter().enumerate() {
                    if let Some(ref mir_body) = ctor.mir_body {
                        for (block_idx, resolved) in
                            Self::collect_mir_resolutions(mir_body, &resolver, &scope)
                        {
                            ctor_res.push((ctor_idx, block_idx, resolved));
                        }
                    }
                }
                if !ctor_res.is_empty() {
                    struct_ctor_resolutions.push((st_idx, ctor_res));
                }

                let mut dtor_res = Vec::new();
                if let Some(ref dtor) = st.destructor {
                    if let Some(ref mir_body) = dtor.mir_body {
                        dtor_res = Self::collect_mir_resolutions(mir_body, &resolver, &scope);
                    }
                }
                if !dtor_res.is_empty() {
                    struct_dtor_resolutions.push((st_idx, dtor_res));
                }
            }
        }

        // Second pass: apply resolutions
        for (func_idx, resolutions) in function_resolutions {
            for (block_idx, resolved) in resolutions {
                if let MirTerminator::Call { func, .. } =
                    &mut self.functions[func_idx].mir_body.blocks[block_idx].terminator
                {
                    *func = resolved;
                }
            }
        }

        for (st_idx, resolutions) in struct_method_resolutions {
            for (method_idx, block_idx, resolved) in resolutions {
                if let Some(ref mut mir_body) = self.structs[st_idx].methods[method_idx].mir_body {
                    if let MirTerminator::Call { func, .. } =
                        &mut mir_body.blocks[block_idx].terminator
                    {
                        *func = resolved;
                    }
                }
            }
        }

        for (st_idx, resolutions) in struct_ctor_resolutions {
            for (ctor_idx, block_idx, resolved) in resolutions {
                if let Some(ref mut mir_body) = self.structs[st_idx].constructors[ctor_idx].mir_body
                {
                    if let MirTerminator::Call { func, .. } =
                        &mut mir_body.blocks[block_idx].terminator
                    {
                        *func = resolved;
                    }
                }
            }
        }

        for (st_idx, resolutions) in struct_dtor_resolutions {
            if let Some(ref mut dtor) = self.structs[st_idx].destructor {
                if let Some(ref mut mir_body) = dtor.mir_body {
                    for (block_idx, resolved) in resolutions {
                        if let MirTerminator::Call { func, .. } =
                            &mut mir_body.blocks[block_idx].terminator
                        {
                            *func = resolved;
                        }
                    }
                }
            }
        }
    }

    /// Collect name resolutions needed for a MIR body.
    fn collect_mir_resolutions(
        mir_body: &MirBody,
        resolver: &resolve::NameResolver,
        scope: &[String],
    ) -> Vec<(usize, String)> {
        let mut resolutions = Vec::new();
        for (block_idx, block) in mir_body.blocks.iter().enumerate() {
            if let MirTerminator::Call { func, .. } = &block.terminator {
                if let Some(qualified) = resolver.resolve_function(func, scope) {
                    resolutions.push((
                        block_idx,
                        resolve::NameResolver::format_qualified_name(&qualified),
                    ));
                }
            }
        }
        resolutions
    }
}

impl Default for CppModule {
    fn default() -> Self {
        Self::new()
    }
}

/// A C++ function with its MIR body.
#[derive(Debug, Clone)]
pub struct CppFunction {
    /// Mangled name for linking
    pub mangled_name: String,
    /// Human-readable name
    pub display_name: String,
    /// Namespace path (e.g., ["rrr", "nested"])
    pub namespace: Vec<String>,
    /// Parameter types
    pub params: Vec<(String, CppType)>,
    /// Return type
    pub return_type: CppType,
    /// Whether the function is noexcept
    pub is_noexcept: bool,
    /// MIR body (serialized for transfer to rustc driver)
    pub mir_body: MirBody,
}

/// A C++ function template declaration.
#[derive(Debug)]
pub struct CppFunctionTemplate {
    /// Template name (e.g., "identity")
    pub name: String,
    /// Namespace path
    pub namespace: Vec<String>,
    /// Template type parameters (e.g., ["T", "U"])
    pub template_params: Vec<String>,
    /// Return type (may reference template params)
    pub return_type: CppType,
    /// Parameters (may reference template params)
    pub params: Vec<(String, CppType)>,
    /// Whether this template has a definition
    pub is_definition: bool,
    /// Whether the function is noexcept
    pub is_noexcept: bool,
    /// Explicit specializations of this template
    pub specializations: Vec<CppTemplateSpecialization>,
    /// Indices of template parameters that are parameter packs (variadic)
    pub parameter_pack_indices: Vec<usize>,
    /// Optional requires clause constraint (C++20)
    pub requires_clause: Option<String>,
}

/// An explicit specialization of a function template.
#[derive(Debug)]
pub struct CppTemplateSpecialization {
    /// Template arguments for this specialization (e.g., [Int] for identity<int>)
    pub args: Vec<CppType>,
    /// The specialized function implementation
    pub function: CppFunction,
}

impl CppFunctionTemplate {
    /// Check if this template has any parameter packs (is variadic).
    pub fn is_variadic(&self) -> bool {
        !self.parameter_pack_indices.is_empty()
    }

    /// Check if a template parameter at the given index is a parameter pack.
    pub fn is_pack_param(&self, index: usize) -> bool {
        self.parameter_pack_indices.contains(&index)
    }

    /// Find an explicit specialization matching the given type arguments.
    pub fn find_specialization(&self, args: &[CppType]) -> Option<&CppFunction> {
        self.specializations
            .iter()
            .find(|spec| spec.args == args)
            .map(|spec| &spec.function)
    }

    /// Add an explicit specialization to this template.
    pub fn add_specialization(&mut self, args: Vec<CppType>, function: CppFunction) {
        self.specializations.push(CppTemplateSpecialization { args, function });
    }

    /// Instantiate this template with concrete type substitutions.
    ///
    /// Given a mapping of template parameter names to concrete types,
    /// produces a concrete function with all template parameters replaced.
    ///
    /// # Example
    /// ```ignore
    /// // template<typename T> T identity(T x);
    /// // instantiate with T = int → int identity(int x);
    /// ```
    pub fn instantiate(
        &self,
        substitutions: &std::collections::HashMap<String, CppType>,
    ) -> CppFunction {
        // Generate a mangled name that includes the template arguments
        let type_args: Vec<String> = self
            .template_params
            .iter()
            .filter_map(|p| substitutions.get(p).map(|t| t.to_rust_type_str()))
            .collect();
        let mangled_suffix = if type_args.is_empty() {
            String::new()
        } else {
            format!("<{}>", type_args.join(", "))
        };

        CppFunction {
            mangled_name: format!("{}{}", self.name, mangled_suffix),
            display_name: self.name.clone(),
            namespace: self.namespace.clone(),
            return_type: self.return_type.substitute(substitutions),
            params: self
                .params
                .iter()
                .map(|(name, ty)| (name.clone(), ty.substitute(substitutions)))
                .collect(),
            is_noexcept: self.is_noexcept,
            mir_body: MirBody::default(),
        }
    }

    /// Deduce template arguments from call argument types and instantiate.
    ///
    /// This is a convenience method that combines deduction and instantiation.
    /// If an explicit specialization matches the deduced types, it is used instead.
    ///
    /// # Example
    /// ```ignore
    /// // template<typename T> T identity(T x);
    /// // identity(42) → deduces T = int, uses specialization if available
    /// ```
    pub fn deduce_and_instantiate(
        &self,
        arg_types: &[CppType],
    ) -> std::result::Result<CppFunction, DeductionError> {
        let substitutions = TypeDeducer::deduce(self, arg_types)?;

        // Build ordered type args for specialization lookup
        let type_args: Vec<CppType> = self
            .template_params
            .iter()
            .filter_map(|p| substitutions.get(p).cloned())
            .collect();

        // Check for explicit specialization first
        if let Some(spec) = self.find_specialization(&type_args) {
            return Ok(spec.clone());
        }

        Ok(self.instantiate(&substitutions))
    }

    /// Deduce template arguments with some explicit and instantiate.
    ///
    /// Explicit arguments are applied first (in template parameter order),
    /// then remaining parameters are deduced from call arguments.
    /// If an explicit specialization matches, it is used instead.
    ///
    /// # Example
    /// ```ignore
    /// // template<typename T, typename U> T convert(U x);
    /// // convert<int>(3.14) → T = int (explicit), U = double (deduced)
    /// ```
    pub fn deduce_and_instantiate_with_explicit(
        &self,
        explicit_args: &[CppType],
        call_arg_types: &[CppType],
    ) -> std::result::Result<CppFunction, DeductionError> {
        let substitutions = TypeDeducer::deduce_with_explicit(self, explicit_args, call_arg_types)?;

        // Build ordered type args for specialization lookup
        let type_args: Vec<CppType> = self
            .template_params
            .iter()
            .filter_map(|p| substitutions.get(p).cloned())
            .collect();

        // Check for explicit specialization first
        if let Some(spec) = self.find_specialization(&type_args) {
            return Ok(spec.clone());
        }

        Ok(self.instantiate(&substitutions))
    }
}

/// A C++ struct/class definition.
#[derive(Debug)]
pub struct CppStruct {
    /// Type name
    pub name: String,
    /// Whether this is a class (vs struct)
    pub is_class: bool,
    /// Namespace path (e.g., ["rrr", "nested"])
    pub namespace: Vec<String>,
    /// Base classes (inheritance)
    pub bases: Vec<CppBaseClass>,
    /// Non-static fields with their types and access specifiers
    pub fields: Vec<CppField>,
    /// Static data members
    pub static_fields: Vec<CppField>,
    /// Constructors
    pub constructors: Vec<CppConstructor>,
    /// Destructor (at most one)
    pub destructor: Option<CppDestructor>,
    /// Methods (member functions)
    pub methods: Vec<CppMethod>,
    /// Member templates (template methods)
    pub member_templates: Vec<CppMemberTemplate>,
    /// Friend declarations
    pub friends: Vec<CppFriend>,
}

/// A C++ class template declaration.
/// Represents `template<typename T> class Box { ... }`
#[derive(Debug)]
pub struct CppClassTemplate {
    /// Template name (e.g., "Box")
    pub name: String,
    /// Whether this is a class (vs struct)
    pub is_class: bool,
    /// Namespace path (e.g., ["rrr", "nested"])
    pub namespace: Vec<String>,
    /// Template type parameters (e.g., ["T", "U"])
    pub template_params: Vec<String>,
    /// Non-static fields with their types (may reference template params)
    pub fields: Vec<CppField>,
    /// Static data members
    pub static_fields: Vec<CppField>,
    /// Constructors (may reference template params)
    pub constructors: Vec<CppConstructor>,
    /// Destructor (at most one)
    pub destructor: Option<CppDestructor>,
    /// Methods (may reference template params)
    pub methods: Vec<CppMethod>,
    /// Member templates (template methods, may reference both class and method template params)
    pub member_templates: Vec<CppMemberTemplate>,
    /// Indices of template parameters that are parameter packs (variadic)
    pub parameter_pack_indices: Vec<usize>,
    /// Optional requires clause constraint (C++20)
    pub requires_clause: Option<String>,
}

/// A partial specialization of a class template.
/// Represents `template<typename T> class Pair<T, T> { ... }`
#[derive(Debug)]
pub struct CppClassTemplatePartialSpec {
    /// Name of the primary template being specialized
    pub template_name: String,
    /// Whether this is a class (vs struct)
    pub is_class: bool,
    /// Namespace path
    pub namespace: Vec<String>,
    /// Template type parameters for this specialization (e.g., ["T"])
    pub template_params: Vec<String>,
    /// Specialization argument pattern (e.g., [TemplateParam("T"), TemplateParam("T")] for Pair<T,T>)
    pub specialization_args: Vec<CppType>,
    /// Non-static fields
    pub fields: Vec<CppField>,
    /// Static data members
    pub static_fields: Vec<CppField>,
    /// Constructors
    pub constructors: Vec<CppConstructor>,
    /// Destructor
    pub destructor: Option<CppDestructor>,
    /// Methods
    pub methods: Vec<CppMethod>,
    /// Member templates
    pub member_templates: Vec<CppMemberTemplate>,
    /// Indices of parameter packs
    pub parameter_pack_indices: Vec<usize>,
}

/// A C++ base class (for inheritance).
#[derive(Debug, Clone)]
pub struct CppBaseClass {
    /// Base class type
    pub base_type: CppType,
    /// Inheritance access specifier (public/protected/private)
    pub access: AccessSpecifier,
    /// Whether this is virtual inheritance
    pub is_virtual: bool,
}

/// A C++ class field (data member).
#[derive(Debug, Clone)]
pub struct CppField {
    /// Field name
    pub name: String,
    /// Field type
    pub ty: CppType,
    /// Access specifier
    pub access: AccessSpecifier,
}

/// A C++ class method (member function).
#[derive(Debug, Clone)]
pub struct CppMethod {
    /// Method name
    pub name: String,
    /// Return type
    pub return_type: CppType,
    /// Parameters
    pub params: Vec<(String, CppType)>,
    /// Whether this is a static method
    pub is_static: bool,
    /// Whether this is a virtual method
    pub is_virtual: bool,
    /// Whether this is a pure virtual method (= 0)
    pub is_pure_virtual: bool,
    /// Whether this method has the override specifier
    pub is_override: bool,
    /// Whether this method has the final specifier
    pub is_final: bool,
    /// Access specifier
    pub access: AccessSpecifier,
    /// MIR body (if this is a definition)
    pub mir_body: Option<MirBody>,
}

/// A member template (template method inside a class).
/// Represents `template<typename U> void process(U value);` inside a class.
#[derive(Debug)]
pub struct CppMemberTemplate {
    /// Method name
    pub name: String,
    /// Template type parameters (e.g., ["U"])
    pub template_params: Vec<String>,
    /// Return type (may reference template params)
    pub return_type: CppType,
    /// Parameters (may reference template params)
    pub params: Vec<(String, CppType)>,
    /// Access specifier
    pub access: AccessSpecifier,
    /// Whether this is a static method
    pub is_static: bool,
    /// Indices of template parameters that are parameter packs
    pub parameter_pack_indices: Vec<usize>,
    /// Whether this is a definition (has body)
    pub is_definition: bool,
}

/// A C++ constructor.
#[derive(Debug, Clone)]
pub struct CppConstructor {
    /// Constructor parameters
    pub params: Vec<(String, CppType)>,
    /// Constructor kind (default, copy, move, or other)
    pub kind: ConstructorKind,
    /// Access specifier
    pub access: AccessSpecifier,
    /// Member initializer list
    pub member_initializers: Vec<MemberInitializer>,
    /// MIR body (if this is a definition)
    pub mir_body: Option<MirBody>,
}

/// A C++ member initializer (e.g., `x(value)` in `: x(value)`).
#[derive(Debug, Clone)]
pub struct MemberInitializer {
    /// The member being initialized
    pub member_name: String,
    /// Whether this was directly initialized (not default)
    pub has_init: bool,
}

/// A C++ destructor.
#[derive(Debug, Clone)]
pub struct CppDestructor {
    /// Access specifier
    pub access: AccessSpecifier,
    /// MIR body (if this is a definition)
    pub mir_body: Option<MirBody>,
}

/// A C++ friend declaration.
#[derive(Debug, Clone)]
pub enum CppFriend {
    /// Friend class (e.g., `friend class Bar;`)
    Class { name: String },
    /// Friend function (e.g., `friend void helper(Foo&);`)
    Function { name: String },
}

/// An extern declaration (function without body).
#[derive(Debug)]
pub struct CppExtern {
    /// Mangled name for linking
    pub mangled_name: String,
    /// Human-readable name
    pub display_name: String,
    /// Namespace path (e.g., ["rrr", "nested"])
    pub namespace: Vec<String>,
    /// Parameter types
    pub params: Vec<(String, CppType)>,
    /// Return type
    pub return_type: CppType,
}

/// A C++20 concept definition.
/// Represents `template<typename T> concept Integral = ...;`
#[derive(Debug, Clone)]
pub struct CppConceptDecl {
    /// Concept name (e.g., "Integral")
    pub name: String,
    /// Namespace path (e.g., ["std"] for std::integral)
    pub namespace: Vec<String>,
    /// Template type parameters (e.g., ["T"])
    pub template_params: Vec<String>,
    /// The constraint expression as text (for display/debugging)
    pub constraint_expr: String,
}

/// A C++ type alias (using or typedef).
/// Represents `using IntAlias = int;` or `typedef int IntAlias;`
#[derive(Debug, Clone)]
pub struct CppTypeAlias {
    /// Alias name (e.g., "IntAlias")
    pub name: String,
    /// Namespace path (e.g., ["std"] for std::string)
    pub namespace: Vec<String>,
    /// The underlying type being aliased
    pub underlying_type: CppType,
    /// Whether this is a template alias
    pub is_template: bool,
    /// Template type parameters (only for template aliases)
    pub template_params: Vec<String>,
}

/// Serialized MIR body that can be transferred to rustc driver.
///
/// This is an intermediate representation that will be converted to actual
/// rustc MIR in the fragile-rustc-driver crate.
#[derive(Debug, Clone)]
pub struct MirBody {
    /// Basic blocks in the MIR
    pub blocks: Vec<MirBasicBlock>,
    /// Local variable declarations
    pub locals: Vec<MirLocal>,
    /// Whether this is a coroutine body (contains co_await, co_yield, or co_return)
    pub is_coroutine: bool,
}

impl MirBody {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            locals: Vec::new(),
            is_coroutine: false,
        }
    }
}

impl Default for MirBody {
    fn default() -> Self {
        Self::new()
    }
}

/// A basic block in MIR.
#[derive(Debug, Clone)]
pub struct MirBasicBlock {
    /// Statements in this block
    pub statements: Vec<MirStatement>,
    /// Terminator instruction
    pub terminator: MirTerminator,
    /// Whether this is a cleanup block (landing pad for unwinding)
    pub is_cleanup: bool,
}

/// A statement in MIR (no control flow).
#[derive(Debug, Clone)]
pub enum MirStatement {
    /// Assign a value to a local
    Assign {
        target: MirPlace,
        value: MirRvalue,
    },
    /// No-op
    Nop,
}

/// A terminator instruction (control flow).
#[derive(Debug, Clone)]
pub enum MirTerminator {
    /// Return from function
    Return,
    /// Unconditional jump
    Goto { target: usize },
    /// Conditional branch
    SwitchInt {
        operand: MirOperand,
        targets: Vec<(i128, usize)>,
        otherwise: usize,
    },
    /// Function call
    Call {
        func: String,
        args: Vec<MirOperand>,
        destination: MirPlace,
        target: Option<usize>,
        /// Cleanup block for unwinding (stack unwinding support)
        unwind: Option<usize>,
    },
    /// Unreachable
    Unreachable,
    /// Resume unwinding (used in cleanup blocks after running destructors)
    Resume,

    // C++20 Coroutine terminators

    /// Yield from a coroutine (co_yield in C++).
    /// Suspends the coroutine and yields a value.
    Yield {
        /// Value being yielded
        value: MirOperand,
        /// Block to resume at after yield
        resume: usize,
        /// Block for when the coroutine is dropped while suspended
        drop: Option<usize>,
    },

    /// Await an awaitable (co_await in C++).
    /// Suspends the coroutine until the awaitable is ready.
    Await {
        /// The awaitable being awaited
        awaitable: MirOperand,
        /// Destination place for the await result
        destination: MirPlace,
        /// Block to resume at after await completes
        resume: usize,
        /// Block for when the coroutine is dropped while suspended
        drop: Option<usize>,
    },

    /// Return from a coroutine (co_return in C++).
    CoroutineReturn {
        /// Optional value being returned (None for `co_return;`)
        value: Option<MirOperand>,
    },
}

/// An rvalue (right-hand side of assignment).
#[derive(Debug, Clone)]
pub enum MirRvalue {
    /// Use an operand directly
    Use(MirOperand),
    /// Binary operation
    BinaryOp {
        op: MirBinOp,
        left: MirOperand,
        right: MirOperand,
    },
    /// Unary operation
    UnaryOp {
        op: MirUnaryOp,
        operand: MirOperand,
    },
    /// Take address of a place
    Ref { place: MirPlace, mutability: bool },
}

/// An operand (something that can be used as input).
#[derive(Debug, Clone)]
pub enum MirOperand {
    /// Copy from a place
    Copy(MirPlace),
    /// Move from a place
    Move(MirPlace),
    /// A constant value
    Constant(MirConstant),
}

/// A place (memory location).
#[derive(Debug, Clone)]
pub struct MirPlace {
    /// Local variable index
    pub local: usize,
    /// Projection elements (field access, deref, index)
    pub projection: Vec<MirProjection>,
}

impl MirPlace {
    pub fn local(local: usize) -> Self {
        Self {
            local,
            projection: Vec::new(),
        }
    }
}

/// Projection element for places.
#[derive(Debug, Clone)]
pub enum MirProjection {
    /// Dereference a pointer
    Deref,
    /// Access a field by index
    Field(usize),
    /// Index into an array
    Index(usize),
}

/// A local variable declaration.
#[derive(Debug, Clone)]
pub struct MirLocal {
    /// Variable name (for debugging)
    pub name: Option<String>,
    /// Type of the local
    pub ty: CppType,
    /// Is this a function argument?
    pub is_arg: bool,
}

/// A constant value.
#[derive(Debug, Clone)]
pub enum MirConstant {
    /// Integer constant
    Int {
        value: i128,
        bits: u32,
        /// Whether this is a signed integer type.
        /// true = signed (i8, i16, i32, i64, i128)
        /// false = unsigned (u8, u16, u32, u64, u128)
        signed: bool,
    },
    /// Floating-point constant
    Float { value: f64, bits: u32 },
    /// Boolean constant
    Bool(bool),
    /// Unit/void
    Unit,
}

/// Binary operations.
#[derive(Debug, Clone, Copy)]
pub enum MirBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Unary operations.
#[derive(Debug, Clone, Copy)]
pub enum MirUnaryOp {
    Neg,
    Not,
}
