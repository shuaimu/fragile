use fragile_common::{Span, Symbol};
use crate::types::{Type, TypeParam, Field, StructDef, Mutability};
use crate::stmt::Stmt;
use crate::expr::Expr;

/// Visibility of an item.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Visibility {
    /// Private to the current module
    #[default]
    Private,
    /// Public to everyone
    Public,
    /// Public within the crate (pub(crate))
    Crate,
    /// Public to the parent module (pub(super))
    Super,
    /// Public to a specific path (pub(in path))
    Restricted(Vec<Symbol>),
}

/// An attribute (e.g., #[inline], #[derive(Debug)])
#[derive(Debug, Clone)]
pub struct Attribute {
    /// The attribute name (e.g., "inline", "derive", "cfg")
    pub name: Symbol,
    /// Optional arguments (e.g., for #[derive(Debug, Clone)], args would be ["Debug", "Clone"])
    pub args: Vec<Symbol>,
    pub span: Span,
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
    /// Attributes on this function (e.g., #[inline], #[no_mangle])
    pub attributes: Vec<Attribute>,
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
/// An associated type in a trait.
#[derive(Debug, Clone)]
pub struct AssociatedType {
    pub name: Symbol,
    pub default_ty: Option<Type>,
}

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
    pub associated_types: Vec<AssociatedType>,
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

/// A use/import declaration.
#[derive(Debug, Clone)]
pub struct UseDef {
    /// The path being imported (e.g., ["std", "io"] for `use std::io`)
    pub path: Vec<Symbol>,
    /// Optional alias (e.g., `use Foo as Bar` has alias "Bar")
    pub alias: Option<Symbol>,
    /// Visibility (for `pub use`)
    pub vis: Visibility,
    pub span: Span,
}

/// A module declaration.
#[derive(Debug, Clone)]
pub struct ModDef {
    pub name: Symbol,
    pub vis: Visibility,
    /// If None, this is an external module (mod foo;)
    /// If Some, this is an inline module with items
    pub items: Option<Vec<Item>>,
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
    Use(UseDef),
    Mod(ModDef),
    // TODO: ExternBlock
}

impl Item {
    pub fn new(kind: ItemKind, span: Span) -> Self {
        Self { kind, span }
    }
}
