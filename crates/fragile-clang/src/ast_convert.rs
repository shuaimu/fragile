//! AstContext-to-ClangNode conversion for LibTooling AST export.
//!
//! Converts `fragile_ast_exporter::clang_ast::AstContext` node trees into the
//! `ClangNode` / `ClangNodeKind` model used by the codegen pipeline.

use crate::ast::{
    AccessSpecifier, ClangNode, ClangNodeKind, ConstructorKind, SourceLocation,
    TemplateSpecializationKind,
};
use crate::types::CppType;
use fragile_ast_exporter::{
    clang_ast::{AstContext, AstNode},
    ASTEntryTag, CborValue,
};
use std::collections::{HashMap, HashSet};

pub fn convert_to_clang_node(ctx: &AstContext, root_id: u64) -> Option<ClangNode> {
    // Limit depth to avoid excessive processing of deeply nested STL internals
    const MAX_DEPTH: usize = 100;

    let mut cache: HashMap<u64, ClangNode> = HashMap::new();
    let mut active: HashSet<u64> = HashSet::new();
    convert_node_with_depth(ctx, root_id, 0, MAX_DEPTH, &mut cache, &mut active)
}

fn extract_string_array_extra(node: &AstNode, index: usize) -> Vec<String> {
    match node.extras.get(index) {
        Some(CborValue::Array(values)) => values
            .iter()
            .filter_map(|v| match v {
                CborValue::Text(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn extract_lambda_capture_default(node: &AstNode) -> crate::ast::CaptureDefault {
    match node.get_u64(0).unwrap_or(0) {
        1 => crate::ast::CaptureDefault::ByCopy,
        2 => crate::ast::CaptureDefault::ByRef,
        _ => crate::ast::CaptureDefault::None,
    }
}

fn extract_lambda_captures(node: &AstNode) -> Vec<(String, bool)> {
    let Some(CborValue::Array(captures)) = node.extras.get(1) else {
        return Vec::new();
    };

    captures
        .iter()
        .filter_map(|entry| {
            let CborValue::Array(parts) = entry else {
                return None;
            };
            // Entry layout from exporter: [capture_kind, is_implicit, var_name]
            let kind = parts
                .first()
                .and_then(|v| match v {
                    CborValue::Integer(i) => Some(*i as u64),
                    _ => None,
                })
                .unwrap_or(0);
            let name = parts
                .get(2)
                .and_then(|v| match v {
                    CborValue::Text(s) => Some(s),
                    _ => None,
                })?
                .trim();
            if name.is_empty() {
                return None;
            }
            // Clang LambdaCaptureKind::LCK_ByRef == 1.
            let by_ref = kind == 1;
            Some((name.to_string(), by_ref))
        })
        .collect()
}

fn extract_lambda_params(ctx: &AstContext, node: &AstNode) -> Vec<(String, CppType)> {
    let Some(CborValue::Array(params)) = node.extras.get(2) else {
        return Vec::new();
    };

    params
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| {
            let CborValue::Array(parts) = entry else {
                return None;
            };
            let raw_name = parts
                .first()
                .and_then(|v| match v {
                    CborValue::Text(s) => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("");
            let name = if raw_name.trim().is_empty() {
                format!("arg{idx}")
            } else {
                raw_name.trim().to_string()
            };

            let ty = parts
                .get(1)
                .and_then(|v| match v {
                    CborValue::Integer(i) => Some(*i as u64),
                    _ => None,
                })
                .and_then(|id| resolve_type(ctx, id))
                .unwrap_or_else(|| CppType::Named("UnknownTagAutoType".to_string()));

            Some((name, ty))
        })
        .collect()
}

fn extract_lambda_return_type(ctx: &AstContext, node: &AstNode) -> CppType {
    node.extras
        .get(3)
        .and_then(|v| match v {
            CborValue::Integer(i) => Some(*i as u64),
            _ => None,
        })
        .and_then(|id| resolve_type(ctx, id))
        .unwrap_or(CppType::Void)
}

fn function_decl_identity_key(node: &AstNode) -> Option<String> {
    if node.tag != ASTEntryTag::TagFunctionDecl {
        return None;
    }

    if let Some(canonical) = node.get_u64(9).filter(|id| *id != 0) {
        return Some(format!("canon:{canonical}"));
    }
    if let Some(mangled) = node.get_string(6).filter(|s| !s.is_empty()) {
        return Some(format!("mangled:{mangled}"));
    }

    let name = node.get_string(0).unwrap_or("");
    if name.is_empty() {
        return None;
    }
    Some(format!("name:{name}:type:{}", node.type_id.unwrap_or(0)))
}

fn function_decl_has_body(node: &AstNode, ctx: &AstContext) -> bool {
    node.children.iter().flatten().any(|child_id| {
        ctx.ast_nodes
            .get(child_id)
            .is_some_and(|child| child.tag == ASTEntryTag::TagCompoundStmt)
    })
}

fn dedup_function_decl_child_ids(ctx: &AstContext, parent: &AstNode) -> Vec<u64> {
    let mut deduped: Vec<u64> = Vec::new();
    let mut key_to_index: HashMap<String, usize> = HashMap::new();

    for child_id in parent.children.iter().filter_map(|child| *child) {
        let Some(child_node) = ctx.ast_nodes.get(&child_id) else {
            continue;
        };
        if child_node.tag != ASTEntryTag::TagFunctionDecl {
            deduped.push(child_id);
            continue;
        }

        let Some(key) = function_decl_identity_key(child_node) else {
            deduped.push(child_id);
            continue;
        };

        if let Some(existing_index) = key_to_index.get(&key).copied() {
            let Some(existing_node_id) = deduped.get(existing_index).copied() else {
                continue;
            };
            let Some(existing_node) = ctx.ast_nodes.get(&existing_node_id) else {
                deduped[existing_index] = child_id;
                continue;
            };

            let existing_has_body = function_decl_has_body(existing_node, ctx);
            let child_has_body = function_decl_has_body(child_node, ctx);
            if child_has_body && !existing_has_body {
                deduped[existing_index] = child_id;
            }
            continue;
        }

        key_to_index.insert(key, deduped.len());
        deduped.push(child_id);
    }

    deduped
}

fn extract_case_stmt_value(ctx: &AstContext, node: &AstNode) -> i128 {
    if let Some(value) = node.get_int(0) {
        return value as i128;
    }

    fn value_from_node(ctx: &AstContext, node: &AstNode, depth: usize) -> Option<i128> {
        if depth > 12 {
            return None;
        }

        match node.tag {
            ASTEntryTag::TagIntegerLiteral | ASTEntryTag::TagCharacterLiteral => {
                node.get_int(0).map(|v| v as i128)
            }
            ASTEntryTag::TagUnaryOperator => {
                let inner = node
                    .children
                    .iter()
                    .flatten()
                    .find_map(|child_id| ctx.ast_nodes.get(child_id))
                    .and_then(|child| value_from_node(ctx, child, depth + 1));
                let op = node.get_string(1).unwrap_or(node.get_string(0).unwrap_or(""));
                match op {
                    "-" => inner.map(|v| -v),
                    "+" => inner,
                    _ => inner,
                }
            }
            _ => node
                .children
                .iter()
                .flatten()
                .find_map(|child_id| ctx.ast_nodes.get(child_id))
                .and_then(|child| value_from_node(ctx, child, depth + 1)),
        }
    }

    value_from_node(ctx, node, 0).unwrap_or(0)
}

fn convert_node_with_depth(
    ctx: &AstContext,
    node_id: u64,
    current_depth: usize,
    max_depth: usize,
    cache: &mut HashMap<u64, ClangNode>,
    active: &mut HashSet<u64>,
) -> Option<ClangNode> {
    if current_depth > max_depth {
        // Return a placeholder for too-deep nodes
        return Some(ClangNode {
            kind: ClangNodeKind::Unknown("TooDeep".to_string()),
            children: vec![],
            location: SourceLocation {
                file: None,
                line: 0,
                column: 0,
            },
        });
    }

    if let Some(cached) = cache.get(&node_id) {
        return Some(cached.clone());
    }

    let node = ctx.ast_nodes.get(&node_id)?;
    if !active.insert(node_id) {
        return Some(ClangNode {
            kind: ClangNodeKind::Unknown("RecursiveRef".to_string()),
            children: vec![],
            location: SourceLocation {
                file: None,
                line: node.loc.begin_line as u32,
                column: node.loc.begin_column as u32,
            },
        });
    }

    let location = SourceLocation {
        file: None, // TODO: Extract from node.loc
        line: node.loc.begin_line as u32,
        column: node.loc.begin_column as u32,
    };

    let kind = match node.tag {
        ASTEntryTag::TagCompoundStmt => ClangNodeKind::CompoundStmt,

        ASTEntryTag::TagReturnStmt => {
            // ReturnStmt is a unit variant, its value is in children
            ClangNodeKind::ReturnStmt
        }

        ASTEntryTag::TagBinaryOperator => {
            // extras[0] = opcode uint, extras[1] = opcode string (from getOpcodeStr)
            let op_str = node.get_string(1).unwrap_or("+");
            let op = parse_binary_op(op_str);
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::BinaryOperator { op, ty }
        }

        ASTEntryTag::TagDeclRefExpr => {
            let name = node.get_string(0).unwrap_or("").to_string();
            let ty = extract_type_from_node(ctx, node);
            let namespace_path = extract_string_array_extra(node, 2);
            ClangNodeKind::DeclRefExpr {
                name,
                ty,
                namespace_path,
            }
        }

        ASTEntryTag::TagIntegerLiteral => {
            let value = node.get_int(0).unwrap_or(0) as i128;
            let cpp_type = Some(extract_type_from_node(ctx, node));
            ClangNodeKind::IntegerLiteral { value, cpp_type }
        }

        ASTEntryTag::TagCXXThisExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXThisExpr { ty }
        }

        ASTEntryTag::TagMemberExpr => {
            let member_name = node.get_string(0).unwrap_or("").to_string();
            let is_arrow = node.get_bool(1).unwrap_or(false);
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::MemberExpr {
                member_name,
                is_arrow,
                ty,
                is_static: false,
                declaring_class: None,
            }
        }

        ASTEntryTag::TagCallExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CallExpr {
                ty,
                template_instantiation: None,
            }
        }

        ASTEntryTag::TagDeclStmt => ClangNodeKind::DeclStmt,

        ASTEntryTag::TagVarDecl => {
            let name = node.get_string(0).unwrap_or("").to_string();
            let ty = extract_type_from_node(ctx, node);
            let has_init = node.children.iter().any(|c| c.is_some());
            let is_constexpr = node.get_bool(2).unwrap_or(false);
            let is_static = node.get_bool(4).unwrap_or(false);
            // Namespace-scope constexpr variables should not be exported as C globals.
            // Treat them as non-exportable declarations to avoid ODR conflicts for
            // header-defined variable templates (e.g., `template<typename T> constexpr bool ..._v`).
            let is_extern = node.get_bool(5).unwrap_or(false) || is_constexpr;
            ClangNodeKind::VarDecl {
                name,
                ty,
                has_init,
                is_static,
                is_extern,
            }
        }

        ASTEntryTag::TagUnaryOperator => {
            // extras[0] = opcode uint (UO_PostInc=0..UO_Coawait=13), extras[1] = isPrefix bool
            let op = if let Some(opcode) = node.get_int(0) {
                parse_unary_op_from_opcode(opcode as u32)
            } else {
                crate::ast::UnaryOp::Plus // fallback
            };
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::UnaryOperator { op, ty }
        }

        ASTEntryTag::TagImplicitCastExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::ImplicitCastExpr {
                cast_kind: crate::ast::CastKind::LValueToRValue,
                ty,
            }
        }

        ASTEntryTag::TagCXXMemberCallExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CallExpr {
                ty,
                template_instantiation: None,
            }
        }

        ASTEntryTag::TagIfStmt => ClangNodeKind::IfStmt,

        ASTEntryTag::TagForStmt => ClangNodeKind::ForStmt,

        ASTEntryTag::TagWhileStmt => ClangNodeKind::WhileStmt,

        ASTEntryTag::TagBreakStmt => ClangNodeKind::BreakStmt,

        ASTEntryTag::TagContinueStmt => ClangNodeKind::ContinueStmt,

        ASTEntryTag::TagParenExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::ParenExpr { ty }
        }

        ASTEntryTag::TagCXXBoolLiteralExpr => {
            // Bool value is typically in extras
            let value = node.get_bool(0).unwrap_or(false);
            ClangNodeKind::BoolLiteral(value)
        }

        ASTEntryTag::TagCXXNullPtrLiteralExpr => ClangNodeKind::NullPtrLiteral,

        ASTEntryTag::TagFloatingLiteral => {
            // Get float value - try different methods
            let value = if let Some(v) = node.get_int(0) {
                v as f64
            } else {
                0.0
            };
            let cpp_type = Some(extract_type_from_node(ctx, node));
            ClangNodeKind::FloatingLiteral { value, cpp_type }
        }

        ASTEntryTag::TagConditionalOperator => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::ConditionalOperator { ty }
        }

        ASTEntryTag::TagCStyleCastExpr
        | ASTEntryTag::TagCXXStaticCastExpr
        | ASTEntryTag::TagCXXReinterpretCastExpr
        | ASTEntryTag::TagCXXConstCastExpr
        | ASTEntryTag::TagCXXFunctionalCastExpr
        | ASTEntryTag::TagCXXDynamicCastExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CastExpr {
                cast_kind: crate::ast::CastKind::LValueToRValue,
                ty,
            }
        }

        ASTEntryTag::TagArraySubscriptExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::ArraySubscriptExpr { ty }
        }

        ASTEntryTag::TagCXXOperatorCallExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CallExpr {
                ty,
                template_instantiation: None,
            }
        }

        ASTEntryTag::TagCompoundAssignOperator => {
            // extras[0] = opcode uint, extras[1] = opcode string (from getOpcodeStr)
            let op_str = node.get_string(1).unwrap_or("+=");
            let op = parse_binary_op(op_str);
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::BinaryOperator { op, ty }
        }

        // Expression wrappers - pass through to children
        ASTEntryTag::TagExprWithCleanups
        | ASTEntryTag::TagMaterializeTemporaryExpr
        | ASTEntryTag::TagCXXBindTemporaryExpr => {
            // These are wrapper nodes, we just use Unknown but children will be processed
            ClangNodeKind::Unknown("ExprWrapper".to_string())
        }

        ASTEntryTag::TagCXXConstructExpr | ASTEntryTag::TagCXXTemporaryObjectExpr => {
            // Use dedicated CXXConstructExpr node to distinguish from function calls
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXConstructExpr { ty }
        }

        ASTEntryTag::TagInitListExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::InitListExpr { ty }
        }

        ASTEntryTag::TagCXXDefaultArgExpr | ASTEntryTag::TagCXXDefaultInitExpr => {
            ClangNodeKind::Unknown("DefaultExpr".to_string())
        }

        ASTEntryTag::TagCXXNewExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXNewExpr {
                ty,
                is_array: false,
                is_placement: false,
            }
        }

        ASTEntryTag::TagCXXDeleteExpr => ClangNodeKind::CXXDeleteExpr { is_array: false },

        ASTEntryTag::TagStringLiteral => {
            let value = node.get_string(0).unwrap_or("").to_string();
            ClangNodeKind::StringLiteral(value)
        }

        ASTEntryTag::TagCharacterLiteral => {
            // Character literals are treated as integer literals in the AST
            let value = node.get_int(0).unwrap_or(0) as i128;
            ClangNodeKind::IntegerLiteral {
                value,
                cpp_type: Some(CppType::Char { signed: true }),
            }
        }

        // Declaration nodes that appear within method bodies
        ASTEntryTag::TagFieldDecl => convert_field_decl_node(ctx, node),

        ASTEntryTag::TagCXXMethodDecl => {
            // Keep non-static methods on the existing libtooling body extraction path.
            // For static inline methods with local bodies (common utility/factory
            // helpers in headers), emit real method declarations so `Type::method()`
            // calls can resolve in strict object mode.
            let is_static = node.get_bool(1).unwrap_or(false);
            let has_body = node.children.iter().any(|child_id_opt| {
                child_id_opt
                    .and_then(|child_id| ctx.ast_nodes.get(&child_id))
                    .is_some_and(|child_node| child_node.tag == ASTEntryTag::TagCompoundStmt)
            });
            let name = node.get_string(0).unwrap_or("");
            let allow_member_surface =
                name == "connect" || name == "error_code_";
            if (is_static && has_body) || allow_member_surface {
                convert_cxx_method_decl_node(ctx, node)
            } else {
                ClangNodeKind::Unknown("InlineMethodDecl".to_string())
            }
        }

        ASTEntryTag::TagCXXConstructorDecl | ASTEntryTag::TagCXXDestructorDecl => {
            // Constructor/destructor declarations - skip
            ClangNodeKind::Unknown("InlineSpecialMemberDecl".to_string())
        }

        ASTEntryTag::TagParmVarDecl => {
            // Parameter declarations
            let name = node.get_string(0).unwrap_or("").to_string();
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::VarDecl {
                name,
                ty,
                has_init: false,
                is_static: false,
                is_extern: false,
            }
        }

        // Additional statement types
        ASTEntryTag::TagDoStmt => ClangNodeKind::DoStmt,

        ASTEntryTag::TagSwitchStmt => ClangNodeKind::SwitchStmt,

        ASTEntryTag::TagCaseStmt => {
            // Case statements have a value expression and body
            // Value can be in extras or nested under ConstantExpr/wrapper children.
            let value = extract_case_stmt_value(ctx, node);
            ClangNodeKind::CaseStmt {
                value,
                enum_name: None,
            }
        }

        ASTEntryTag::TagDefaultStmt => ClangNodeKind::DefaultStmt,

        ASTEntryTag::TagGotoStmt => {
            let label = node.get_string(0).unwrap_or("").to_string();
            ClangNodeKind::GotoStmt { label }
        }

        ASTEntryTag::TagLabelStmt => {
            let label = node.get_string(0).unwrap_or("").to_string();
            ClangNodeKind::LabelStmt { label }
        }

        ASTEntryTag::TagNullStmt => ClangNodeKind::NullStmt,

        ASTEntryTag::TagCXXForRangeStmt => {
            // Range-based for loop - extract var name and type
            let var_name = node.get_string(0).unwrap_or("item").to_string();
            let var_type = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXForRangeStmt { var_name, var_type }
        }

        ASTEntryTag::TagCXXTryStmt => ClangNodeKind::TryStmt,

        ASTEntryTag::TagCXXCatchStmt => {
            let exception_ty = Some(extract_type_from_node(ctx, node));
            ClangNodeKind::CatchStmt { exception_ty }
        }

        ASTEntryTag::TagCXXThrowExpr => {
            let exception_ty = Some(extract_type_from_node(ctx, node));
            ClangNodeKind::ThrowExpr { exception_ty }
        }

        // Additional expression types
        ASTEntryTag::TagUnaryExprOrTypeTraitExpr => {
            // sizeof, alignof, etc.
            let kind_str = node.get_string(0).unwrap_or("sizeof").to_string();
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::UnaryExprOrTypeTraitExpr {
                kind: kind_str,
                argument_type: Some(ty),
            }
        }

        ASTEntryTag::TagCXXStdInitializerListExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::InitListExpr { ty }
        }

        ASTEntryTag::TagLambdaExpr => ClangNodeKind::LambdaExpr {
            params: extract_lambda_params(ctx, node),
            return_type: extract_lambda_return_type(ctx, node),
            capture_default: extract_lambda_capture_default(node),
            captures: extract_lambda_captures(node),
        },

        ASTEntryTag::TagTypeTraitExpr => {
            // Type traits like std::is_same_v
            ClangNodeKind::BoolLiteral(false) // Placeholder
        }

        ASTEntryTag::TagImplicitValueInitExpr | ASTEntryTag::TagCXXScalarValueInitExpr => {
            // Value initialization - generates default/zero value
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXDefaultInitExpr { ty }
        }

        // Record/class related declarations that might appear in bodies
        ASTEntryTag::TagCXXRecordDecl => convert_record_decl_node(ctx, node),

        ASTEntryTag::TagClassTemplateSpecializationDecl => {
            convert_class_template_specialization_decl_node(ctx, node)
        }

        ASTEntryTag::TagTypedefDecl => convert_typedef_decl_node(ctx, node),

        ASTEntryTag::TagTypeAliasDecl => convert_type_alias_decl_node(ctx, node),

        ASTEntryTag::TagEnumDecl => convert_enum_decl_node(ctx, node),

        ASTEntryTag::TagEnumConstantDecl => convert_enum_constant_decl_node(node),

        ASTEntryTag::TagAccessSpecDecl => {
            // public/private/protected - skip
            ClangNodeKind::Unknown("AccessSpec".to_string())
        }

        ASTEntryTag::TagStaticAssertDecl => ClangNodeKind::Unknown("StaticAssert".to_string()),

        ASTEntryTag::TagFunctionDecl => convert_function_decl_node(ctx, node),

        ASTEntryTag::TagFunctionTemplateDecl => convert_function_template_decl_node(ctx, node),

        ASTEntryTag::TagClassTemplateDecl => convert_class_template_decl_node(node),

        ASTEntryTag::TagNamespaceDecl => convert_namespace_decl_node(node),

        ASTEntryTag::TagUsingDecl | ASTEntryTag::TagUsingDirectiveDecl => {
            ClangNodeKind::Unknown("NamespaceRelated".to_string())
        }

        ASTEntryTag::TagTemplateTypeParmDecl => convert_template_type_param_decl_node(node),

        ASTEntryTag::TagNonTypeTemplateParmDecl => {
            // Constrained fallback: keep non-type template params in the current
            // AST model by normalizing them into TemplateTypeParmDecl metadata.
            convert_template_param_decl_fallback_node(node, "__fragile_nttp")
        }

        ASTEntryTag::TagTemplateTemplateParmDecl => {
            // Constrained fallback: preserve template-template param position/pack
            // metadata without introducing a new node variant yet.
            convert_template_param_decl_fallback_node(node, "__fragile_ttpl")
        }

        _ => {
            // For unknown nodes, create an Unknown variant
            ClangNodeKind::Unknown(format!("{:?}", node.tag))
        }
    };

    // Convert children with depth tracking.
    // Namespace/linkage containers can carry repeated declarations for the same
    // canonical function symbol (redecls + reopened scopes). Dedup here so
    // codegen sees one declaration surface per symbol.
    let child_ids: Vec<u64> = match node.tag {
        ASTEntryTag::TagNamespaceDecl => dedup_function_decl_child_ids(ctx, node),
        _ => node
            .children
            .iter()
            .filter_map(|child_id| *child_id)
            .collect(),
    };
    let children: Vec<ClangNode> = child_ids
        .iter()
        .filter_map(|id| {
            convert_node_with_depth(ctx, *id, current_depth + 1, max_depth, cache, active)
        })
        .collect();

    let converted = ClangNode {
        kind,
        children,
        location,
    };
    active.remove(&node_id);
    cache.insert(node_id, converted.clone());
    Some(converted)
}

fn convert_function_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let raw_name = node.get_string(0).unwrap_or("").to_string();
    let name = if raw_name.is_empty() {
        "__fragile_libtooling_anon_fn".to_string()
    } else {
        raw_name
    };
    let is_static = node.get_bool(3).unwrap_or(false);
    let mangled_name = node
        .get_string(6)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| name.clone());

    let (return_type_opt, fn_param_types, is_variadic) = if let Some(type_id) = node.type_id {
        resolve_function_proto_type(ctx, type_id)
    } else {
        (None, Vec::new(), false)
    };

    let return_type = return_type_opt.unwrap_or(CppType::Int { signed: true });

    let mut params: Vec<(String, CppType)> = Vec::new();
    let mut param_index = 0usize;
    for child_id_opt in &node.children {
        let Some(child_id) = child_id_opt else {
            continue;
        };
        let Some(child_node) = ctx.ast_nodes.get(child_id) else {
            continue;
        };
        if child_node.tag != ASTEntryTag::TagParmVarDecl {
            continue;
        }

        let raw_param_name = child_node.get_string(0).unwrap_or("").to_string();
        let param_name = if raw_param_name.is_empty() {
            format!("arg{param_index}")
        } else {
            raw_param_name
        };
        let param_type = child_node
            .type_id
            .and_then(|type_id| resolve_type(ctx, type_id))
            .or_else(|| fn_param_types.get(param_index).cloned())
            .unwrap_or_else(|| CppType::Named("auto".to_string()));
        params.push((param_name, param_type));
        param_index += 1;
    }

    // If ParmVarDecl children are unavailable, preserve parameter arity/types
    // from FunctionProtoType to avoid dropping callable function surfaces.
    if params.is_empty() && !fn_param_types.is_empty() {
        for (idx, ty) in fn_param_types.iter().cloned().enumerate() {
            params.push((format!("arg{idx}"), ty));
        }
    }

    let is_definition = node.children.iter().any(|child_id_opt| {
        child_id_opt
            .and_then(|child_id| ctx.ast_nodes.get(&child_id))
            .is_some_and(|child_node| child_node.tag == ASTEntryTag::TagCompoundStmt)
    });

    let is_template_instantiation = node.get_bool(4).unwrap_or(false);
    let template_args = extract_function_template_instantiation_args(node);
    if (is_template_instantiation || !template_args.is_empty()) && !template_args.is_empty() {
        return ClangNodeKind::FunctionTemplateInstantiation {
            name: name.clone(),
            mangled_name: mangled_name.clone(),
            return_type,
            params,
            template_args,
            is_noexcept: false,
        };
    }

    ClangNodeKind::FunctionDecl {
        name: name.clone(),
        mangled_name,
        is_static,
        return_type,
        params,
        is_definition,
        is_variadic,
        is_noexcept: false,
        is_coroutine: false,
        coroutine_info: None,
    }
}

fn resolve_parent_record_name(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
    parent_extra_index: usize,
) -> String {
    if let Some(parent_id) = node.get_u64(parent_extra_index).filter(|id| *id != 0) {
        if let Some(parent) = ctx.ast_nodes.get(&parent_id) {
            if matches!(
                parent.tag,
                ASTEntryTag::TagCXXRecordDecl
                    | ASTEntryTag::TagClassTemplateDecl
                    | ASTEntryTag::TagClassTemplateSpecializationDecl
            ) {
                let parent_name = parent.get_string(0).unwrap_or("").to_string();
                if !parent_name.is_empty() {
                    return parent_name;
                }
            }
        }
    }

    // Fallback when parent pointer payload is unavailable: recover the direct
    // record child relationship from exported children links.
    for candidate in ctx.ast_nodes.values() {
        if !matches!(
            candidate.tag,
            ASTEntryTag::TagCXXRecordDecl
                | ASTEntryTag::TagClassTemplateDecl
                | ASTEntryTag::TagClassTemplateSpecializationDecl
        ) {
            continue;
        }
        if !candidate
            .children
            .iter()
            .any(|child| child.is_some_and(|id| id == node.id))
        {
            continue;
        }
        let parent_name = candidate.get_string(0).unwrap_or("").to_string();
        if !parent_name.is_empty() {
            return parent_name;
        }
    }

    String::new()
}

fn convert_cxx_method_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let name = node.get_string(0).unwrap_or("").to_string();
    if name.is_empty() {
        return ClangNodeKind::Unknown("InlineMethodDecl".to_string());
    }

    let class_name = resolve_parent_record_name(ctx, node, 7);
    let is_static = node.get_bool(1).unwrap_or(false);
    let is_const = node.get_bool(2).unwrap_or(false);
    let is_virtual = node.get_bool(3).unwrap_or(false);
    let is_pure_virtual = node.get_bool(4).unwrap_or(false);
    let access = decode_access_specifier(node.get_int(5));

    let (return_type_opt, fn_param_types, _is_variadic) = if let Some(type_id) = node.type_id {
        resolve_function_proto_type(ctx, type_id)
    } else {
        (None, Vec::new(), false)
    };
    let return_type = return_type_opt.unwrap_or(CppType::Void);

    let mut params: Vec<(String, CppType)> = Vec::new();
    let mut param_index = 0usize;
    for child_id_opt in &node.children {
        let Some(child_id) = child_id_opt else {
            continue;
        };
        let Some(child_node) = ctx.ast_nodes.get(child_id) else {
            continue;
        };
        if child_node.tag != ASTEntryTag::TagParmVarDecl {
            continue;
        }

        let raw_param_name = child_node.get_string(0).unwrap_or("").to_string();
        let param_name = if raw_param_name.is_empty() {
            format!("arg{param_index}")
        } else {
            raw_param_name
        };
        let param_type = child_node
            .type_id
            .and_then(|type_id| resolve_type(ctx, type_id))
            .or_else(|| fn_param_types.get(param_index).cloned())
            .unwrap_or_else(|| CppType::Named("auto".to_string()));
        params.push((param_name, param_type));
        param_index += 1;
    }
    if params.is_empty() && !fn_param_types.is_empty() {
        for (idx, ty) in fn_param_types.iter().cloned().enumerate() {
            params.push((format!("arg{idx}"), ty));
        }
    }

    let is_definition = node.children.iter().any(|child_id_opt| {
        child_id_opt
            .and_then(|child_id| ctx.ast_nodes.get(&child_id))
            .is_some_and(|child_node| child_node.tag == ASTEntryTag::TagCompoundStmt)
    });

    ClangNodeKind::CXXMethodDecl {
        class_name,
        name,
        return_type,
        params,
        is_definition,
        is_static,
        is_virtual,
        is_pure_virtual,
        is_override: false,
        is_final: false,
        is_const,
        access,
    }
}

fn convert_cxx_constructor_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let class_name = resolve_parent_record_name(ctx, node, 6);
    let is_default_ctor = node.get_bool(0).unwrap_or(false);
    let is_copy_ctor = node.get_bool(1).unwrap_or(false);
    let is_move_ctor = node.get_bool(2).unwrap_or(false);
    let access = decode_access_specifier(node.get_int(4));

    let ctor_kind = if is_copy_ctor {
        ConstructorKind::Copy
    } else if is_move_ctor {
        ConstructorKind::Move
    } else if is_default_ctor {
        ConstructorKind::Default
    } else {
        ConstructorKind::Other
    };

    let (_return_type_opt, fn_param_types, _is_variadic) = if let Some(type_id) = node.type_id {
        resolve_function_proto_type(ctx, type_id)
    } else {
        (None, Vec::new(), false)
    };

    let mut params: Vec<(String, CppType)> = Vec::new();
    let mut param_index = 0usize;
    for child_id_opt in &node.children {
        let Some(child_id) = child_id_opt else {
            continue;
        };
        let Some(child_node) = ctx.ast_nodes.get(child_id) else {
            continue;
        };
        if child_node.tag != ASTEntryTag::TagParmVarDecl {
            continue;
        }

        let raw_param_name = child_node.get_string(0).unwrap_or("").to_string();
        let param_name = if raw_param_name.is_empty() {
            format!("arg{param_index}")
        } else {
            raw_param_name
        };
        let param_type = child_node
            .type_id
            .and_then(|type_id| resolve_type(ctx, type_id))
            .or_else(|| fn_param_types.get(param_index).cloned())
            .unwrap_or_else(|| CppType::Named("auto".to_string()));
        params.push((param_name, param_type));
        param_index += 1;
    }
    if params.is_empty() && !fn_param_types.is_empty() {
        for (idx, ty) in fn_param_types.iter().cloned().enumerate() {
            params.push((format!("arg{idx}"), ty));
        }
    }

    let is_definition = node.children.iter().any(|child_id_opt| {
        child_id_opt
            .and_then(|child_id| ctx.ast_nodes.get(&child_id))
            .is_some_and(|child_node| child_node.tag == ASTEntryTag::TagCompoundStmt)
    });

    ClangNodeKind::ConstructorDecl {
        class_name,
        params,
        is_definition,
        ctor_kind,
        access,
    }
}

fn convert_cxx_destructor_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let class_name = resolve_parent_record_name(ctx, node, 3);
    let access = decode_access_specifier(node.get_int(1));
    let is_definition = node.children.iter().any(|child_id_opt| {
        child_id_opt
            .and_then(|child_id| ctx.ast_nodes.get(&child_id))
            .is_some_and(|child_node| child_node.tag == ASTEntryTag::TagCompoundStmt)
    });
    ClangNodeKind::DestructorDecl {
        class_name,
        is_definition,
        access,
    }
}

fn extract_function_template_instantiation_args(
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> Vec<CppType> {
    use fragile_ast_exporter::CborValue;

    let mut template_args = Vec::new();
    if let Some(CborValue::Array(args)) = node.extras.get(5) {
        for arg in args {
            if let CborValue::Text(text) = arg {
                let normalized = text.trim();
                if !normalized.is_empty() {
                    template_args.push(convert_template_arg_text_to_cpp_type(normalized));
                }
            }
        }
    }

    template_args
}

fn convert_template_arg_text_to_cpp_type(arg: &str) -> CppType {
    match arg {
        "void" => CppType::Void,
        "bool" => CppType::Bool,
        "char" => CppType::Char { signed: true },
        "signed char" => CppType::Char { signed: true },
        "unsigned char" => CppType::Char { signed: false },
        "short" => CppType::Short { signed: true },
        "unsigned short" => CppType::Short { signed: false },
        "int" => CppType::Int { signed: true },
        "unsigned int" => CppType::Int { signed: false },
        "long" | "long int" => CppType::Long { signed: true },
        "unsigned long" | "unsigned long int" => CppType::Long { signed: false },
        "long long" | "long long int" => CppType::LongLong { signed: true },
        "unsigned long long" | "unsigned long long int" => CppType::LongLong { signed: false },
        "float" => CppType::Float,
        "double" | "long double" => CppType::Double,
        _ => CppType::Named(arg.to_string()),
    }
}

fn convert_function_template_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    use fragile_ast_exporter::CborValue;

    let raw_name = node.get_string(0).unwrap_or("").to_string();
    let name = if raw_name.is_empty() {
        "__fragile_libtooling_anon_fn_template".to_string()
    } else {
        raw_name
    };

    let mut template_params = Vec::new();
    if let Some(CborValue::Array(params)) = node.extras.get(1) {
        for entry in params {
            if let CborValue::Text(param) = entry {
                if !param.is_empty() {
                    template_params.push(param.clone());
                }
            }
        }
    }

    let (return_type_opt, fn_param_types, _is_variadic) = if let Some(type_id) = node.type_id {
        resolve_function_proto_type(ctx, type_id)
    } else {
        (None, Vec::new(), false)
    };
    let return_type = return_type_opt.unwrap_or(CppType::Int { signed: true });

    let mut parameter_pack_indices = Vec::new();
    let mut template_param_index = 0usize;
    let mut params: Vec<(String, CppType)> = Vec::new();
    let mut param_index = 0usize;

    for child_id_opt in &node.children {
        let Some(child_id) = child_id_opt else {
            continue;
        };
        let Some(child_node) = ctx.ast_nodes.get(child_id) else {
            continue;
        };

        match child_node.tag {
            ASTEntryTag::TagTemplateTypeParmDecl
            | ASTEntryTag::TagNonTypeTemplateParmDecl
            | ASTEntryTag::TagTemplateTemplateParmDecl => {
                if child_node.get_bool(3).unwrap_or(false) {
                    parameter_pack_indices.push(template_param_index);
                }
                if template_params.len() <= template_param_index {
                    let raw_param_name = child_node.get_string(0).unwrap_or("").to_string();
                    if raw_param_name.is_empty() {
                        template_params.push(format!("T{template_param_index}"));
                    } else {
                        template_params.push(raw_param_name);
                    }
                }
                template_param_index += 1;
            }
            ASTEntryTag::TagParmVarDecl => {
                let raw_param_name = child_node.get_string(0).unwrap_or("").to_string();
                let param_name = if raw_param_name.is_empty() {
                    format!("arg{param_index}")
                } else {
                    raw_param_name
                };
                let param_type = child_node
                    .type_id
                    .and_then(|type_id| resolve_type(ctx, type_id))
                    .or_else(|| fn_param_types.get(param_index).cloned())
                    .unwrap_or_else(|| CppType::Named("auto".to_string()));
                params.push((param_name, param_type));
                param_index += 1;
            }
            _ => {}
        }
    }

    // Preserve parameter arity/types when ParmVarDecl children are unavailable.
    if params.is_empty() && !fn_param_types.is_empty() {
        for (idx, ty) in fn_param_types.iter().cloned().enumerate() {
            params.push((format!("arg{idx}"), ty));
        }
    }

    let is_definition = node.children.iter().any(|child_id_opt| {
        child_id_opt
            .and_then(|child_id| ctx.ast_nodes.get(&child_id))
            .is_some_and(|child_node| child_node.tag == ASTEntryTag::TagCompoundStmt)
    });

    ClangNodeKind::FunctionTemplateDecl {
        name,
        template_params,
        return_type,
        params,
        is_definition,
        parameter_pack_indices,
        requires_clause: None,
        is_noexcept: node.get_bool(2).unwrap_or(false),
    }
}

fn convert_record_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let name = node.get_string(0).unwrap_or("").to_string();
    if name.is_empty() {
        return ClangNodeKind::Unknown("InlineClassDecl".to_string());
    }

    let is_struct = node.get_bool(1).unwrap_or(false);
    let is_class = node.get_bool(2).unwrap_or(false);
    let is_union = node.get_bool(3).unwrap_or(false);
    let is_definition = node.get_bool(4).unwrap_or(false);

    if is_union {
        let mut fields = Vec::new();
        for child_id_opt in &node.children {
            let Some(child_id) = child_id_opt else {
                continue;
            };
            let Some(child_node) = ctx.ast_nodes.get(child_id) else {
                continue;
            };
            if child_node.tag != ASTEntryTag::TagFieldDecl {
                continue;
            }
            let field_name = child_node.get_string(0).unwrap_or("").to_string();
            let field_ty = child_node
                .type_id
                .and_then(|type_id| resolve_type(ctx, type_id))
                .unwrap_or_else(|| extract_type_from_node(ctx, child_node));
            fields.push((field_name, field_ty));
        }

        return ClangNodeKind::UnionDecl { name, fields };
    }

    let mut fields = Vec::new();
    for child_id_opt in &node.children {
        let Some(child_id) = child_id_opt else {
            continue;
        };
        let Some(child_node) = ctx.ast_nodes.get(child_id) else {
            continue;
        };
        if child_node.tag != ASTEntryTag::TagFieldDecl {
            continue;
        }
        let field_name = child_node.get_string(0).unwrap_or("").to_string();
        let field_ty = child_node
            .type_id
            .and_then(|type_id| resolve_type(ctx, type_id))
            .unwrap_or_else(|| extract_type_from_node(ctx, child_node));
        fields.push((field_name, field_ty));
    }

    ClangNodeKind::RecordDecl {
        name,
        is_class: if is_struct { false } else { is_class },
        is_definition,
        fields,
    }
}

fn convert_field_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let name = node.get_string(0).unwrap_or("").to_string();
    let ty = node
        .type_id
        .and_then(|type_id| resolve_type(ctx, type_id))
        .unwrap_or_else(|| extract_type_from_node(ctx, node));
    let access = decode_access_specifier(node.get_int(2));
    ClangNodeKind::FieldDecl {
        name,
        ty,
        access,
        is_static: false,
        bit_field_width: None,
    }
}

fn convert_class_template_decl_node(
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    use fragile_ast_exporter::CborValue;

    let name = node.get_string(0).unwrap_or("").to_string();
    if name.is_empty() {
        return ClangNodeKind::Unknown("InlineClassTemplateDecl".to_string());
    }

    let mut template_params = Vec::new();
    if let Some(CborValue::Array(params)) = node.extras.get(1) {
        for entry in params {
            if let CborValue::Text(param) = entry {
                if !param.is_empty() {
                    template_params.push(param.clone());
                }
            }
        }
    }

    let is_class = node.get_bool(2).unwrap_or(false);
    ClangNodeKind::ClassTemplateDecl {
        name,
        template_params,
        is_class,
        parameter_pack_indices: Vec::new(),
        requires_clause: None,
    }
}

fn decode_template_specialization_kind(
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> TemplateSpecializationKind {
    if let Some(kind_value) = node.get_u64(5) {
        return match kind_value {
            0 => TemplateSpecializationKind::Undeclared,
            1 => TemplateSpecializationKind::ImplicitInstantiation,
            2 => TemplateSpecializationKind::ExplicitSpecialization,
            3 => TemplateSpecializationKind::ExplicitInstantiationDeclaration,
            4 => TemplateSpecializationKind::ExplicitInstantiationDefinition,
            _ => TemplateSpecializationKind::Undeclared,
        };
    }

    // Backward compatibility for exporter payloads that only included two booleans.
    let is_implicit_instantiation = node.get_bool(3).unwrap_or(false);
    let is_explicit_specialization = node.get_bool(4).unwrap_or(false);
    if is_explicit_specialization {
        TemplateSpecializationKind::ExplicitSpecialization
    } else if is_implicit_instantiation {
        TemplateSpecializationKind::ImplicitInstantiation
    } else {
        TemplateSpecializationKind::Undeclared
    }
}

fn convert_class_template_specialization_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    use fragile_ast_exporter::CborValue;

    let raw_name = node.get_string(1).unwrap_or("").to_string();
    let mut name = if raw_name.is_empty() {
        node.get_string(0).unwrap_or("").to_string()
    } else {
        raw_name
    };
    if !name.contains('<') {
        let mut template_args = Vec::new();
        if let Some(CborValue::Array(args)) = node.extras.get(2) {
            for arg in args {
                if let CborValue::Array(pair) = arg {
                    if let Some(CborValue::Text(arg_str)) = pair.get(1) {
                        let normalized = arg_str.trim();
                        if !normalized.is_empty() {
                            template_args.push(normalized.to_string());
                        }
                    }
                }
            }
        }
        if !template_args.is_empty() {
            name = format!("{}<{}>", name, template_args.join(", "));
        }
    }

    if name.is_empty() {
        return ClangNodeKind::Unknown("InlineClassTemplateSpecializationDecl".to_string());
    }

    let mut fields = Vec::new();
    let mut has_member_children = false;
    for child_id_opt in &node.children {
        let Some(child_id) = child_id_opt else {
            continue;
        };
        let Some(child_node) = ctx.ast_nodes.get(child_id) else {
            continue;
        };
        match child_node.tag {
            ASTEntryTag::TagFieldDecl => {
                let field_name = child_node.get_string(0).unwrap_or("").to_string();
                let field_ty = child_node
                    .type_id
                    .and_then(|type_id| resolve_type(ctx, type_id))
                    .unwrap_or_else(|| extract_type_from_node(ctx, child_node));
                fields.push((field_name, field_ty));
                has_member_children = true;
            }
            ASTEntryTag::TagCXXMethodDecl
            | ASTEntryTag::TagCXXConstructorDecl
            | ASTEntryTag::TagCXXDestructorDecl => {
                has_member_children = true;
            }
            _ => {}
        }
    }

    let specialization_kind = decode_template_specialization_kind(node);

    ClangNodeKind::RecordDecl {
        name,
        // Constrained fallback until class/struct identity is exported directly
        // for template specializations.
        is_class: false,
        is_definition: match specialization_kind {
            // `extern template` declarations intentionally avoid emitting concrete bodies.
            TemplateSpecializationKind::ExplicitInstantiationDeclaration => false,
            // These forms own the concrete specialization.
            TemplateSpecializationKind::ExplicitSpecialization
            | TemplateSpecializationKind::ExplicitInstantiationDefinition => true,
            TemplateSpecializationKind::Undeclared
            | TemplateSpecializationKind::ImplicitInstantiation => has_member_children,
        },
        fields,
    }
}

fn convert_template_type_param_decl_node(
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    convert_template_param_decl_fallback_node(node, "__fragile_tparam")
}

fn convert_template_param_decl_fallback_node(
    node: &fragile_ast_exporter::clang_ast::AstNode,
    unnamed_prefix: &str,
) -> ClangNodeKind {
    let depth = node.get_u64(1).unwrap_or(0) as u32;
    let index = node.get_u64(2).unwrap_or(0) as u32;
    let raw_name = node.get_string(0).unwrap_or("").to_string();
    let name = if raw_name.is_empty() {
        format!("{unnamed_prefix}_{depth}_{index}")
    } else {
        raw_name
    };

    ClangNodeKind::TemplateTypeParmDecl {
        name,
        depth,
        index,
        is_pack: node.get_bool(3).unwrap_or(false),
    }
}

fn decode_access_specifier(raw: Option<i64>) -> AccessSpecifier {
    match raw {
        // Accept both clang::AccessSpecifier and CX_CXXAccessSpecifier-like
        // numeric encodings; default permissive for record field emission.
        Some(0) | Some(1) => AccessSpecifier::Public,
        Some(2) => AccessSpecifier::Protected,
        Some(3) => AccessSpecifier::Private,
        _ => AccessSpecifier::Public,
    }
}

fn convert_typedef_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let name = node.get_string(0).unwrap_or("").to_string();
    if name.is_empty() {
        return ClangNodeKind::Unknown("InlineTypedef".to_string());
    }

    let underlying_type = node
        .type_id
        .and_then(|type_id| resolve_type(ctx, type_id))
        .unwrap_or_else(|| extract_type_from_node(ctx, node));

    ClangNodeKind::TypedefDecl {
        name,
        underlying_type,
    }
}

fn convert_type_alias_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let name = node.get_string(0).unwrap_or("").to_string();
    if name.is_empty() {
        return ClangNodeKind::Unknown("InlineTypedef".to_string());
    }

    let underlying_type = node
        .type_id
        .and_then(|type_id| resolve_type(ctx, type_id))
        .unwrap_or_else(|| extract_type_from_node(ctx, node));

    ClangNodeKind::TypeAliasDecl {
        name,
        underlying_type,
    }
}

fn convert_enum_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let mut name = node.get_string(0).unwrap_or("").to_string();
    if name.is_empty() {
        // Preserve anonymous enums as explicit enum nodes so downstream codegen
        // can still surface their constants.
        name = format!(
            "(unnamed enum at file_{}:{}:{})",
            node.loc.file_id, node.loc.begin_line, node.loc.begin_column
        );
    }

    let is_scoped = node.get_bool(1).unwrap_or(false);
    let underlying_type = node
        .type_id
        .and_then(|type_id| resolve_type(ctx, type_id))
        .unwrap_or(CppType::Int { signed: true });

    ClangNodeKind::EnumDecl {
        name,
        is_scoped,
        underlying_type,
    }
}

fn convert_enum_constant_decl_node(
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let name = node.get_string(0).unwrap_or("").to_string();
    if name.is_empty() {
        return ClangNodeKind::Unknown("InlineEnumConstantDecl".to_string());
    }
    let value = node.get_int(1).map(|v| v as i64);
    ClangNodeKind::EnumConstantDecl { name, value }
}

fn convert_namespace_decl_node(node: &fragile_ast_exporter::clang_ast::AstNode) -> ClangNodeKind {
    let raw_name = node.get_string(0).unwrap_or("").to_string();
    let is_inline = node.get_bool(1).unwrap_or(false);
    let is_anonymous = node.get_bool(2).unwrap_or(false);
    let name = if is_anonymous || raw_name.is_empty() {
        None
    } else {
        Some(raw_name)
    };
    ClangNodeKind::NamespaceDecl { name, is_inline }
}

fn extract_type_from_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> CppType {
    if let Some(type_id) = node.type_id {
        // Prefer the full resolver so wrappers/typedefs/template substitutions
        // don't degrade to `auto` placeholders in downstream codegen.
        if let Some(resolved) = resolve_type(ctx, type_id) {
            return resolved;
        }

        let unqualified_type_id =
            fragile_ast_exporter::clang_ast::TypeNode::unqualified_id(type_id);
        if let Some(type_node) = ctx.type_nodes.get(&unqualified_type_id) {
            // Convert type node tag to a proper CppType
            use fragile_ast_exporter::ASTEntryTag;
            match type_node.tag {
                ASTEntryTag::TagVoid => return CppType::Void,
                ASTEntryTag::TagBool => return CppType::Bool,
                ASTEntryTag::TagInt => return CppType::Int { signed: true },
                ASTEntryTag::TagUInt => return CppType::Int { signed: false },
                ASTEntryTag::TagLong => return CppType::Long { signed: true },
                ASTEntryTag::TagULong => return CppType::Long { signed: false },
                ASTEntryTag::TagLongLong => return CppType::LongLong { signed: true },
                ASTEntryTag::TagULongLong => return CppType::LongLong { signed: false },
                ASTEntryTag::TagShort => return CppType::Short { signed: true },
                ASTEntryTag::TagUShort => return CppType::Short { signed: false },
                ASTEntryTag::TagChar | ASTEntryTag::TagSChar => {
                    return CppType::Char { signed: true }
                }
                ASTEntryTag::TagUChar => return CppType::Char { signed: false },
                ASTEntryTag::TagFloat => return CppType::Float,
                ASTEntryTag::TagDouble => return CppType::Double,
                ASTEntryTag::TagLongDouble => return CppType::Double, // Use double as fallback
                ASTEntryTag::TagPointerType => {
                    // For pointer types, try to get the pointee type
                    // For now, just return a void pointer
                    return CppType::Pointer {
                        pointee: Box::new(CppType::Void),
                        is_const: false,
                    };
                }
                ASTEntryTag::TagLValueReferenceType => {
                    return CppType::Reference {
                        referent: Box::new(CppType::Int { signed: true }),
                        is_const: false,
                        is_rvalue: false,
                    };
                }
                ASTEntryTag::TagRValueReferenceType => {
                    return CppType::Reference {
                        referent: Box::new(CppType::Int { signed: true }),
                        is_const: false,
                        is_rvalue: true,
                    };
                }
                // For record/class types, use the type name if available
                ASTEntryTag::TagRecordType
                | ASTEntryTag::TagElaboratedType
                | ASTEntryTag::TagTemplateSpecializationType => {
                    // Try to get a name from the type node
                    if let Some(name) = type_node.get_string(0) {
                        if !name.is_empty() {
                            return CppType::Named(name.to_string());
                        }
                    }
                    return CppType::Named("auto".to_string());
                }
                _ => {
                    // For other types, use 'auto' as a placeholder
                    return CppType::Named("auto".to_string());
                }
            }
        }
    }
    // Default to int for unknown types
    CppType::Int { signed: true }
}

fn parse_binary_op(op: &str) -> crate::ast::BinaryOp {
    use crate::ast::BinaryOp;
    match op {
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Sub,
        "*" => BinaryOp::Mul,
        "/" => BinaryOp::Div,
        "%" => BinaryOp::Rem,
        "==" => BinaryOp::Eq,
        "!=" => BinaryOp::Ne,
        "<" => BinaryOp::Lt,
        "<=" => BinaryOp::Le,
        ">" => BinaryOp::Gt,
        ">=" => BinaryOp::Ge,
        "&&" => BinaryOp::LAnd,
        "||" => BinaryOp::LOr,
        "&" => BinaryOp::And, // Bitwise AND
        "|" => BinaryOp::Or,  // Bitwise OR
        "^" => BinaryOp::Xor, // Bitwise XOR
        "<<" => BinaryOp::Shl,
        ">>" => BinaryOp::Shr,
        "=" => BinaryOp::Assign,
        "+=" => BinaryOp::AddAssign,
        "-=" => BinaryOp::SubAssign,
        "*=" => BinaryOp::MulAssign,
        "/=" => BinaryOp::DivAssign,
        "%=" => BinaryOp::RemAssign,
        "&=" => BinaryOp::AndAssign,
        "|=" => BinaryOp::OrAssign,
        "^=" => BinaryOp::XorAssign,
        "<<=" => BinaryOp::ShlAssign,
        ">>=" => BinaryOp::ShrAssign,
        "," => BinaryOp::Comma,
        _ => BinaryOp::Add, // Default fallback
    }
}

fn parse_unary_op_from_opcode(opcode: u32) -> crate::ast::UnaryOp {
    use crate::ast::UnaryOp;
    // Matches Clang's UnaryOperatorKind enum (ast_tags.hpp)
    match opcode {
        0 => UnaryOp::PostInc, // UO_PostInc
        1 => UnaryOp::PostDec, // UO_PostDec
        2 => UnaryOp::PreInc,  // UO_PreInc
        3 => UnaryOp::PreDec,  // UO_PreDec
        4 => UnaryOp::AddrOf,  // UO_AddrOf
        5 => UnaryOp::Deref,   // UO_Deref
        6 => UnaryOp::Plus,    // UO_Plus
        7 => UnaryOp::Minus,   // UO_Minus
        8 => UnaryOp::Not,     // UO_Not (~)
        9 => UnaryOp::LNot,    // UO_LNot (!)
        _ => UnaryOp::Plus,    // fallback
    }
}
fn resolve_function_proto_type(
    ctx: &AstContext,
    type_id: u64,
) -> (Option<CppType>, Vec<CppType>, bool) {
    use fragile_ast_exporter::clang_ast::TypeNode;
    use fragile_ast_exporter::CborValue;

    let type_node = match ctx.get_type(TypeNode::unqualified_id(type_id)) {
        Some(t) => t,
        None => return (None, Vec::new(), false),
    };

    if type_node.tag != ASTEntryTag::TagFunctionProtoType {
        return (None, Vec::new(), false);
    }

    // extras[0] = return type ID
    let return_type = match type_node.extras.first() {
        Some(CborValue::Integer(ret_id)) => {
            let ret_id = *ret_id as u64;
            resolve_type(ctx, ret_id)
        }
        _ => None,
    };

    // extras[1] = array of parameter type IDs
    let mut param_types = Vec::new();
    if let Some(CborValue::Array(param_ids)) = type_node.extras.get(1) {
        for param_id in param_ids {
            if let CborValue::Integer(id) = param_id {
                let id = *id as u64;
                if let Some(ptype) = resolve_type(ctx, id) {
                    param_types.push(ptype);
                }
            }
        }
    }

    // extras[2] = isVariadic
    let is_variadic = matches!(type_node.extras.get(2), Some(CborValue::Bool(true)));

    (return_type, param_types, is_variadic)
}

/// Format a CppType as a C++ type string for use in template arguments.
fn format_cpp_type(ty: &CppType) -> String {
    match ty {
        CppType::Void => "void".to_string(),
        CppType::Bool => "bool".to_string(),
        CppType::Char { signed: true } => "char".to_string(),
        CppType::Char { signed: false } => "unsigned char".to_string(),
        CppType::Short { signed: true } => "short".to_string(),
        CppType::Short { signed: false } => "unsigned short".to_string(),
        CppType::Int { signed: true } => "int".to_string(),
        CppType::Int { signed: false } => "unsigned int".to_string(),
        CppType::Long { signed: true } => "long".to_string(),
        CppType::Long { signed: false } => "unsigned long".to_string(),
        CppType::LongLong { signed: true } => "long long".to_string(),
        CppType::LongLong { signed: false } => "unsigned long long".to_string(),
        CppType::Float => "float".to_string(),
        CppType::Double => "double".to_string(),
        CppType::Named(name) => name.clone(),
        CppType::Pointer { pointee, is_const } => {
            if *is_const {
                format!("const {} *", format_cpp_type(pointee))
            } else {
                format!("{} *", format_cpp_type(pointee))
            }
        }
        CppType::Reference {
            referent,
            is_const,
            is_rvalue,
        } => {
            let ref_sym = if *is_rvalue { "&&" } else { "&" };
            if *is_const {
                format!("const {}{}", format_cpp_type(referent), ref_sym)
            } else {
                format!("{}{}", format_cpp_type(referent), ref_sym)
            }
        }
        CppType::Array { element, size } => {
            if let Some(n) = size {
                format!("{}[{}]", format_cpp_type(element), n)
            } else {
                format!("{}[]", format_cpp_type(element))
            }
        }
        CppType::Function {
            return_type,
            params,
            ..
        } => {
            let param_strs: Vec<String> = params.iter().map(format_cpp_type).collect();
            format!(
                "{}({})",
                format_cpp_type(return_type),
                param_strs.join(", ")
            )
        }
        CppType::TemplateParam { name, .. } => name.clone(),
        CppType::DependentType { spelling } => spelling.clone(),
        CppType::ParameterPack { name, .. } => format!("{}...", name),
    }
}
fn resolve_type(ctx: &AstContext, type_id: u64) -> Option<CppType> {
    use fragile_ast_exporter::CborValue;

    let type_node = ctx.get_type(type_id)?;

    match type_node.tag {
        // SubstTemplateTypeParmType contains a reference to the replacement type
        ASTEntryTag::TagSubstTemplateTypeParmType => {
            if let Some(CborValue::Integer(replacement_id)) = type_node.extras.first() {
                let replacement_id = *replacement_id as u64;
                resolve_type(ctx, replacement_id)
            } else {
                None
            }
        }

        // ElaboratedType wraps another type (e.g., "typename Foo::bar")
        ASTEntryTag::TagElaboratedType => {
            if let Some(CborValue::Integer(inner_id)) = type_node.extras.first() {
                let inner_id = *inner_id as u64;
                resolve_type(ctx, inner_id)
            } else {
                None
            }
        }

        // DecltypeType - follow to the underlying type
        // extras[0] = underlying type ID
        ASTEntryTag::TagDecltypeType => {
            if let Some(CborValue::Integer(underlying_id)) = type_node.extras.first() {
                let underlying_id = *underlying_id as u64;
                resolve_type(ctx, underlying_id)
            } else {
                None
            }
        }

        // Wrapper types that forward to an inner type ID.
        ASTEntryTag::TagDecayedType
        | ASTEntryTag::TagAttributedType
        | ASTEntryTag::TagParenType => {
            if let Some(CborValue::Integer(inner_id)) = type_node.extras.first() {
                let inner_id = *inner_id as u64;
                resolve_type(ctx, inner_id)
            } else {
                None
            }
        }

        // Typedef type - follow to the underlying type
        // extras[0] = name, extras[1] = underlying type ID
        ASTEntryTag::TagTypedefType => {
            // Always follow the underlying type to get the actual type
            if let Some(CborValue::Integer(underlying_id)) = type_node.extras.get(1) {
                let underlying_id = *underlying_id as u64;
                let result = resolve_type(ctx, underlying_id);
                if result.is_some() {
                    return result;
                }
                // Fall back to typedef name if underlying can't be resolved
                let name = type_node.get_string(0).unwrap_or("").to_string();
                if !name.is_empty() {
                    Some(CppType::Named(name))
                } else {
                    None
                }
            } else {
                // Fallback to typedef name if no underlying type
                let name = type_node.get_string(0).unwrap_or("").to_string();
                if name.is_empty() {
                    None
                } else {
                    Some(CppType::Named(name))
                }
            }
        }

        // Record type (struct/class)
        // extras[0] = decl ID, extras[1] = name
        ASTEntryTag::TagRecordType => {
            // Name is at index 1, not index 0
            let name = type_node.get_string(1).unwrap_or("").to_string();
            if name.is_empty() {
                None
            } else {
                // Clean up the name if it has "struct " or "class " prefix
                let clean_name = name
                    .strip_prefix("struct ")
                    .or_else(|| name.strip_prefix("class "))
                    .unwrap_or(&name)
                    .to_string();
                Some(CppType::Named(clean_name))
            }
        }

        // Pointer type
        ASTEntryTag::TagPointerType => {
            if let Some(CborValue::Integer(pointee_id)) = type_node.extras.first() {
                let pointee_id = *pointee_id as u64;
                // Check const qualifier bit (bit 0) from encodeQualType
                let is_const = (pointee_id & 0x1) != 0;
                if let Some(pointee_type) = resolve_type(ctx, pointee_id) {
                    Some(CppType::Pointer {
                        pointee: Box::new(pointee_type),
                        is_const,
                    })
                } else {
                    Some(CppType::Pointer {
                        pointee: Box::new(CppType::Void),
                        is_const,
                    })
                }
            } else {
                Some(CppType::Pointer {
                    pointee: Box::new(CppType::Void),
                    is_const: false,
                })
            }
        }

        // Reference types (lvalue & rvalue)
        ASTEntryTag::TagLValueReferenceType | ASTEntryTag::TagRValueReferenceType => {
            let is_rvalue = type_node.tag == ASTEntryTag::TagRValueReferenceType;
            if let Some(CborValue::Integer(ref_id)) = type_node.extras.first() {
                let ref_id = *ref_id as u64;
                // Check const qualifier bit (bit 0) from encodeQualType
                let is_const = (ref_id & 0x1) != 0;
                if let Some(ref_type) = resolve_type(ctx, ref_id) {
                    Some(CppType::Reference {
                        referent: Box::new(ref_type),
                        is_const,
                        is_rvalue,
                    })
                } else {
                    Some(CppType::Reference {
                        referent: Box::new(CppType::Void),
                        is_const,
                        is_rvalue,
                    })
                }
            } else {
                Some(CppType::Reference {
                    referent: Box::new(CppType::Void),
                    is_const: false,
                    is_rvalue,
                })
            }
        }

        // Array types
        ASTEntryTag::TagConstantArrayType => {
            if let Some(CborValue::Integer(element_id)) = type_node.extras.first() {
                let element_id = *element_id as u64;
                let element_type = resolve_type(ctx, element_id).unwrap_or(CppType::Void);
                let size = type_node.extras.get(1).and_then(|value| match value {
                    CborValue::Integer(raw) => Some(*raw as usize),
                    _ => None,
                });
                Some(CppType::Array {
                    element: Box::new(element_type),
                    size,
                })
            } else {
                Some(CppType::Array {
                    element: Box::new(CppType::Void),
                    size: None,
                })
            }
        }
        ASTEntryTag::TagIncompleteArrayType
        | ASTEntryTag::TagVariableArrayType
        | ASTEntryTag::TagDependentSizedArrayType => {
            if let Some(CborValue::Integer(element_id)) = type_node.extras.first() {
                let element_id = *element_id as u64;
                let element_type = resolve_type(ctx, element_id).unwrap_or(CppType::Void);
                Some(CppType::Array {
                    element: Box::new(element_type),
                    size: None,
                })
            } else {
                Some(CppType::Array {
                    element: Box::new(CppType::Void),
                    size: None,
                })
            }
        }

        // Function type
        ASTEntryTag::TagFunctionProtoType => {
            let return_type = type_node
                .extras
                .first()
                .and_then(|value| match value {
                    CborValue::Integer(ret_id) => resolve_type(ctx, *ret_id as u64),
                    _ => None,
                })
                .unwrap_or(CppType::Void);

            let mut params = Vec::new();
            if let Some(CborValue::Array(param_ids)) = type_node.extras.get(1) {
                for param_id in param_ids {
                    if let CborValue::Integer(raw_id) = param_id {
                        let param_ty = resolve_type(ctx, *raw_id as u64)
                            .unwrap_or_else(|| CppType::Named("auto".to_string()));
                        params.push(param_ty);
                    }
                }
            }

            let is_variadic = matches!(type_node.extras.get(2), Some(CborValue::Bool(true)));
            Some(CppType::Function {
                return_type: Box::new(return_type),
                params,
                is_variadic,
            })
        }

        // Primitive types
        ASTEntryTag::TagInt => Some(CppType::Int { signed: true }),
        ASTEntryTag::TagUInt => Some(CppType::Int { signed: false }),
        ASTEntryTag::TagLong => Some(CppType::Long { signed: true }),
        ASTEntryTag::TagULong => Some(CppType::Long { signed: false }),
        ASTEntryTag::TagLongLong => Some(CppType::LongLong { signed: true }),
        ASTEntryTag::TagULongLong => Some(CppType::LongLong { signed: false }),
        ASTEntryTag::TagShort => Some(CppType::Short { signed: true }),
        ASTEntryTag::TagUShort => Some(CppType::Short { signed: false }),
        ASTEntryTag::TagChar => Some(CppType::Char { signed: true }),
        ASTEntryTag::TagSChar => Some(CppType::Char { signed: true }),
        ASTEntryTag::TagUChar => Some(CppType::Char { signed: false }),
        ASTEntryTag::TagWChar => Some(CppType::Int { signed: true }),
        ASTEntryTag::TagChar16 => Some(CppType::Short { signed: false }),
        ASTEntryTag::TagChar32 => Some(CppType::Int { signed: false }),
        ASTEntryTag::TagFloat => Some(CppType::Float),
        ASTEntryTag::TagDouble => Some(CppType::Double),
        ASTEntryTag::TagBool => Some(CppType::Bool),
        ASTEntryTag::TagVoid => Some(CppType::Void),

        // Template specialization type (e.g., std::less<int>)
        // extras[0] = template name (string)
        // extras[1] = array of template argument type IDs
        // extras[2] = aliased type ID (for type alias templates like __type_identity_t)
        ASTEntryTag::TagTemplateSpecializationType => {
            let name = type_node.get_string(0).unwrap_or("").to_string();

            // First check if there's an aliased type (for type alias templates)
            // If so, follow that instead of building the name manually
            if let Some(CborValue::Integer(aliased_id)) = type_node.extras.get(2) {
                let aliased_id = *aliased_id as u64;
                if aliased_id != 0 {
                    // This is a type alias template - follow to the aliased type
                    if let Some(resolved) = resolve_type(ctx, aliased_id) {
                        return Some(resolved);
                    }
                    // If aliased type couldn't be resolved, fall through to manual construction
                }
            }

            if name.is_empty() {
                return None;
            }

            // Try to extract template arguments
            if let Some(CborValue::Array(arg_ids)) = type_node.extras.get(1) {
                let mut args = Vec::new();
                for arg_id in arg_ids {
                    if let CborValue::Integer(id) = arg_id {
                        let id = *id as u64;
                        if id != 0 {
                            if let Some(arg_type) = resolve_type(ctx, id) {
                                args.push(format_cpp_type(&arg_type));
                            }
                            // Note: If resolve_type returns None, we skip the arg
                            // This loses information but avoids broken type names
                        }
                        // Note: id == 0 means non-type template argument, skip it
                    }
                }

                if !args.is_empty() {
                    // Build full template name with args
                    let full_name = format!("{}<{}>", name, args.join(", "));
                    Some(CppType::Named(full_name))
                } else {
                    Some(CppType::Named(name))
                }
            } else {
                Some(CppType::Named(name))
            }
        }

        // Fallback: return the type's string representation as a named type
        _ => {
            let name = type_node.get_string(0).unwrap_or("").to_string();
            if name.is_empty() {
                // Try to use tag name as a hint
                Some(CppType::Named(format!("Unknown{:?}", type_node.tag)))
            } else {
                Some(CppType::Named(name))
            }
        }
    }
}
