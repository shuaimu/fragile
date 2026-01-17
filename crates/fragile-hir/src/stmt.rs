use fragile_common::Span;
use crate::types::{Type, Mutability};
use crate::expr::{Expr, Pattern};
use crate::item::Item;

/// A statement in the HIR.
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    /// Let binding: let x = expr
    Let {
        pattern: Pattern,
        ty: Option<Type>,
        init: Option<Expr>,
        mutability: Mutability,
    },

    /// Expression statement: expr;
    Expr(Expr),

    /// Item statement (function inside function, etc.)
    Item(Box<Item>),

    /// Empty statement: ;
    Empty,
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn expr(e: Expr) -> Self {
        let span = e.span;
        Self { kind: StmtKind::Expr(e), span }
    }
}
