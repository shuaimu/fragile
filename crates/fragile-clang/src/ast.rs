//! Clang AST representation.
//!
//! This module provides a simplified view of the Clang AST that's easier to
//! work with for MIR conversion.

use crate::types::CppType;

/// A parsed Clang AST.
#[derive(Debug)]
pub struct ClangAst {
    /// Root translation unit
    pub translation_unit: ClangNode,
}

/// A node in the Clang AST.
#[derive(Debug)]
pub struct ClangNode {
    /// Kind of this node
    pub kind: ClangNodeKind,
    /// Child nodes
    pub children: Vec<ClangNode>,
    /// Source location info (for error messages)
    pub location: SourceLocation,
}

/// Source location for error reporting.
#[derive(Debug, Clone, Default)]
pub struct SourceLocation {
    pub file: Option<String>,
    pub line: u32,
    pub column: u32,
}

/// C++ access specifier for class members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessSpecifier {
    /// Public access - accessible from anywhere
    Public,
    /// Private access - accessible only from within the class
    #[default]
    Private,
    /// Protected access - accessible from class and derived classes
    Protected,
}

/// C++ constructor kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConstructorKind {
    /// Default constructor (no parameters or all defaulted)
    Default,
    /// Copy constructor (takes const T&)
    Copy,
    /// Move constructor (takes T&&)
    Move,
    /// Other constructor (parameterized, converting, etc.)
    #[default]
    Other,
}

/// Kinds of Clang AST nodes we care about.
#[derive(Debug)]
pub enum ClangNodeKind {
    /// Translation unit (root)
    TranslationUnit,

    // Declarations
    /// Function declaration/definition
    FunctionDecl {
        name: String,
        return_type: CppType,
        params: Vec<(String, CppType)>,
        is_definition: bool,
        /// Whether the function is declared noexcept
        is_noexcept: bool,
    },
    /// Function template declaration
    FunctionTemplateDecl {
        name: String,
        /// Template type parameters (e.g., ["T", "U"] for template<typename T, typename U>)
        template_params: Vec<String>,
        /// Return type (may be dependent on template params)
        return_type: CppType,
        /// Parameters (may be dependent on template params)
        params: Vec<(String, CppType)>,
        is_definition: bool,
        /// Indices of template parameters that are parameter packs (variadic)
        parameter_pack_indices: Vec<usize>,
        /// Optional requires clause constraint (C++20)
        requires_clause: Option<String>,
        /// Whether the function is declared noexcept
        is_noexcept: bool,
    },
    /// Class template declaration (e.g., template<typename T> class Box { ... })
    ClassTemplateDecl {
        name: String,
        /// Template type parameters (e.g., ["T", "U"])
        template_params: Vec<String>,
        /// Whether this is a class (vs struct)
        is_class: bool,
        /// Indices of parameter packs
        parameter_pack_indices: Vec<usize>,
        /// Optional requires clause constraint (C++20)
        requires_clause: Option<String>,
    },
    /// Class template partial specialization (e.g., template<typename T> class Pair<T, T> { ... })
    ClassTemplatePartialSpecDecl {
        name: String,
        /// Template type parameters for this specialization
        template_params: Vec<String>,
        /// The specialization argument pattern as types
        specialization_args: Vec<CppType>,
        /// Whether this is a class (vs struct)
        is_class: bool,
        /// Indices of parameter packs
        parameter_pack_indices: Vec<usize>,
    },
    /// Template type parameter (e.g., typename T or typename... Args)
    TemplateTypeParmDecl {
        name: String,
        /// Depth in nested template declarations
        depth: u32,
        /// Index within the template parameter list
        index: u32,
        /// Whether this is a parameter pack (typename... Args)
        is_pack: bool,
    },
    /// Parameter declaration
    ParmVarDecl {
        name: String,
        ty: CppType,
    },
    /// Variable declaration
    VarDecl {
        name: String,
        ty: CppType,
        has_init: bool,
    },
    /// Struct/class declaration
    RecordDecl {
        name: String,
        is_class: bool,
        fields: Vec<(String, CppType)>,
    },
    /// Field declaration
    FieldDecl {
        name: String,
        ty: CppType,
        access: AccessSpecifier,
        is_static: bool,
    },
    /// C++ method declaration
    CXXMethodDecl {
        name: String,
        return_type: CppType,
        params: Vec<(String, CppType)>,
        is_definition: bool,
        is_static: bool,
        is_virtual: bool,
        is_pure_virtual: bool,
        is_override: bool,
        is_final: bool,
        access: AccessSpecifier,
    },
    /// Constructor declaration
    ConstructorDecl {
        class_name: String,
        params: Vec<(String, CppType)>,
        is_definition: bool,
        ctor_kind: ConstructorKind,
        access: AccessSpecifier,
    },
    /// Destructor declaration
    DestructorDecl {
        class_name: String,
        is_definition: bool,
        access: AccessSpecifier,
    },
    /// Member reference (e.g., in member initializer lists)
    MemberRef {
        name: String,
    },
    /// Friend declaration
    FriendDecl {
        /// Friend class name (if friend class)
        friend_class: Option<String>,
        /// Friend function name (if friend function)
        friend_function: Option<String>,
    },
    /// C++ base class specifier (inheritance)
    CXXBaseSpecifier {
        /// Base class type
        base_type: CppType,
        /// Inheritance access specifier (public/protected/private)
        access: AccessSpecifier,
        /// Whether this is virtual inheritance
        is_virtual: bool,
    },
    /// Namespace declaration
    NamespaceDecl {
        /// Namespace name (None for anonymous namespaces)
        name: Option<String>,
    },
    /// Language linkage specification (e.g., `extern "C" { ... }`)
    /// This is a container for declarations with different linkage.
    /// Children are the actual declarations.
    LinkageSpecDecl,
    /// Using namespace directive (e.g., `using namespace std;`)
    UsingDirective {
        /// The namespace path being imported (e.g., ["std"] or ["rrr", "base"])
        namespace: Vec<String>,
    },
    /// Using declaration (e.g., `using std::cout;`)
    UsingDeclaration {
        /// The fully qualified name being imported
        qualified_name: Vec<String>,
    },
    /// Type alias declaration (e.g., `using IntAlias = int;`)
    TypeAliasDecl {
        /// The alias name
        name: String,
        /// The underlying type
        underlying_type: CppType,
    },
    /// Type alias template declaration (e.g., `template<typename T> using Ptr = T*;`)
    TypeAliasTemplateDecl {
        /// The alias name
        name: String,
        /// Template type parameters
        template_params: Vec<String>,
        /// The underlying type (may reference template params)
        underlying_type: CppType,
    },
    /// Typedef declaration (old C-style, e.g., `typedef int IntAlias;`)
    TypedefDecl {
        /// The typedef name
        name: String,
        /// The underlying type
        underlying_type: CppType,
    },

    // Statements
    /// Compound statement (block)
    CompoundStmt,
    /// Return statement
    ReturnStmt,
    /// If statement
    IfStmt,
    /// While statement
    WhileStmt,
    /// For statement
    ForStmt,
    /// Declaration statement
    DeclStmt,
    /// Expression statement
    ExprStmt,
    /// Break statement
    BreakStmt,
    /// Continue statement
    ContinueStmt,
    /// Switch statement
    SwitchStmt,
    /// Case statement (case label with optional nested case or body)
    CaseStmt {
        /// The case value (constant expression)
        value: i128,
    },
    /// Default statement in switch
    DefaultStmt,

    // Expressions
    /// Integer literal
    IntegerLiteral(i128),
    /// Floating-point literal
    FloatingLiteral(f64),
    /// Boolean literal
    BoolLiteral(bool),
    /// String literal
    StringLiteral(String),
    /// Reference to a declared entity
    DeclRefExpr {
        name: String,
        ty: CppType,
    },
    /// Binary operator
    BinaryOperator {
        op: BinaryOp,
        ty: CppType,
    },
    /// Unary operator
    UnaryOperator {
        op: UnaryOp,
        ty: CppType,
    },
    /// Function call
    CallExpr {
        ty: CppType,
    },
    /// Member access (a.b or a->b)
    MemberExpr {
        member_name: String,
        is_arrow: bool,
        ty: CppType,
    },
    /// Array subscript (a[i])
    ArraySubscriptExpr {
        ty: CppType,
    },
    /// Cast expression
    CastExpr {
        cast_kind: CastKind,
        ty: CppType,
    },
    /// Conditional operator (a ? b : c)
    ConditionalOperator {
        ty: CppType,
    },
    /// Parenthesized expression
    ParenExpr {
        ty: CppType,
    },
    /// Implicit cast (inserted by compiler)
    ImplicitCastExpr {
        cast_kind: CastKind,
        ty: CppType,
    },

    /// Type trait expression (e.g., __is_integral(T), __is_same(T, U))
    /// These are Clang's built-in type trait intrinsics.
    TypeTraitExpr {
        /// The kind of type trait being evaluated
        trait_kind: TypeTraitKind,
        /// The type arguments to the trait
        type_args: Vec<CppType>,
    },

    // C++20 Concepts

    /// Concept definition (e.g., template<typename T> concept Integral = ...)
    ConceptDecl {
        /// Name of the concept
        name: String,
        /// Template type parameters (e.g., ["T"])
        template_params: Vec<String>,
        /// The constraint expression as text (for display/debugging)
        constraint_expr: String,
    },

    /// Requires expression (e.g., requires(T a) { a + a; })
    RequiresExpr {
        /// Parameters for the requires expression (may be empty)
        params: Vec<(String, CppType)>,
        /// Requirements inside the requires expression
        requirements: Vec<Requirement>,
    },

    /// Concept specialization expression (e.g., Integral<T> in requires clause)
    ConceptSpecializationExpr {
        /// Name of the concept being referenced
        concept_name: String,
        /// Template arguments to the concept
        template_args: Vec<CppType>,
    },

    // C++20 Coroutines

    /// co_await expression (C++20 coroutine)
    /// Suspends execution until the awaitable is ready.
    CoawaitExpr {
        /// Type of the operand being awaited
        operand_ty: CppType,
        /// Result type of the await expression
        result_ty: CppType,
    },

    /// co_yield expression (C++20 coroutine)
    /// Yields a value and suspends the coroutine.
    CoyieldExpr {
        /// Type of the value being yielded
        value_ty: CppType,
        /// Result type of the yield expression (from yield_value)
        result_ty: CppType,
    },

    /// co_return statement (C++20 coroutine)
    /// Returns from a coroutine, optionally with a value.
    CoreturnStmt {
        /// Type of the returned value (None for `co_return;`)
        value_ty: Option<CppType>,
    },

    // C++ Exception Handling

    /// try statement with catch handlers
    TryStmt,

    /// catch handler in a try statement
    CatchStmt {
        /// Type being caught (None for `catch(...)`)
        exception_ty: Option<CppType>,
    },

    /// throw expression
    ThrowExpr {
        /// Type being thrown (None for `throw;` rethrow)
        exception_ty: Option<CppType>,
    },

    // C++ RTTI (Run-Time Type Information)

    /// typeid expression (returns std::type_info const&)
    TypeidExpr {
        /// Type of the result (std::type_info const&)
        result_ty: CppType,
    },

    /// dynamic_cast expression
    DynamicCastExpr {
        /// Target type of the cast
        target_ty: CppType,
    },

    /// Unknown or unhandled node kind
    Unknown(String),
}

/// Kinds of built-in type traits (Clang intrinsics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeTraitKind {
    /// __is_integral(T) - checks if T is an integral type
    IsIntegral,
    /// __is_signed(T) - checks if T is a signed type
    IsSigned,
    /// __is_unsigned(T) - checks if T is an unsigned type
    IsUnsigned,
    /// __is_floating_point(T) - checks if T is a floating point type
    IsFloatingPoint,
    /// __is_arithmetic(T) - checks if T is arithmetic (integral or floating)
    IsArithmetic,
    /// __is_scalar(T) - checks if T is scalar (arithmetic, pointer, enum)
    IsScalar,
    /// __is_pointer(T) - checks if T is a pointer type
    IsPointer,
    /// __is_reference(T) - checks if T is a reference type
    IsReference,
    /// __is_same(T, U) - checks if T and U are the same type
    IsSame,
    /// __is_base_of(Base, Derived) - checks if Base is a base class of Derived
    IsBaseOf,
    /// __is_trivially_copyable(T) - checks if T is trivially copyable
    IsTriviallyCopyable,
    /// __is_trivially_destructible(T) - checks if T is trivially destructible
    IsTriviallyDestructible,
    /// Unknown/other type trait
    Unknown,
}

/// A single requirement inside a requires expression.
#[derive(Debug, Clone)]
pub enum Requirement {
    /// Simple requirement: expression must be valid (e.g., `a + b;`)
    Simple {
        /// The expression text
        expr: String,
    },
    /// Type requirement: type must exist (e.g., `typename T::value_type;`)
    Type {
        /// The type name/expression
        type_name: String,
    },
    /// Compound requirement: expr with optional noexcept and return type constraint
    /// (e.g., `{ a + b } -> std::same_as<T>;` or `{ a + b } noexcept;`)
    Compound {
        /// The expression text
        expr: String,
        /// Whether noexcept is required
        is_noexcept: bool,
        /// Optional return type constraint (e.g., "std::same_as<T>")
        return_constraint: Option<String>,
    },
    /// Nested requirement: requires clause inside requires (e.g., `requires Concept<T>;`)
    Nested {
        /// The nested constraint expression
        constraint: String,
    },
}

/// Binary operators.
#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    // Bitwise
    And,
    Or,
    Xor,
    Shl,
    Shr,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    LAnd,
    LOr,
    // Assignment
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    AndAssign,
    OrAssign,
    XorAssign,
    ShlAssign,
    ShrAssign,
    // Comma
    Comma,
}

/// Unary operators.
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    /// Prefix increment (++x)
    PreInc,
    /// Prefix decrement (--x)
    PreDec,
    /// Postfix increment (x++)
    PostInc,
    /// Postfix decrement (x--)
    PostDec,
    /// Address-of (&x)
    AddrOf,
    /// Dereference (*x)
    Deref,
    /// Unary plus (+x)
    Plus,
    /// Unary minus (-x)
    Minus,
    /// Bitwise not (~x)
    Not,
    /// Logical not (!x)
    LNot,
}

/// Cast kinds.
#[derive(Debug, Clone, Copy)]
pub enum CastKind {
    /// No-op cast (e.g., const removal for value)
    NoOp,
    /// Integral conversion (e.g., int to long)
    IntegralCast,
    /// Floating-point conversion
    FloatingCast,
    /// Float to int
    FloatingToIntegral,
    /// Int to float
    IntegralToFloating,
    /// Pointer to int
    PointerToIntegral,
    /// Int to pointer
    IntegralToPointer,
    /// Pointer to pointer
    BitCast,
    /// Array to pointer decay
    ArrayToPointerDecay,
    /// Function to pointer decay
    FunctionToPointerDecay,
    /// L-value to r-value conversion
    LValueToRValue,
    /// Null pointer to type
    NullToPointer,
    /// Unknown/other cast
    Other,
}

impl ClangNode {
    /// Create a new node with the given kind.
    pub fn new(kind: ClangNodeKind) -> Self {
        Self {
            kind,
            children: Vec::new(),
            location: SourceLocation::default(),
        }
    }

    /// Add a child node.
    pub fn with_child(mut self, child: ClangNode) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple child nodes.
    pub fn with_children(mut self, children: Vec<ClangNode>) -> Self {
        self.children = children;
        self
    }

    /// Set the source location.
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = location;
        self
    }
}
