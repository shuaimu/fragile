use fragile_common::{Span, Symbol};
use crate::types::{Type, TypeParam, Field, StructDef, Mutability};
use crate::stmt::Stmt;
use crate::expr::Expr;

/// Visibility of an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Private,
    Public,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: Symbol,
    pub ty: Type,
    pub mutability: Mutability,
    pub span: Span,
}

/// A function signature.
#[derive(Debug, Clone)]
pub struct FnSig {
    pub params: Vec<Param>,
    pub ret_ty: Type,
    pub is_variadic: bool,
}

/// ABI specification for functions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Abi {
    /// Default ABI (Rust calling convention)
    #[default]
    Rust,
    /// C calling convention
    C,
    /// Other ABI (name stored)
    Other(String),
}

/// A function definition.
#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: Symbol,
    pub vis: Visibility,
    pub type_params: Vec<TypeParam>,
    pub sig: FnSig,
    pub body: Option<Expr>,
    pub span: Span,
    /// The source language of this function.
    pub source_lang: SourceLang,
    /// The ABI for this function (None = default Rust ABI, Some("C") = extern "C")
    pub abi: Abi,
}

/// The source language a construct came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLang {
    Rust,
    Cpp,
    Go,
}

/// An enum variant.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: Symbol,
    pub fields: Vec<Field>,
    pub discriminant: Option<i128>,
}

/// An enum definition.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: Symbol,
    pub vis: Visibility,
    pub type_params: Vec<TypeParam>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// A trait/interface method.
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: Symbol,
    pub sig: FnSig,
    pub has_default: bool,
}

/// A trait/interface definition.
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: Symbol,
    pub vis: Visibility,
    pub type_params: Vec<TypeParam>,
    pub methods: Vec<TraitMethod>,
    pub span: Span,
}

/// An impl block.
#[derive(Debug, Clone)]
pub struct ImplDef {
    pub type_params: Vec<TypeParam>,
    pub trait_ref: Option<Symbol>,
    pub self_ty: Type,
    pub items: Vec<Item>,
    pub span: Span,
}

/// A type alias.
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: Symbol,
    pub vis: Visibility,
    pub type_params: Vec<TypeParam>,
    pub ty: Type,
    pub span: Span,
}

/// A constant definition.
#[derive(Debug, Clone)]
pub struct ConstDef {
    pub name: Symbol,
    pub vis: Visibility,
    pub ty: Type,
    pub value: Expr,
    pub span: Span,
}

/// A static variable definition.
#[derive(Debug, Clone)]
pub struct StaticDef {
    pub name: Symbol,
    pub vis: Visibility,
    pub mutability: Mutability,
    pub ty: Type,
    pub init: Option<Expr>,
    pub span: Span,
}

/// A top-level item.
#[derive(Debug, Clone)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    Function(FnDef),
    Struct(StructDef),
    Enum(EnumDef),
    Trait(TraitDef),
    Impl(ImplDef),
    TypeAlias(TypeAlias),
    Const(ConstDef),
    Static(StaticDef),
    // TODO: Use, Mod, ExternBlock
}

impl Item {
    pub fn new(kind: ItemKind, span: Span) -> Self {
        Self { kind, span }
    }
}
