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

pub use parse::ClangParser;
pub use convert::MirConverter;
pub use ast::{AccessSpecifier, ClangAst, ClangNode, ClangNodeKind, ConstructorKind};
pub use types::CppType;

use miette::Result;
use std::path::Path;

/// Parse a C++ source file and convert to MIR bodies.
///
/// Returns a map of function names to their MIR representations.
pub fn compile_cpp_file(path: &Path) -> Result<CppModule> {
    let parser = ClangParser::new()?;
    let ast = parser.parse_file(path)?;
    let converter = MirConverter::new();
    converter.convert(ast)
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
    /// Using namespace directives (for name resolution)
    pub using_directives: Vec<UsingDirective>,
    /// Using declarations (specific name imports)
    pub using_declarations: Vec<UsingDeclaration>,
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
            using_directives: Vec::new(),
            using_declarations: Vec::new(),
        }
    }
}

impl Default for CppModule {
    fn default() -> Self {
        Self::new()
    }
}

/// A C++ function with its MIR body.
#[derive(Debug)]
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
    /// MIR body (serialized for transfer to rustc driver)
    pub mir_body: MirBody,
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
    /// Fields with their types and access specifiers
    pub fields: Vec<(String, CppType, AccessSpecifier)>,
    /// Constructors
    pub constructors: Vec<CppConstructor>,
    /// Destructor (at most one)
    pub destructor: Option<CppDestructor>,
    /// Methods (converted to associated functions)
    pub methods: Vec<CppFunction>,
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
}

impl MirBody {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            locals: Vec::new(),
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
    },
    /// Unreachable
    Unreachable,
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
    Int { value: i128, bits: u32 },
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
