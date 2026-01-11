use fragile_common::{Span, Symbol};
use crate::types::Type;
use crate::stmt::Stmt;

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
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
    And,
    Or,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,      // -x
    Not,      // !x
    Deref,    // *x
    AddrOf,   // &x
    AddrOfMut, // &mut x
}

/// A literal value.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i128),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Unit,
}

/// An expression in the HIR.
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
    pub ty: Option<Type>,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    /// A literal value
    Literal(Literal),

    /// A variable reference
    Ident(Symbol),

    /// Binary operation: a + b
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    /// Unary operation: -x, !x, *x, &x
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },

    /// Function call: f(a, b)
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },

    /// Method call: x.method(a, b)
    MethodCall {
        receiver: Box<Expr>,
        method: Symbol,
        args: Vec<Expr>,
    },

    /// Field access: x.field
    Field {
        expr: Box<Expr>,
        field: Symbol,
    },

    /// Index: x[i]
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },

    /// Block expression: { stmts; expr }
    Block {
        stmts: Vec<Stmt>,
        expr: Option<Box<Expr>>,
    },

    /// If expression: if cond { then } else { else }
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },

    /// Match expression (Rust), switch (Go/C++)
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    /// Loop: loop { body }
    Loop {
        body: Box<Expr>,
    },

    /// While loop: while cond { body }
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },

    /// For loop: for pat in iter { body }
    For {
        var: Symbol,
        iter: Box<Expr>,
        body: Box<Expr>,
    },

    /// Return: return expr
    Return(Option<Box<Expr>>),

    /// Break: break expr
    Break(Option<Box<Expr>>),

    /// Continue
    Continue,

    /// Array literal: [a, b, c]
    Array(Vec<Expr>),

    /// Tuple literal: (a, b, c)
    Tuple(Vec<Expr>),

    /// Struct literal: Foo { field: value }
    Struct {
        name: Symbol,
        fields: Vec<(Symbol, Expr)>,
    },

    /// Type cast: x as T
    Cast {
        expr: Box<Expr>,
        ty: Type,
    },

    /// Lambda/closure: |x| x + 1
    Lambda {
        params: Vec<(Symbol, Option<Type>)>,
        body: Box<Expr>,
    },

    /// Assignment: x = y
    Assign {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    /// Compound assignment: x += y
    AssignOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    /// Error expression (for error recovery)
    Error,
}

/// A match arm.
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

/// A pattern (for match, let, etc.).
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Wildcard: _
    Wildcard,
    /// Variable binding: x
    Ident(Symbol),
    /// Literal pattern: 42
    Literal(Literal),
    /// Tuple pattern: (a, b)
    Tuple(Vec<Pattern>),
    /// Struct pattern: Foo { a, b }
    Struct {
        name: Symbol,
        fields: Vec<(Symbol, Pattern)>,
    },
    /// Variant pattern: Some(x)
    Variant {
        name: Symbol,
        patterns: Vec<Pattern>,
    },
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span, ty: None }
    }
}
