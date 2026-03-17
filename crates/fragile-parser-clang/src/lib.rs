use fragile_clang::{
    ClangAst, ClangNode, ClangNodeKind, ClangParser, CppType,
    ParserLanguage as ClangParserLanguage,
};
use fragile_parser_core::{
    ParseRequest, ParserBackend, ParserLanguage, ParserNode, ParserOutputV1, ParserTranslationUnit,
    PARSER_OUTPUT_SCHEMA_VERSION_V1,
};
use std::collections::HashSet;
use std::path::Path;

pub const FRAGILE_PARSER_CLANG_BACKEND_ID: &str = "fragile-parser-clang";

#[derive(Debug, Default, Clone, Copy)]
pub struct FragileParserClangBackend;

impl ParserBackend for FragileParserClangBackend {
    fn backend_id(&self) -> &'static str {
        FRAGILE_PARSER_CLANG_BACKEND_ID
    }

    fn parse(&self, request: &ParseRequest) -> Result<ParserOutputV1, String> {
        let include_paths = build_effective_include_paths(request);
        let defines = build_effective_defines(request);
        let parser_language = to_clang_parser_language(request.language);
        let parser = ClangParser::with_paths_defines_and_language(include_paths, defines, parser_language)
            .map_err(|err| format!("failed to initialize clang parser: {err}"))?;
        let ast = parser.parse_file(&request.source_path).map_err(|err| {
            format!(
                "failed to parse `{}` with clang backend: {err}",
                request.source_path.display()
            )
        })?;
        Ok(convert_clang_ast_to_parser_output(request, ast))
    }
}

fn to_clang_parser_language(language: ParserLanguage) -> ClangParserLanguage {
    match language {
        ParserLanguage::C => ClangParserLanguage::C,
        ParserLanguage::Cpp => ClangParserLanguage::Cpp,
    }
}

fn build_effective_include_paths(request: &ParseRequest) -> Vec<String> {
    let requested = request
        .include_directives
        .iter()
        .map(|directive| directive.path.clone());
    let frontend = collect_frontend_include_paths(&request.frontend_args).into_iter();
    dedupe_stable(requested.chain(frontend))
}

fn build_effective_defines(request: &ParseRequest) -> Vec<String> {
    let requested = request.defines.iter().cloned();
    let frontend = collect_frontend_defines(&request.frontend_args).into_iter();
    dedupe_stable(requested.chain(frontend))
}

fn collect_frontend_include_paths(frontend_args: &[String]) -> Vec<String> {
    let mut include_paths = Vec::new();
    let mut index = 0;
    while index < frontend_args.len() {
        let arg = frontend_args[index].as_str();
        if matches!(arg, "-I" | "-isystem" | "-iquote") {
            if let Some(next) = frontend_args.get(index + 1) {
                if let Some(path) = sanitize_frontend_value(next) {
                    include_paths.push(path);
                }
            }
            index += 2;
            continue;
        }
        if let Some(path) = arg
            .strip_prefix("-I")
            .or_else(|| arg.strip_prefix("-isystem"))
            .or_else(|| arg.strip_prefix("-iquote"))
            .and_then(sanitize_frontend_value)
        {
            include_paths.push(path);
        }
        index += 1;
    }
    include_paths
}

fn collect_frontend_defines(frontend_args: &[String]) -> Vec<String> {
    let mut defines = Vec::new();
    let mut index = 0;
    while index < frontend_args.len() {
        let arg = frontend_args[index].as_str();
        if arg == "-D" {
            if let Some(next) = frontend_args.get(index + 1) {
                if let Some(define) = sanitize_frontend_value(next) {
                    defines.push(define);
                }
            }
            index += 2;
            continue;
        }
        if let Some(define) = arg.strip_prefix("-D").and_then(sanitize_frontend_value) {
            defines.push(define);
        }
        index += 1;
    }
    defines
}

fn sanitize_frontend_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix('=').unwrap_or(trimmed).trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn dedupe_stable<I>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for value in values {
        let normalized = value.trim();
        if normalized.is_empty() {
            continue;
        }
        let normalized = normalized.to_string();
        if seen.insert(normalized.clone()) {
            deduped.push(normalized);
        }
    }
    deduped
}

fn convert_clang_ast_to_parser_output(request: &ParseRequest, ast: ClangAst) -> ParserOutputV1 {
    ParserOutputV1 {
        schema_version: PARSER_OUTPUT_SCHEMA_VERSION_V1.to_string(),
        translation_unit: ParserTranslationUnit {
            source_path: request.source_path.clone(),
            language: request.language,
            parser_backend: FRAGILE_PARSER_CLANG_BACKEND_ID.to_string(),
            frontend_args: request.frontend_args.clone(),
            defines: request.defines.clone(),
            include_directives: request.include_directives.clone(),
        },
        nodes: flatten_clang_ast_nodes(&ast.translation_unit),
        diagnostics: Vec::new(),
    }
}

fn flatten_clang_ast_nodes(root: &ClangNode) -> Vec<ParserNode> {
    let mut nodes = Vec::new();
    let mut next_node_id = 0usize;
    flatten_clang_ast_node(root, None, &mut next_node_id, &mut nodes);
    nodes
}

fn flatten_clang_ast_node(
    node: &ClangNode,
    parent_id: Option<&str>,
    next_node_id: &mut usize,
    out: &mut Vec<ParserNode>,
) {
    let node_id = format!("n{}", *next_node_id);
    *next_node_id += 1;
    out.push(ParserNode {
        node_id: node_id.clone(),
        parent_id: parent_id.map(str::to_string),
        node_kind: map_parser_node_kind(&node.kind),
        name: extract_node_name(node),
        cpp_type: extract_cpp_type(&node.kind),
    });
    for child in &node.children {
        flatten_clang_ast_node(child, Some(node_id.as_str()), next_node_id, out);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanonicalStlFamily {
    Vector,
    Map,
    UnorderedMap,
    String,
    Optional,
    Variant,
    Tuple,
    SharedPtr,
    UniquePtr,
}

impl CanonicalStlFamily {
    fn from_symbol_leaf(leaf: &str) -> Option<Self> {
        match leaf {
            "vector" => Some(Self::Vector),
            "map" => Some(Self::Map),
            "unordered_map" => Some(Self::UnorderedMap),
            "basic_string" | "string" => Some(Self::String),
            "optional" => Some(Self::Optional),
            "variant" => Some(Self::Variant),
            "tuple" => Some(Self::Tuple),
            "shared_ptr" => Some(Self::SharedPtr),
            "unique_ptr" => Some(Self::UniquePtr),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Vector => "vector",
            Self::Map => "map",
            Self::UnorderedMap => "unordered_map",
            Self::String => "string",
            Self::Optional => "optional",
            Self::Variant => "variant",
            Self::Tuple => "tuple",
            Self::SharedPtr => "shared_ptr",
            Self::UniquePtr => "unique_ptr",
        }
    }
}

fn is_std_namespace_passthrough(segment: &str) -> bool {
    segment.starts_with("__") || segment == "pmr" || segment == "experimental"
}

fn detect_direct_std_stl_family_in_token(token: &str) -> Option<CanonicalStlFamily> {
    let path = token.trim_matches(':');
    if path.is_empty() || !path.contains("std::") {
        return None;
    }

    let segments = path
        .split("::")
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return None;
    }

    for (index, segment) in segments.iter().enumerate() {
        if *segment != "std" {
            continue;
        }

        let mut symbol_index = index + 1;
        while symbol_index < segments.len() && is_std_namespace_passthrough(segments[symbol_index])
        {
            symbol_index += 1;
        }
        if symbol_index >= segments.len() {
            continue;
        }
        if let Some(family) = CanonicalStlFamily::from_symbol_leaf(segments[symbol_index]) {
            return Some(family);
        }
    }

    None
}

fn detect_direct_std_stl_family_in_spelling(spelling: &str) -> Option<CanonicalStlFamily> {
    spelling
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
        .find_map(detect_direct_std_stl_family_in_token)
}

/// Detects canonical STL family names from direct `std::` spellings in node
/// names or C++ type spellings.
pub fn detect_direct_std_stl_family(
    name: Option<&str>,
    cpp_type: Option<&str>,
) -> Option<&'static str> {
    name
        .into_iter()
        .chain(cpp_type)
        .find_map(|spelling| detect_direct_std_stl_family_in_spelling(spelling))
        .map(CanonicalStlFamily::as_str)
}

fn map_parser_node_kind(kind: &ClangNodeKind) -> String {
    let variant = clang_kind_variant(kind);
    let parser_kind = match variant.as_str() {
        "TranslationUnit" => "translation_unit",
        "NamespaceDecl" => "namespace_decl",
        "RecordDecl" | "UnionDecl" | "ClassTemplateDecl" | "ClassTemplatePartialSpecDecl" => {
            "record_decl"
        }
        "EnumDecl" | "EnumConstantDecl" => "enum_decl",
        "FunctionDecl" | "FunctionTemplateDecl" | "FunctionTemplateInstantiation" => {
            "function_decl"
        }
        "CXXMethodDecl" => "method_decl",
        "ConstructorDecl" => "constructor_decl",
        "DestructorDecl" => "destructor_decl",
        "VarDecl" => "variable_decl",
        "FieldDecl" => "field_decl",
        "ParmVarDecl" => "parameter_decl",
        "TypeAliasDecl" | "TypeAliasTemplateDecl" | "TypedefDecl" | "TemplateTypeParmDecl" => {
            "type_ref"
        }
        _ if variant.ends_with("Expr") => "expression",
        _ if variant.ends_with("Stmt") => "statement",
        _ => "statement",
    };
    parser_kind.to_string()
}

fn clang_kind_variant(kind: &ClangNodeKind) -> String {
    format!("{kind:?}")
        .split(|ch: char| ch == ' ' || ch == '{' || ch == '(')
        .next()
        .unwrap_or("Unknown")
        .to_string()
}

fn extract_node_name(node: &ClangNode) -> Option<String> {
    match &node.kind {
        ClangNodeKind::TranslationUnit => node
            .location
            .file
            .as_ref()
            .and_then(|file| Path::new(file).file_name())
            .map(|file_name| file_name.to_string_lossy().into_owned()),
        ClangNodeKind::FunctionDecl { name, .. }
        | ClangNodeKind::FunctionTemplateDecl { name, .. }
        | ClangNodeKind::FunctionTemplateInstantiation { name, .. }
        | ClangNodeKind::ClassTemplateDecl { name, .. }
        | ClangNodeKind::ClassTemplatePartialSpecDecl { name, .. }
        | ClangNodeKind::TemplateTypeParmDecl { name, .. }
        | ClangNodeKind::ParmVarDecl { name, .. }
        | ClangNodeKind::VarDecl { name, .. }
        | ClangNodeKind::RecordDecl { name, .. }
        | ClangNodeKind::UnionDecl { name, .. }
        | ClangNodeKind::FieldDecl { name, .. }
        | ClangNodeKind::EnumDecl { name, .. }
        | ClangNodeKind::EnumConstantDecl { name, .. }
        | ClangNodeKind::CXXMethodDecl { name, .. }
        | ClangNodeKind::TypeAliasDecl { name, .. }
        | ClangNodeKind::TypeAliasTemplateDecl { name, .. }
        | ClangNodeKind::TypedefDecl { name, .. }
        | ClangNodeKind::DeclRefExpr { name, .. }
        | ClangNodeKind::ConceptDecl { name, .. } => Some(name.clone()),
        ClangNodeKind::ConstructorDecl { class_name, .. }
        | ClangNodeKind::DestructorDecl { class_name, .. } => Some(class_name.clone()),
        ClangNodeKind::NamespaceDecl { name, .. } => name.clone(),
        ClangNodeKind::UsingDirective { namespace } => {
            if namespace.is_empty() {
                None
            } else {
                Some(namespace.join("::"))
            }
        }
        ClangNodeKind::UsingDeclaration { qualified_name } => {
            if qualified_name.is_empty() {
                None
            } else {
                Some(qualified_name.join("::"))
            }
        }
        ClangNodeKind::ModuleImportDecl { module_name, .. } => Some(module_name.clone()),
        ClangNodeKind::GotoStmt { label } | ClangNodeKind::LabelStmt { label } => {
            Some(label.clone())
        }
        ClangNodeKind::MemberExpr { member_name, .. } => Some(member_name.clone()),
        ClangNodeKind::TypeTraitExpr { trait_kind, .. } => Some(format!("{trait_kind:?}")),
        ClangNodeKind::ConceptSpecializationExpr { concept_name, .. } => Some(concept_name.clone()),
        ClangNodeKind::CaseStmt { value, .. } => Some(value.to_string()),
        ClangNodeKind::StringLiteral(value) => Some(value.clone()),
        ClangNodeKind::Unknown(name) => Some(name.clone()),
        _ => None,
    }
}

fn extract_cpp_type(kind: &ClangNodeKind) -> Option<String> {
    match kind {
        ClangNodeKind::FunctionDecl { return_type, .. }
        | ClangNodeKind::FunctionTemplateDecl { return_type, .. }
        | ClangNodeKind::FunctionTemplateInstantiation { return_type, .. }
        | ClangNodeKind::CXXMethodDecl { return_type, .. }
        | ClangNodeKind::LambdaExpr { return_type, .. } => Some(cpp_type_to_string(return_type)),
        ClangNodeKind::ParmVarDecl { ty, .. }
        | ClangNodeKind::VarDecl { ty, .. }
        | ClangNodeKind::FieldDecl { ty, .. }
        | ClangNodeKind::DeclRefExpr { ty, .. }
        | ClangNodeKind::BinaryOperator { ty, .. }
        | ClangNodeKind::UnaryOperator { ty, .. }
        | ClangNodeKind::CallExpr { ty, .. }
        | ClangNodeKind::CXXConstructExpr { ty, .. }
        | ClangNodeKind::MemberExpr { ty, .. }
        | ClangNodeKind::ArraySubscriptExpr { ty }
        | ClangNodeKind::CastExpr { ty, .. }
        | ClangNodeKind::ConditionalOperator { ty }
        | ClangNodeKind::ParenExpr { ty }
        | ClangNodeKind::ImplicitCastExpr { ty, .. }
        | ClangNodeKind::InitListExpr { ty }
        | ClangNodeKind::CXXDefaultInitExpr { ty }
        | ClangNodeKind::CXXThisExpr { ty }
        | ClangNodeKind::DynamicCastExpr { target_ty: ty }
        | ClangNodeKind::CXXNewExpr { ty, .. } => Some(cpp_type_to_string(ty)),
        ClangNodeKind::TypeAliasDecl {
            underlying_type: ty, ..
        }
        | ClangNodeKind::TypeAliasTemplateDecl {
            underlying_type: ty, ..
        }
        | ClangNodeKind::TypedefDecl {
            underlying_type: ty, ..
        } => Some(cpp_type_to_string(ty)),
        ClangNodeKind::EnumDecl {
            underlying_type: ty, ..
        } => Some(cpp_type_to_string(ty)),
        ClangNodeKind::CXXForRangeStmt { var_type, .. } => Some(cpp_type_to_string(var_type)),
        ClangNodeKind::EvaluatedExpr { ty, .. } => Some(cpp_type_to_string(ty)),
        ClangNodeKind::TypeidExpr { result_ty, .. } => Some(cpp_type_to_string(result_ty)),
        ClangNodeKind::CoawaitExpr { result_ty, .. } => Some(cpp_type_to_string(result_ty)),
        ClangNodeKind::CoyieldExpr { result_ty, .. } => Some(cpp_type_to_string(result_ty)),
        ClangNodeKind::CoreturnStmt { value_ty } => value_ty.as_ref().map(cpp_type_to_string),
        ClangNodeKind::CatchStmt { exception_ty } | ClangNodeKind::ThrowExpr { exception_ty } => {
            exception_ty.as_ref().map(cpp_type_to_string)
        }
        ClangNodeKind::UnaryExprOrTypeTraitExpr { argument_type, .. } => {
            argument_type.as_ref().map(cpp_type_to_string)
        }
        ClangNodeKind::IntegerLiteral {
            cpp_type: Some(ty), ..
        }
        | ClangNodeKind::FloatingLiteral {
            cpp_type: Some(ty), ..
        } => Some(cpp_type_to_string(ty)),
        ClangNodeKind::TypeTraitExpr { type_args, .. } => {
            if type_args.is_empty() {
                None
            } else {
                Some(
                    type_args
                        .iter()
                        .map(cpp_type_to_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }
        }
        _ => None,
    }
}

fn cpp_type_to_string(ty: &CppType) -> String {
    match ty {
        CppType::Void => "void".to_string(),
        CppType::Bool => "bool".to_string(),
        CppType::Char { signed: true } => "signed char".to_string(),
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
        CppType::Pointer { pointee, is_const } => {
            if *is_const {
                format!("const {}*", cpp_type_to_string(pointee))
            } else {
                format!("{}*", cpp_type_to_string(pointee))
            }
        }
        CppType::Reference {
            referent,
            is_const,
            is_rvalue,
        } => {
            let suffix = if *is_rvalue { "&&" } else { "&" };
            if *is_const {
                format!("const {}{}", cpp_type_to_string(referent), suffix)
            } else {
                format!("{}{}", cpp_type_to_string(referent), suffix)
            }
        }
        CppType::Array { element, size } => match size {
            Some(count) => format!("{}[{}]", cpp_type_to_string(element), count),
            None => format!("{}[]", cpp_type_to_string(element)),
        },
        CppType::Named(name) => name.clone(),
        CppType::Function {
            return_type,
            params,
            is_variadic,
        } => {
            let mut args = params.iter().map(cpp_type_to_string).collect::<Vec<_>>();
            if *is_variadic {
                args.push("...".to_string());
            }
            format!("{}({})", cpp_type_to_string(return_type), args.join(", "))
        }
        CppType::TemplateParam { name, .. } | CppType::ParameterPack { name, .. } => name.clone(),
        CppType::DependentType { spelling } => spelling.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        detect_direct_std_stl_family, FragileParserClangBackend, FRAGILE_PARSER_CLANG_BACKEND_ID,
    };
    use fragile_parser_core::{
        IncludeDirective, IncludeDirectiveKind, ParseRequest, ParserBackend, ParserLanguage,
        PARSER_OUTPUT_SCHEMA_VERSION_V1,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("fragile_parser_clang_{label}_{stamp}"));
        fs::create_dir_all(&dir).expect("failed to create temporary directory");
        dir
    }

    fn build_request(
        source_path: PathBuf,
        language: ParserLanguage,
        frontend_args: Vec<String>,
        defines: Vec<String>,
        include_directives: Vec<IncludeDirective>,
    ) -> ParseRequest {
        ParseRequest {
            source_path,
            language,
            frontend_args,
            defines,
            include_directives,
        }
    }

    fn assert_sequential_node_ids(node_ids: &[String]) {
        for (idx, node_id) in node_ids.iter().enumerate() {
            assert_eq!(
                node_id,
                &format!("n{idx}"),
                "node ids should be deterministic pre-order indices"
            );
        }
    }

    #[test]
    fn backend_id_matches_contract_constant() {
        let backend = FragileParserClangBackend;
        assert_eq!(backend.backend_id(), FRAGILE_PARSER_CLANG_BACKEND_ID);
    }

    #[test]
    fn detect_direct_std_stl_family_matches_known_std_shapes() {
        let cases = [
            (Some("std::vector<int>"), None, Some("vector")),
            (Some("std::map<int, int>"), None, Some("map")),
            (
                Some("std::__1::unordered_map<int, int>"),
                None,
                Some("unordered_map"),
            ),
            (Some("std::basic_string<char>"), None, Some("string")),
            (Some("std::string"), None, Some("string")),
            (Some("std::optional<int>"), None, Some("optional")),
            (Some("std::variant<int, double>"), None, Some("variant")),
            (Some("std::tuple<int, double>"), None, Some("tuple")),
            (
                Some("std::shared_ptr<MyType>"),
                None,
                Some("shared_ptr"),
            ),
            (
                Some("std::unique_ptr<MyType>"),
                None,
                Some("unique_ptr"),
            ),
            (
                Some("alias"),
                Some("const std::pmr::vector<int>&"),
                Some("vector"),
            ),
        ];

        for (name, cpp_type, expected) in cases {
            assert_eq!(
                detect_direct_std_stl_family(name, cpp_type),
                expected,
                "unexpected direct std STL family detection for name={:?}, cpp_type={:?}",
                name,
                cpp_type
            );
        }
    }

    #[test]
    fn detect_direct_std_stl_family_rejects_non_std_or_non_target_symbols() {
        let cases = [
            (Some("mystd::vector<int>"), None),
            (Some("std::allocator<int>"), None),
            (Some("my::tuple<int, int>"), None),
            (Some("vector<int>"), None),
            (Some("std::filesystem::path"), None),
            (Some("LocalType"), Some("LocalType*")),
        ];

        for (name, cpp_type) in cases {
            assert_eq!(
                detect_direct_std_stl_family(name, cpp_type),
                None,
                "expected no direct std STL family for name={:?}, cpp_type={:?}",
                name,
                cpp_type
            );
        }
    }

    #[test]
    fn parse_c_file_emits_v1_output_and_deterministic_node_ids() {
        let temp_dir = unique_temp_dir("c_basic");
        let source = temp_dir.join("basic.c");
        fs::write(
            &source,
            r#"
int add(int lhs, int rhs) {
    return lhs + rhs;
}
"#,
        )
        .expect("failed to write C source");

        let request = build_request(
            source.clone(),
            ParserLanguage::C,
            Vec::new(),
            vec!["C_MODE_TEST=1".to_string()],
            Vec::new(),
        );
        let backend = FragileParserClangBackend;
        let output = backend.parse(&request).expect("C parse should succeed");
        assert_eq!(output.schema_version, PARSER_OUTPUT_SCHEMA_VERSION_V1);
        assert_eq!(output.translation_unit.source_path, source);
        assert_eq!(
            output.translation_unit.parser_backend,
            FRAGILE_PARSER_CLANG_BACKEND_ID
        );
        assert_eq!(output.translation_unit.language, ParserLanguage::C);
        assert_eq!(output.translation_unit.defines, vec!["C_MODE_TEST=1".to_string()]);
        assert_eq!(output.nodes.first().map(|node| node.node_kind.as_str()), Some("translation_unit"));
        assert!(
            output
                .nodes
                .iter()
                .any(|node| node.node_kind == "function_decl" && node.name.as_deref() == Some("add")),
            "expected function_decl for add in parser output"
        );
        assert_sequential_node_ids(
            &output
                .nodes
                .iter()
                .map(|node| node.node_id.clone())
                .collect::<Vec<_>>(),
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn parse_cpp_honors_include_directives_and_explicit_defines() {
        let temp_dir = unique_temp_dir("cpp_include_directive");
        let include_dir = temp_dir.join("include");
        fs::create_dir_all(&include_dir).expect("failed to create include directory");
        let header = include_dir.join("config_header.h");
        let source = temp_dir.join("main.cpp");
        fs::write(
            &header,
            r#"
#pragma once
inline int config_value() { return PARSER_DEFINE_FLAG; }
"#,
        )
        .expect("failed to write header");
        fs::write(
            &source,
            r#"
#include "config_header.h"
#ifndef PARSER_DEFINE_FLAG
#error PARSER_DEFINE_FLAG must be defined
#endif
int from_config() { return config_value(); }
"#,
        )
        .expect("failed to write C++ source");

        let request = build_request(
            source.clone(),
            ParserLanguage::Cpp,
            Vec::new(),
            vec!["PARSER_DEFINE_FLAG=42".to_string()],
            vec![IncludeDirective {
                kind: IncludeDirectiveKind::Include,
                path: include_dir.to_string_lossy().into_owned(),
            }],
        );
        let backend = FragileParserClangBackend;
        let output = backend
            .parse(&request)
            .expect("C++ parse should succeed with include directives and defines");
        assert_eq!(output.translation_unit.language, ParserLanguage::Cpp);
        assert_eq!(
            output.translation_unit.include_directives,
            request.include_directives
        );
        assert_eq!(output.translation_unit.defines, request.defines);
        assert!(
            output.nodes.iter().any(|node| {
                node.node_kind == "function_decl" && node.name.as_deref() == Some("from_config")
            }),
            "expected from_config function_decl in parser output"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn parse_cpp_honors_frontend_args_for_include_and_define() {
        let temp_dir = unique_temp_dir("cpp_frontend_args");
        let include_dir = temp_dir.join("headers");
        fs::create_dir_all(&include_dir).expect("failed to create headers directory");
        let header = include_dir.join("frontend_header.h");
        let source = temp_dir.join("frontend.cpp");
        fs::write(
            &header,
            r#"
#pragma once
inline int frontend_flag_value() { return FRONTEND_DEFINE_FLAG; }
"#,
        )
        .expect("failed to write frontend header");
        fs::write(
            &source,
            r#"
#include "frontend_header.h"
#ifndef FRONTEND_DEFINE_FLAG
#error FRONTEND_DEFINE_FLAG must be defined
#endif
int from_frontend() { return frontend_flag_value(); }
"#,
        )
        .expect("failed to write frontend source");

        let request = build_request(
            source,
            ParserLanguage::Cpp,
            vec![
                "-I".to_string(),
                include_dir.to_string_lossy().into_owned(),
                "-D".to_string(),
                "FRONTEND_DEFINE_FLAG=7".to_string(),
            ],
            Vec::new(),
            Vec::new(),
        );
        let backend = FragileParserClangBackend;
        let output = backend
            .parse(&request)
            .expect("C++ parse should succeed with frontend include/define args");
        assert!(
            output.nodes.iter().any(|node| {
                node.node_kind == "function_decl" && node.name.as_deref() == Some("from_frontend")
            }),
            "expected from_frontend function_decl in parser output"
        );
        assert_eq!(output.translation_unit.frontend_args, request.frontend_args);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn parse_reports_missing_source_path() {
        let missing = std::env::temp_dir().join("fragile_parser_clang_missing.cpp");
        let request = build_request(
            missing.clone(),
            ParserLanguage::Cpp,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let backend = FragileParserClangBackend;
        let err = backend
            .parse(&request)
            .expect_err("parse should fail for missing source path");
        assert!(
            err.contains("failed to parse"),
            "expected parse failure wrapper, got: {err}"
        );
        assert!(
            err.contains(&missing.display().to_string()),
            "expected missing source path in error, got: {err}"
        );
    }
}
