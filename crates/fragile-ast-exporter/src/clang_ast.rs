//! Clang AST representation in Rust
//!
//! This module deserializes the CBOR-encoded AST from the C++ exporter
//! into Rust data structures.

use serde_cbor::Value;
use std::collections::HashMap;
use std::fmt;
use std::io::{Error, ErrorKind};

use crate::ffi::ASTEntryTag;

/// Source location in a file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SrcLoc {
    pub file_id: u64,
    pub line: u64,
    pub column: u64,
}

impl fmt::Display for SrcLoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "file_{}:{}:{}", self.file_id, self.line, self.column)
    }
}

/// Source range spanning multiple locations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SrcSpan {
    pub file_id: u64,
    pub begin_line: u64,
    pub begin_column: u64,
    pub end_line: u64,
    pub end_column: u64,
}

impl SrcSpan {
    pub fn begin(&self) -> SrcLoc {
        SrcLoc {
            file_id: self.file_id,
            line: self.begin_line,
            column: self.begin_column,
        }
    }

    pub fn end(&self) -> SrcLoc {
        SrcLoc {
            file_id: self.file_id,
            line: self.end_line,
            column: self.end_column,
        }
    }
}

/// Whether an expression is an lvalue or rvalue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LRValue {
    LValue,
    RValue,
}

/// An AST node (declaration, statement, or expression)
#[derive(Debug, Clone)]
pub struct AstNode {
    /// The node's unique ID (pointer address from C++)
    pub id: u64,
    /// The kind of AST node
    pub tag: ASTEntryTag,
    /// Child node IDs
    pub children: Vec<Option<u64>>,
    /// Source location
    pub loc: SrcSpan,
    /// Type ID (for expressions)
    pub type_id: Option<u64>,
    /// Extra data specific to the node kind
    pub extras: Vec<Value>,
}

impl AstNode {
    /// Get a string extra at the given index
    pub fn get_string(&self, index: usize) -> Option<&str> {
        self.extras.get(index).and_then(|v| match v {
            Value::Text(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Get a bool extra at the given index
    pub fn get_bool(&self, index: usize) -> Option<bool> {
        self.extras.get(index).and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        })
    }

    /// Get an integer extra at the given index
    pub fn get_int(&self, index: usize) -> Option<i64> {
        self.extras.get(index).and_then(|v| match v {
            Value::Integer(i) => Some(*i as i64),
            _ => None,
        })
    }

    /// Get a u64 extra at the given index
    pub fn get_u64(&self, index: usize) -> Option<u64> {
        self.extras.get(index).and_then(|v| match v {
            Value::Integer(i) => Some(*i as u64),
            _ => None,
        })
    }
}

/// A type node
#[derive(Debug, Clone)]
pub struct TypeNode {
    /// The type's unique ID
    pub id: u64,
    /// The kind of type
    pub tag: ASTEntryTag,
    /// Extra data specific to the type kind
    pub extras: Vec<Value>,
}

impl TypeNode {
    // Masks for decoding qualified type IDs
    pub const ID_MASK: u64 = !0b111;
    pub const CONST_MASK: u64 = 0b001;
    pub const RESTRICT_MASK: u64 = 0b010;
    pub const VOLATILE_MASK: u64 = 0b100;

    /// Check if the type is const-qualified
    pub fn is_const(type_id: u64) -> bool {
        type_id & Self::CONST_MASK != 0
    }

    /// Check if the type is restrict-qualified
    pub fn is_restrict(type_id: u64) -> bool {
        type_id & Self::RESTRICT_MASK != 0
    }

    /// Check if the type is volatile-qualified
    pub fn is_volatile(type_id: u64) -> bool {
        type_id & Self::VOLATILE_MASK != 0
    }

    /// Get the unqualified type ID
    pub fn unqualified_id(type_id: u64) -> u64 {
        type_id & Self::ID_MASK
    }

    /// Get a string extra at the given index
    pub fn get_string(&self, index: usize) -> Option<&str> {
        self.extras.get(index).and_then(|v| match v {
            Value::Text(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Get a u64 extra at the given index (for type references)
    pub fn get_u64(&self, index: usize) -> Option<u64> {
        self.extras.get(index).and_then(|v| match v {
            Value::Integer(i) => Some(*i as u64),
            _ => None,
        })
    }
}

/// Template argument
#[derive(Debug, Clone)]
pub struct TemplateArg {
    /// Argument kind (from Clang)
    pub kind: u64,
    /// String representation
    pub value: String,
}

/// Template specialization info
#[derive(Debug, Clone)]
pub struct TemplateSpecialization {
    /// Name of the template
    pub name: String,
    /// Qualified name (includes template args)
    pub qualified_name: String,
    /// Template arguments
    pub args: Vec<TemplateArg>,
    /// Is implicit instantiation
    pub is_implicit: bool,
    /// Is explicit specialization
    pub is_explicit: bool,
}

/// Source file information
#[derive(Debug, Clone)]
pub struct SrcFile {
    pub path: Option<String>,
    pub include_loc: Option<SrcLoc>,
}

/// The complete AST context
#[derive(Debug)]
pub struct AstContext {
    /// All AST nodes indexed by ID
    pub ast_nodes: HashMap<u64, AstNode>,
    /// All type nodes indexed by ID
    pub type_nodes: HashMap<u64, TypeNode>,
    /// Top-level declaration IDs
    pub top_nodes: Vec<u64>,
    /// Source files
    pub files: Vec<SrcFile>,
}

impl AstContext {
    /// Get an AST node by ID
    pub fn get_node(&self, id: u64) -> Option<&AstNode> {
        self.ast_nodes.get(&id)
    }

    /// Get a type node by ID
    pub fn get_type(&self, id: u64) -> Option<&TypeNode> {
        let unqualified = TypeNode::unqualified_id(id);
        self.type_nodes.get(&unqualified)
    }

    /// Get all nodes of a specific tag
    pub fn nodes_with_tag(&self, tag: ASTEntryTag) -> impl Iterator<Item = &AstNode> {
        self.ast_nodes.values().filter(move |n| n.tag == tag)
    }

    /// Get all template specializations
    pub fn template_specializations(&self) -> impl Iterator<Item = &AstNode> {
        self.nodes_with_tag(ASTEntryTag::TagClassTemplateSpecializationDecl)
    }

    /// Get methods of a class/struct
    pub fn get_methods(&self, class_id: u64) -> Vec<&AstNode> {
        self.ast_nodes
            .values()
            .filter(|n| {
                matches!(
                    n.tag,
                    ASTEntryTag::TagCXXMethodDecl
                        | ASTEntryTag::TagCXXConstructorDecl
                        | ASTEntryTag::TagCXXDestructorDecl
                )
            })
            .filter(|n| {
                // Check if parent class matches
                n.extras.iter().any(|e| match e {
                    Value::Integer(i) => *i as u64 == class_id,
                    _ => false,
                })
            })
            .collect()
    }
}

// Helper functions for CBOR parsing
fn expect_u64(val: &Value) -> Result<u64, Error> {
    match val {
        Value::Integer(n) => Ok(*n as u64),
        _ => Err(Error::new(ErrorKind::InvalidData, "Expected integer")),
    }
}

fn expect_opt_u64(val: &Value) -> Result<Option<u64>, Error> {
    match val {
        Value::Null => Ok(None),
        Value::Integer(n) => Ok(Some(*n as u64)),
        _ => Err(Error::new(ErrorKind::InvalidData, "Expected integer or null")),
    }
}

fn import_ast_tag(tag: u64) -> ASTEntryTag {
    // Map known tag values to their enum variants
    // Unknown tags are mapped to TagTypeUnknown to avoid undefined behavior from transmute
    match tag as u32 {
        0 => ASTEntryTag::TagFunctionDecl,
        1 => ASTEntryTag::TagCXXMethodDecl,
        2 => ASTEntryTag::TagCXXConstructorDecl,
        3 => ASTEntryTag::TagCXXDestructorDecl,
        4 => ASTEntryTag::TagParmVarDecl,
        10 => ASTEntryTag::TagVarDecl,
        11 => ASTEntryTag::TagFieldDecl,
        20 => ASTEntryTag::TagCXXRecordDecl,
        21 => ASTEntryTag::TagClassTemplateDecl,
        22 => ASTEntryTag::TagClassTemplateSpecializationDecl,
        23 => ASTEntryTag::TagTypedefDecl,
        24 => ASTEntryTag::TagTypeAliasDecl,
        25 => ASTEntryTag::TagEnumDecl,
        26 => ASTEntryTag::TagEnumConstantDecl,
        30 => ASTEntryTag::TagNamespaceDecl,
        31 => ASTEntryTag::TagUsingDecl,
        32 => ASTEntryTag::TagUsingDirectiveDecl,
        40 => ASTEntryTag::TagTemplateTypeParmDecl,
        41 => ASTEntryTag::TagNonTypeTemplateParmDecl,
        42 => ASTEntryTag::TagTemplateTemplateParmDecl,
        43 => ASTEntryTag::TagFunctionTemplateDecl,
        50 => ASTEntryTag::TagAccessSpecDecl,
        60 => ASTEntryTag::TagStaticAssertDecl,
        100 => ASTEntryTag::TagCompoundStmt,
        101 => ASTEntryTag::TagDeclStmt,
        102 => ASTEntryTag::TagReturnStmt,
        103 => ASTEntryTag::TagIfStmt,
        104 => ASTEntryTag::TagWhileStmt,
        105 => ASTEntryTag::TagDoStmt,
        106 => ASTEntryTag::TagForStmt,
        107 => ASTEntryTag::TagCXXForRangeStmt,
        108 => ASTEntryTag::TagBreakStmt,
        109 => ASTEntryTag::TagContinueStmt,
        110 => ASTEntryTag::TagSwitchStmt,
        111 => ASTEntryTag::TagCaseStmt,
        112 => ASTEntryTag::TagDefaultStmt,
        113 => ASTEntryTag::TagGotoStmt,
        114 => ASTEntryTag::TagLabelStmt,
        115 => ASTEntryTag::TagNullStmt,
        116 => ASTEntryTag::TagCXXTryStmt,
        117 => ASTEntryTag::TagCXXCatchStmt,
        118 => ASTEntryTag::TagCXXThrowExpr,
        200 => ASTEntryTag::TagIntegerLiteral,
        201 => ASTEntryTag::TagFloatingLiteral,
        202 => ASTEntryTag::TagCharacterLiteral,
        203 => ASTEntryTag::TagStringLiteral,
        204 => ASTEntryTag::TagCXXBoolLiteralExpr,
        205 => ASTEntryTag::TagCXXNullPtrLiteralExpr,
        210 => ASTEntryTag::TagBinaryOperator,
        211 => ASTEntryTag::TagUnaryOperator,
        212 => ASTEntryTag::TagCompoundAssignOperator,
        213 => ASTEntryTag::TagConditionalOperator,
        220 => ASTEntryTag::TagDeclRefExpr,
        221 => ASTEntryTag::TagMemberExpr,
        222 => ASTEntryTag::TagCXXThisExpr,
        230 => ASTEntryTag::TagCallExpr,
        231 => ASTEntryTag::TagCXXMemberCallExpr,
        232 => ASTEntryTag::TagCXXOperatorCallExpr,
        233 => ASTEntryTag::TagCXXConstructExpr,
        234 => ASTEntryTag::TagCXXTemporaryObjectExpr,
        235 => ASTEntryTag::TagCXXNewExpr,
        236 => ASTEntryTag::TagCXXDeleteExpr,
        240 => ASTEntryTag::TagImplicitCastExpr,
        241 => ASTEntryTag::TagCStyleCastExpr,
        242 => ASTEntryTag::TagCXXStaticCastExpr,
        243 => ASTEntryTag::TagCXXDynamicCastExpr,
        244 => ASTEntryTag::TagCXXReinterpretCastExpr,
        245 => ASTEntryTag::TagCXXConstCastExpr,
        246 => ASTEntryTag::TagCXXFunctionalCastExpr,
        250 => ASTEntryTag::TagArraySubscriptExpr,
        251 => ASTEntryTag::TagInitListExpr,
        252 => ASTEntryTag::TagCXXStdInitializerListExpr,
        260 => ASTEntryTag::TagUnaryExprOrTypeTraitExpr,
        270 => ASTEntryTag::TagParenExpr,
        271 => ASTEntryTag::TagExprWithCleanups,
        272 => ASTEntryTag::TagMaterializeTemporaryExpr,
        273 => ASTEntryTag::TagCXXBindTemporaryExpr,
        280 => ASTEntryTag::TagCXXDefaultArgExpr,
        281 => ASTEntryTag::TagCXXDefaultInitExpr,
        290 => ASTEntryTag::TagLambdaExpr,
        295 => ASTEntryTag::TagTypeTraitExpr,
        296 => ASTEntryTag::TagImplicitValueInitExpr,
        297 => ASTEntryTag::TagCXXScalarValueInitExpr,
        400 => ASTEntryTag::TagTypeUnknown,
        500 => ASTEntryTag::TagVoid,
        501 => ASTEntryTag::TagBool,
        502 => ASTEntryTag::TagChar,
        503 => ASTEntryTag::TagSChar,
        504 => ASTEntryTag::TagUChar,
        505 => ASTEntryTag::TagWChar,
        506 => ASTEntryTag::TagChar16,
        507 => ASTEntryTag::TagChar32,
        508 => ASTEntryTag::TagShort,
        509 => ASTEntryTag::TagUShort,
        510 => ASTEntryTag::TagInt,
        511 => ASTEntryTag::TagUInt,
        512 => ASTEntryTag::TagLong,
        513 => ASTEntryTag::TagULong,
        514 => ASTEntryTag::TagLongLong,
        515 => ASTEntryTag::TagULongLong,
        516 => ASTEntryTag::TagInt128,
        517 => ASTEntryTag::TagUInt128,
        518 => ASTEntryTag::TagFloat,
        519 => ASTEntryTag::TagDouble,
        520 => ASTEntryTag::TagLongDouble,
        521 => ASTEntryTag::TagFloat128,
        530 => ASTEntryTag::TagPointerType,
        531 => ASTEntryTag::TagLValueReferenceType,
        532 => ASTEntryTag::TagRValueReferenceType,
        540 => ASTEntryTag::TagConstantArrayType,
        541 => ASTEntryTag::TagIncompleteArrayType,
        542 => ASTEntryTag::TagVariableArrayType,
        543 => ASTEntryTag::TagDependentSizedArrayType,
        550 => ASTEntryTag::TagFunctionProtoType,
        551 => ASTEntryTag::TagFunctionNoProtoType,
        560 => ASTEntryTag::TagRecordType,
        561 => ASTEntryTag::TagEnumType,
        570 => ASTEntryTag::TagTypedefType,
        571 => ASTEntryTag::TagElaboratedType,
        572 => ASTEntryTag::TagDecayedType,
        580 => ASTEntryTag::TagTemplateTypeParmType,
        581 => ASTEntryTag::TagSubstTemplateTypeParmType,
        582 => ASTEntryTag::TagTemplateSpecializationType,
        583 => ASTEntryTag::TagDependentNameType,
        584 => ASTEntryTag::TagAutoType,
        585 => ASTEntryTag::TagDecltypeType,
        590 => ASTEntryTag::TagAttributedType,
        591 => ASTEntryTag::TagParenType,
        _ => ASTEntryTag::TagTypeUnknown, // Unknown tag value
    }
}

/// Process CBOR data into an AstContext
pub fn process(items: Value) -> Result<AstContext, Error> {
    let mut ast_nodes: HashMap<u64, AstNode> = HashMap::new();
    let mut type_nodes: HashMap<u64, TypeNode> = HashMap::new();

    // The top-level value should be an array of all nodes
    let all_nodes = match items {
        Value::Array(arr) => arr,
        _ => return Err(Error::new(ErrorKind::InvalidData, "Expected array at top level")),
    };

    for entry in all_nodes {
        let entry_array = match entry {
            Value::Array(arr) => arr,
            _ => continue, // Skip malformed entries
        };

        if entry_array.len() < 2 {
            continue;
        }

        let entry_id = expect_u64(&entry_array[0])?;
        let tag_num = expect_u64(&entry_array[1])?;

        // Determine if this is an AST node or type node based on tag value
        // Type tags start at 400+
        if tag_num >= 400 {
            // Type node
            let tag = import_ast_tag(tag_num);
            let extras: Vec<Value> = entry_array.into_iter().skip(2).collect();

            type_nodes.insert(
                entry_id,
                TypeNode {
                    id: entry_id,
                    tag,
                    extras,
                },
            );
        } else {
            // AST node
            let tag = import_ast_tag(tag_num);

            // Parse children array
            let children: Vec<Option<u64>> = if entry_array.len() > 2 {
                match &entry_array[2] {
                    Value::Array(arr) => arr
                        .iter()
                        .map(|v| expect_opt_u64(v).unwrap_or(None))
                        .collect(),
                    _ => vec![],
                }
            } else {
                vec![]
            };

            // Parse source location (indices 3-7)
            let loc = if entry_array.len() > 7 {
                SrcSpan {
                    file_id: expect_u64(&entry_array[3]).unwrap_or(0),
                    begin_line: expect_u64(&entry_array[4]).unwrap_or(0),
                    begin_column: expect_u64(&entry_array[5]).unwrap_or(0),
                    end_line: expect_u64(&entry_array[6]).unwrap_or(0),
                    end_column: expect_u64(&entry_array[7]).unwrap_or(0),
                }
            } else {
                SrcSpan::default()
            };

            // Parse type ID (index 8)
            let type_id = if entry_array.len() > 8 {
                expect_opt_u64(&entry_array[8]).unwrap_or(None)
            } else {
                None
            };

            // Extra data (index 9+)
            let extras: Vec<Value> = entry_array.into_iter().skip(9).collect();

            ast_nodes.insert(
                entry_id,
                AstNode {
                    id: entry_id,
                    tag,
                    children,
                    loc,
                    type_id,
                    extras,
                },
            );
        }
    }

    // Collect top-level nodes (those not referenced as children)
    let all_children: std::collections::HashSet<u64> = ast_nodes
        .values()
        .flat_map(|n| n.children.iter().filter_map(|c| *c))
        .collect();

    let top_nodes: Vec<u64> = ast_nodes
        .keys()
        .filter(|id| !all_children.contains(id))
        .copied()
        .collect();

    Ok(AstContext {
        ast_nodes,
        type_nodes,
        top_nodes,
        files: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_qualifiers() {
        let base_id: u64 = 0x1234_5678_0000_0000;

        // Test const
        let const_id = base_id | TypeNode::CONST_MASK;
        assert!(TypeNode::is_const(const_id));
        assert!(!TypeNode::is_volatile(const_id));
        assert_eq!(TypeNode::unqualified_id(const_id), base_id);

        // Test volatile
        let volatile_id = base_id | TypeNode::VOLATILE_MASK;
        assert!(!TypeNode::is_const(volatile_id));
        assert!(TypeNode::is_volatile(volatile_id));

        // Test const volatile
        let cv_id = base_id | TypeNode::CONST_MASK | TypeNode::VOLATILE_MASK;
        assert!(TypeNode::is_const(cv_id));
        assert!(TypeNode::is_volatile(cv_id));
        assert_eq!(TypeNode::unqualified_id(cv_id), base_id);
    }
}
