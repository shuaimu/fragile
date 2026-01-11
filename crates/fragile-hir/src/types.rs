use fragile_common::Symbol;

/// Primitive types common across all languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    // Integers
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    // Floats
    F32,
    F64,
    // Other
    Bool,
    Char,
    Unit,    // Rust's (), Go's struct{}, C++ void (in some contexts)
    Never,   // Rust's !, diverging
}

/// Mutability qualifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mutability {
    #[default]
    Immutable,
    Mutable,
}

/// A type in the HIR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Primitive types (i32, bool, etc.)
    Primitive(PrimitiveType),

    /// Pointer type (*const T, *mut T, *T in Go, T* in C++)
    Pointer {
        inner: Box<Type>,
        mutability: Mutability,
    },

    /// Reference type (&T, &mut T in Rust)
    Reference {
        inner: Box<Type>,
        mutability: Mutability,
        // TODO: lifetimes
    },

    /// Fixed-size array [T; N]
    Array {
        inner: Box<Type>,
        size: u64,
    },

    /// Slice type [T] (Rust), []T (Go)
    Slice {
        inner: Box<Type>,
    },

    /// Function type fn(A, B) -> C
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
        is_variadic: bool,
    },

    /// Named type (struct, enum, interface, etc.)
    Named {
        name: Symbol,
        type_args: Vec<Type>,
    },

    /// Tuple type (A, B, C)
    Tuple(Vec<Type>),

    /// Type variable (for inference)
    Infer(u32),

    /// Error type (for error recovery)
    Error,
}

impl Type {
    pub fn unit() -> Self {
        Type::Primitive(PrimitiveType::Unit)
    }

    pub fn bool() -> Self {
        Type::Primitive(PrimitiveType::Bool)
    }

    pub fn i32() -> Self {
        Type::Primitive(PrimitiveType::I32)
    }

    pub fn i64() -> Self {
        Type::Primitive(PrimitiveType::I64)
    }

    pub fn f64() -> Self {
        Type::Primitive(PrimitiveType::F64)
    }

    pub fn ptr(inner: Type, mutability: Mutability) -> Self {
        Type::Pointer {
            inner: Box::new(inner),
            mutability,
        }
    }

    pub fn slice(inner: Type) -> Self {
        Type::Slice {
            inner: Box::new(inner),
        }
    }
}

/// A struct field.
#[derive(Debug, Clone)]
pub struct Field {
    pub name: Symbol,
    pub ty: Type,
    pub is_public: bool,
}

/// A struct definition.
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: Symbol,
    pub fields: Vec<Field>,
    pub type_params: Vec<TypeParam>,
}

/// A type parameter (generic).
#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: Symbol,
    pub bounds: Vec<TraitBound>,
}

/// A trait bound.
#[derive(Debug, Clone)]
pub struct TraitBound {
    pub trait_name: Symbol,
    pub type_args: Vec<Type>,
}
