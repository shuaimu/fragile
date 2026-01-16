//! Clang AST parsing using libclang.

use crate::ast::{
    AccessSpecifier, BinaryOp, CastKind, ClangAst, ClangNode, ClangNodeKind, ConstructorKind,
    SourceLocation, UnaryOp,
};
use crate::types::CppType;
use miette::{miette, Result};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;

/// Parser that uses libclang to parse C++ source files.
pub struct ClangParser {
    index: clang_sys::CXIndex,
}

impl ClangParser {
    /// Create a new Clang parser.
    pub fn new() -> Result<Self> {
        unsafe {
            let index = clang_sys::clang_createIndex(0, 0);
            if index.is_null() {
                return Err(miette!("Failed to create clang index"));
            }
            Ok(Self { index })
        }
    }

    /// Parse a C++ source file into a Clang AST.
    pub fn parse_file(&self, path: &Path) -> Result<ClangAst> {
        let path_str = path.to_string_lossy();
        let c_path = CString::new(path_str.as_ref())
            .map_err(|_| miette!("Invalid path: {}", path_str))?;

        // Compiler arguments for C++ parsing
        let args: Vec<CString> = vec![
            CString::new("-x").unwrap(),
            CString::new("c++").unwrap(),
            CString::new("-std=c++17").unwrap(),
        ];
        let c_args: Vec<*const i8> = args.iter().map(|s| s.as_ptr()).collect();

        unsafe {
            let tu = clang_sys::clang_parseTranslationUnit(
                self.index,
                c_path.as_ptr(),
                c_args.as_ptr(),
                c_args.len() as i32,
                ptr::null_mut(),
                0,
                clang_sys::CXTranslationUnit_None,
            );

            if tu.is_null() {
                return Err(miette!("Failed to parse file: {}", path_str));
            }

            // Check for errors
            let num_diagnostics = clang_sys::clang_getNumDiagnostics(tu);
            for i in 0..num_diagnostics {
                let diag = clang_sys::clang_getDiagnostic(tu, i);
                let severity = clang_sys::clang_getDiagnosticSeverity(diag);

                if severity >= clang_sys::CXDiagnostic_Error {
                    let spelling = clang_sys::clang_getDiagnosticSpelling(diag);
                    let msg = cx_string_to_string(spelling);
                    clang_sys::clang_disposeDiagnostic(diag);
                    clang_sys::clang_disposeTranslationUnit(tu);
                    return Err(miette!("Clang error: {}", msg));
                }
                clang_sys::clang_disposeDiagnostic(diag);
            }

            // Get the cursor for the translation unit
            let cursor = clang_sys::clang_getTranslationUnitCursor(tu);

            // Convert to our AST representation
            let root = self.convert_cursor(cursor);

            clang_sys::clang_disposeTranslationUnit(tu);

            Ok(ClangAst {
                translation_unit: root,
            })
        }
    }

    /// Parse C++ source code from a string.
    pub fn parse_string(&self, source: &str, filename: &str) -> Result<ClangAst> {
        let c_filename = CString::new(filename).unwrap();
        let c_source = CString::new(source).unwrap();

        // Create an unsaved file
        let unsaved_file = clang_sys::CXUnsavedFile {
            Filename: c_filename.as_ptr(),
            Contents: c_source.as_ptr(),
            Length: source.len() as u64,
        };

        // Compiler arguments
        let args: Vec<CString> = vec![
            CString::new("-x").unwrap(),
            CString::new("c++").unwrap(),
            CString::new("-std=c++17").unwrap(),
        ];
        let c_args: Vec<*const i8> = args.iter().map(|s| s.as_ptr()).collect();

        unsafe {
            let tu = clang_sys::clang_parseTranslationUnit(
                self.index,
                c_filename.as_ptr(),
                c_args.as_ptr(),
                c_args.len() as i32,
                &unsaved_file as *const _ as *mut _,
                1,
                clang_sys::CXTranslationUnit_None,
            );

            if tu.is_null() {
                return Err(miette!("Failed to parse source code"));
            }

            let cursor = clang_sys::clang_getTranslationUnitCursor(tu);
            let root = self.convert_cursor(cursor);

            clang_sys::clang_disposeTranslationUnit(tu);

            Ok(ClangAst {
                translation_unit: root,
            })
        }
    }

    /// Convert a Clang cursor to our AST node.
    fn convert_cursor(&self, cursor: clang_sys::CXCursor) -> ClangNode {
        unsafe {
            let kind = clang_sys::clang_getCursorKind(cursor);
            let location = self.get_location(cursor);

            let node_kind = self.convert_cursor_kind(cursor, kind);

            // Get children
            let mut children = Vec::new();
            let children_ptr: *mut Vec<ClangNode> = &mut children;

            extern "C" fn visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let parser = &*(data as *const ClangParser);
                    let children = &mut *(data as *mut Vec<ClangNode>);

                    // Skip null cursors
                    if clang_sys::clang_Cursor_isNull(child) != 0 {
                        return clang_sys::CXChildVisit_Continue;
                    }

                    children.push(parser.convert_cursor(child));
                    clang_sys::CXChildVisit_Continue
                }
            }

            // Visit children
            clang_sys::clang_visitChildren(cursor, visitor, children_ptr as clang_sys::CXClientData);

            ClangNode {
                kind: node_kind,
                children,
                location,
            }
        }
    }

    /// Get source location from cursor.
    fn get_location(&self, cursor: clang_sys::CXCursor) -> SourceLocation {
        unsafe {
            let loc = clang_sys::clang_getCursorLocation(cursor);
            let mut file: clang_sys::CXFile = ptr::null_mut();
            let mut line: u32 = 0;
            let mut column: u32 = 0;

            clang_sys::clang_getSpellingLocation(
                loc,
                &mut file,
                &mut line,
                &mut column,
                ptr::null_mut(),
            );

            let file_name = if !file.is_null() {
                let name = clang_sys::clang_getFileName(file);
                Some(cx_string_to_string(name))
            } else {
                None
            };

            SourceLocation {
                file: file_name,
                line,
                column,
            }
        }
    }

    /// Convert a Clang cursor kind to our AST node kind.
    fn convert_cursor_kind(
        &self,
        cursor: clang_sys::CXCursor,
        kind: clang_sys::CXCursorKind,
    ) -> ClangNodeKind {
        unsafe {
            match kind {
                clang_sys::CXCursor_TranslationUnit => ClangNodeKind::TranslationUnit,

                // Declarations
                clang_sys::CXCursor_FunctionDecl => {
                    let name = cursor_spelling(cursor);
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let return_type = self.convert_type(clang_sys::clang_getResultType(cursor_type));
                    let num_args = clang_sys::clang_Cursor_getNumArguments(cursor);

                    let mut params = Vec::new();
                    for i in 0..num_args {
                        let arg = clang_sys::clang_Cursor_getArgument(cursor, i as u32);
                        let arg_name = cursor_spelling(arg);
                        let arg_type = clang_sys::clang_getCursorType(arg);
                        params.push((arg_name, self.convert_type(arg_type)));
                    }

                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;

                    ClangNodeKind::FunctionDecl {
                        name,
                        return_type,
                        params,
                        is_definition,
                    }
                }

                clang_sys::CXCursor_ParmDecl => {
                    let name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::ParmVarDecl { name, ty }
                }

                clang_sys::CXCursor_VarDecl => {
                    let name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // Check if it has an initializer by looking at children
                    let has_init = false; // Will be determined by children
                    ClangNodeKind::VarDecl { name, ty, has_init }
                }

                clang_sys::CXCursor_StructDecl | clang_sys::CXCursor_ClassDecl => {
                    let name = cursor_spelling(cursor);
                    let is_class = kind == clang_sys::CXCursor_ClassDecl;
                    // Fields will be collected from children
                    ClangNodeKind::RecordDecl {
                        name,
                        is_class,
                        fields: Vec::new(),
                    }
                }

                clang_sys::CXCursor_FieldDecl => {
                    let name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    let access = self.get_access_specifier(cursor);
                    ClangNodeKind::FieldDecl { name, ty, access }
                }

                clang_sys::CXCursor_Constructor => {
                    let class_name = self.get_parent_class_name(cursor);
                    let params = self.extract_params(cursor);
                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;
                    let ctor_kind = self.get_constructor_kind(cursor);
                    let access = self.get_access_specifier(cursor);
                    ClangNodeKind::ConstructorDecl {
                        class_name,
                        params,
                        is_definition,
                        ctor_kind,
                        access,
                    }
                }

                clang_sys::CXCursor_Destructor => {
                    let class_name = self.get_parent_class_name(cursor);
                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;
                    let access = self.get_access_specifier(cursor);
                    ClangNodeKind::DestructorDecl {
                        class_name,
                        is_definition,
                        access,
                    }
                }

                clang_sys::CXCursor_Namespace => {
                    let name = cursor_spelling(cursor);
                    let name_opt = if name.is_empty() { None } else { Some(name) };
                    ClangNodeKind::NamespaceDecl { name: name_opt }
                }

                clang_sys::CXCursor_UsingDirective => {
                    // For UsingDirective, the child nodes contain a NamespaceRef
                    // that points to the nominated namespace
                    let namespace = self.get_using_directive_namespace(cursor);
                    ClangNodeKind::UsingDirective { namespace }
                }

                clang_sys::CXCursor_UsingDeclaration => {
                    // Get the qualified name from the using declaration
                    let qualified_name = self.get_qualified_name(cursor);
                    ClangNodeKind::UsingDeclaration { qualified_name }
                }

                clang_sys::CXCursor_MemberRef => {
                    let name = cursor_spelling(cursor);
                    ClangNodeKind::MemberRef { name }
                }

                // Statements
                clang_sys::CXCursor_CompoundStmt => ClangNodeKind::CompoundStmt,
                clang_sys::CXCursor_ReturnStmt => ClangNodeKind::ReturnStmt,
                clang_sys::CXCursor_IfStmt => ClangNodeKind::IfStmt,
                clang_sys::CXCursor_WhileStmt => ClangNodeKind::WhileStmt,
                clang_sys::CXCursor_ForStmt => ClangNodeKind::ForStmt,
                clang_sys::CXCursor_DeclStmt => ClangNodeKind::DeclStmt,
                clang_sys::CXCursor_BreakStmt => ClangNodeKind::BreakStmt,
                clang_sys::CXCursor_ContinueStmt => ClangNodeKind::ContinueStmt,

                // Expressions
                clang_sys::CXCursor_IntegerLiteral => {
                    // Try to evaluate the literal
                    let eval = clang_sys::clang_Cursor_Evaluate(cursor);
                    if eval.is_null() {
                        ClangNodeKind::IntegerLiteral(0)
                    } else {
                        let result = clang_sys::clang_EvalResult_getAsInt(eval) as i128;
                        clang_sys::clang_EvalResult_dispose(eval);
                        ClangNodeKind::IntegerLiteral(result)
                    }
                }

                clang_sys::CXCursor_FloatingLiteral => {
                    let eval = clang_sys::clang_Cursor_Evaluate(cursor);
                    let result = if !eval.is_null() {
                        let val = clang_sys::clang_EvalResult_getAsDouble(eval);
                        clang_sys::clang_EvalResult_dispose(eval);
                        val
                    } else {
                        0.0
                    };
                    ClangNodeKind::FloatingLiteral(result)
                }

                clang_sys::CXCursor_DeclRefExpr => {
                    let name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::DeclRefExpr { name, ty }
                }

                clang_sys::CXCursor_BinaryOperator => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // Get the operator from tokens
                    let op = self.get_binary_op(cursor);
                    ClangNodeKind::BinaryOperator { op, ty }
                }

                clang_sys::CXCursor_UnaryOperator => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    let op = self.get_unary_op(cursor);
                    ClangNodeKind::UnaryOperator { op, ty }
                }

                clang_sys::CXCursor_CallExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::CallExpr { ty }
                }

                clang_sys::CXCursor_MemberRefExpr => {
                    let member_name = cursor_spelling(cursor);
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // Check if arrow or dot
                    let is_arrow = false; // TODO: determine from cursor
                    ClangNodeKind::MemberExpr {
                        member_name,
                        is_arrow,
                        ty,
                    }
                }

                clang_sys::CXCursor_ArraySubscriptExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::ArraySubscriptExpr { ty }
                }

                clang_sys::CXCursor_ParenExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::ParenExpr { ty }
                }

                clang_sys::CXCursor_CStyleCastExpr => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::CastExpr {
                        cast_kind: CastKind::Other,
                        ty,
                    }
                }

                clang_sys::CXCursor_ConditionalOperator => {
                    let ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::ConditionalOperator { ty }
                }

                _ => {
                    let kind_spelling = clang_sys::clang_getCursorKindSpelling(kind);
                    ClangNodeKind::Unknown(cx_string_to_string(kind_spelling))
                }
            }
        }
    }

    /// Convert a Clang type to our type representation.
    fn convert_type(&self, ty: clang_sys::CXType) -> CppType {
        unsafe {
            let kind = ty.kind;
            match kind {
                clang_sys::CXType_Void => CppType::Void,
                clang_sys::CXType_Bool => CppType::Bool,
                clang_sys::CXType_Char_S | clang_sys::CXType_SChar => {
                    CppType::Char { signed: true }
                }
                clang_sys::CXType_Char_U | clang_sys::CXType_UChar => {
                    CppType::Char { signed: false }
                }
                clang_sys::CXType_Short => CppType::Short { signed: true },
                clang_sys::CXType_UShort => CppType::Short { signed: false },
                clang_sys::CXType_Int => CppType::Int { signed: true },
                clang_sys::CXType_UInt => CppType::Int { signed: false },
                clang_sys::CXType_Long => CppType::Long { signed: true },
                clang_sys::CXType_ULong => CppType::Long { signed: false },
                clang_sys::CXType_LongLong => CppType::LongLong { signed: true },
                clang_sys::CXType_ULongLong => CppType::LongLong { signed: false },
                clang_sys::CXType_Float => CppType::Float,
                clang_sys::CXType_Double => CppType::Double,

                clang_sys::CXType_Pointer => {
                    let pointee = clang_sys::clang_getPointeeType(ty);
                    let is_const = clang_sys::clang_isConstQualifiedType(pointee) != 0;
                    CppType::Pointer {
                        pointee: Box::new(self.convert_type(pointee)),
                        is_const,
                    }
                }

                clang_sys::CXType_LValueReference | clang_sys::CXType_RValueReference => {
                    let referent = clang_sys::clang_getPointeeType(ty);
                    let is_const = clang_sys::clang_isConstQualifiedType(referent) != 0;
                    CppType::Reference {
                        referent: Box::new(self.convert_type(referent)),
                        is_const,
                    }
                }

                clang_sys::CXType_ConstantArray => {
                    let element = clang_sys::clang_getArrayElementType(ty);
                    let size = clang_sys::clang_getArraySize(ty) as usize;
                    CppType::Array {
                        element: Box::new(self.convert_type(element)),
                        size: Some(size),
                    }
                }

                clang_sys::CXType_IncompleteArray => {
                    let element = clang_sys::clang_getArrayElementType(ty);
                    CppType::Array {
                        element: Box::new(self.convert_type(element)),
                        size: None,
                    }
                }

                clang_sys::CXType_Record | clang_sys::CXType_Elaborated => {
                    let spelling = clang_sys::clang_getTypeSpelling(ty);
                    let name = cx_string_to_string(spelling);
                    // Clean up the name (remove "struct " prefix, etc.)
                    let name = name
                        .trim_start_matches("struct ")
                        .trim_start_matches("class ")
                        .to_string();
                    CppType::Named(name)
                }

                clang_sys::CXType_FunctionProto => {
                    let return_type = clang_sys::clang_getResultType(ty);
                    let num_args = clang_sys::clang_getNumArgTypes(ty);
                    let mut params = Vec::new();
                    for i in 0..num_args {
                        let arg_type = clang_sys::clang_getArgType(ty, i as u32);
                        params.push(self.convert_type(arg_type));
                    }
                    let is_variadic = clang_sys::clang_isFunctionTypeVariadic(ty) != 0;

                    CppType::Function {
                        return_type: Box::new(self.convert_type(return_type)),
                        params,
                        is_variadic,
                    }
                }

                _ => {
                    let spelling = clang_sys::clang_getTypeSpelling(ty);
                    CppType::Named(cx_string_to_string(spelling))
                }
            }
        }
    }

    /// Get binary operator from cursor.
    fn get_binary_op(&self, _cursor: clang_sys::CXCursor) -> BinaryOp {
        // TODO: Extract actual operator from tokens
        // For now, default to Add - will be fixed in Phase 2
        BinaryOp::Add
    }

    /// Get unary operator from cursor.
    fn get_unary_op(&self, _cursor: clang_sys::CXCursor) -> UnaryOp {
        // TODO: Extract actual operator from tokens
        // For now, default to Minus - will be fixed in Phase 2
        UnaryOp::Minus
    }

    /// Get the access specifier for a cursor (field, method, etc.).
    fn get_access_specifier(&self, cursor: clang_sys::CXCursor) -> AccessSpecifier {
        unsafe {
            let access = clang_sys::clang_getCXXAccessSpecifier(cursor);
            match access {
                clang_sys::CX_CXXPublic => AccessSpecifier::Public,
                clang_sys::CX_CXXProtected => AccessSpecifier::Protected,
                clang_sys::CX_CXXPrivate => AccessSpecifier::Private,
                _ => AccessSpecifier::Private, // Default to private for invalid/unknown
            }
        }
    }

    /// Get the constructor kind for a constructor cursor.
    fn get_constructor_kind(&self, cursor: clang_sys::CXCursor) -> ConstructorKind {
        unsafe {
            if clang_sys::clang_CXXConstructor_isDefaultConstructor(cursor) != 0 {
                ConstructorKind::Default
            } else if clang_sys::clang_CXXConstructor_isCopyConstructor(cursor) != 0 {
                ConstructorKind::Copy
            } else if clang_sys::clang_CXXConstructor_isMoveConstructor(cursor) != 0 {
                ConstructorKind::Move
            } else {
                ConstructorKind::Other
            }
        }
    }

    /// Get the parent class name for a member cursor (constructor, destructor, method).
    fn get_parent_class_name(&self, cursor: clang_sys::CXCursor) -> String {
        unsafe {
            let parent = clang_sys::clang_getCursorSemanticParent(cursor);
            cursor_spelling(parent)
        }
    }

    /// Extract function parameters from a cursor (function, constructor, etc.).
    fn extract_params(&self, cursor: clang_sys::CXCursor) -> Vec<(String, CppType)> {
        unsafe {
            let num_args = clang_sys::clang_Cursor_getNumArguments(cursor);
            let mut params = Vec::new();
            for i in 0..num_args {
                let arg = clang_sys::clang_Cursor_getArgument(cursor, i as u32);
                let arg_name = cursor_spelling(arg);
                let arg_type = clang_sys::clang_getCursorType(arg);
                params.push((arg_name, self.convert_type(arg_type)));
            }
            params
        }
    }

    /// Get the namespace path from a UsingDirective cursor by examining its children.
    fn get_using_directive_namespace(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        unsafe {
            // The using directive has NamespaceRef children pointing to the namespace(s)
            // We need to visit children and collect namespace names
            let mut namespace_path = Vec::new();
            let namespace_path_ptr: *mut Vec<String> = &mut namespace_path;

            extern "C" fn namespace_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let path = &mut *(data as *mut Vec<String>);
                    let kind = clang_sys::clang_getCursorKind(child);

                    if kind == clang_sys::CXCursor_NamespaceRef {
                        let name = cursor_spelling(child);
                        if !name.is_empty() {
                            path.push(name);
                        }
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(cursor, namespace_visitor, namespace_path_ptr as clang_sys::CXClientData);

            namespace_path
        }
    }

    /// Get the namespace path from a UsingDirective cursor.
    #[allow(dead_code)]
    fn get_namespace_path(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        unsafe {
            // Get the nominated namespace from the using directive
            let referenced = clang_sys::clang_getCursorReferenced(cursor);
            if clang_sys::clang_Cursor_isNull(referenced) != 0 {
                return Vec::new();
            }

            // Build the namespace path by traversing semantic parents
            self.build_namespace_path(referenced)
        }
    }

    /// Get the qualified name from a UsingDeclaration cursor.
    fn get_qualified_name(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        unsafe {
            // Get the referenced declaration
            let referenced = clang_sys::clang_getCursorReferenced(cursor);
            if clang_sys::clang_Cursor_isNull(referenced) != 0 {
                return Vec::new();
            }

            // Build the qualified name path
            self.build_namespace_path(referenced)
        }
    }

    /// Build a namespace path by traversing semantic parents.
    /// Returns the full qualified path for the given cursor.
    fn build_namespace_path(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        unsafe {
            let mut path = Vec::new();
            let mut current = cursor;

            // First, add the cursor itself if it's a namespace
            let kind = clang_sys::clang_getCursorKind(current);
            if kind == clang_sys::CXCursor_Namespace {
                let name = cursor_spelling(current);
                if !name.is_empty() {
                    path.push(name);
                }
            }

            // Traverse up through parents to build the full path
            loop {
                // Move to parent
                let parent = clang_sys::clang_getCursorSemanticParent(current);
                let parent_kind = clang_sys::clang_getCursorKind(parent);

                // Stop at translation unit or if parent is same as current
                if parent_kind == clang_sys::CXCursor_TranslationUnit
                    || clang_sys::clang_Cursor_isNull(parent) != 0
                    || clang_sys::clang_equalCursors(current, parent) != 0
                {
                    break;
                }

                // Add parent namespace to path
                if parent_kind == clang_sys::CXCursor_Namespace {
                    let name = cursor_spelling(parent);
                    if !name.is_empty() {
                        path.push(name);
                    }
                }

                current = parent;
            }

            // Reverse to get outermost first (outer::inner -> ["outer", "inner"])
            path.reverse();
            path
        }
    }
}

impl Drop for ClangParser {
    fn drop(&mut self) {
        unsafe {
            clang_sys::clang_disposeIndex(self.index);
        }
    }
}

/// Convert a CXString to a Rust String.
fn cx_string_to_string(cx_string: clang_sys::CXString) -> String {
    unsafe {
        let c_str = clang_sys::clang_getCString(cx_string);
        let result = if c_str.is_null() {
            String::new()
        } else {
            CStr::from_ptr(c_str).to_string_lossy().into_owned()
        };
        clang_sys::clang_disposeString(cx_string);
        result
    }
}

/// Get the spelling of a cursor.
fn cursor_spelling(cursor: clang_sys::CXCursor) -> String {
    unsafe {
        let spelling = clang_sys::clang_getCursorSpelling(cursor);
        cx_string_to_string(spelling)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int add(int a, int b) {
                    return a + b;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Check that we got a translation unit
        match &ast.translation_unit.kind {
            ClangNodeKind::TranslationUnit => {}
            _ => panic!("Expected TranslationUnit"),
        }
    }

    #[test]
    fn test_parse_namespace() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                namespace foo {
                    int bar() {
                        return 42;
                    }
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        // Check that we got a translation unit
        assert!(matches!(
            ast.translation_unit.kind,
            ClangNodeKind::TranslationUnit
        ));

        // Should have a namespace child
        assert!(!ast.translation_unit.children.is_empty());
        let ns = &ast.translation_unit.children[0];
        match &ns.kind {
            ClangNodeKind::NamespaceDecl { name } => {
                assert_eq!(name.as_deref(), Some("foo"));
            }
            _ => panic!("Expected NamespaceDecl, got {:?}", ns.kind),
        }

        // Namespace should have a function child
        assert!(!ns.children.is_empty());
        match &ns.children[0].kind {
            ClangNodeKind::FunctionDecl { name, .. } => {
                assert_eq!(name, "bar");
            }
            _ => panic!("Expected FunctionDecl inside namespace"),
        }
    }

    #[test]
    fn test_parse_anonymous_namespace() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                namespace {
                    int hidden() {
                        return 0;
                    }
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        assert!(!ast.translation_unit.children.is_empty());
        let ns = &ast.translation_unit.children[0];
        match &ns.kind {
            ClangNodeKind::NamespaceDecl { name } => {
                assert!(name.is_none(), "Expected anonymous namespace");
            }
            _ => panic!("Expected NamespaceDecl"),
        }
    }

    #[test]
    fn test_parse_using_namespace() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                namespace foo {
                    int x;
                }
                using namespace foo;
                "#,
                "test.cpp",
            )
            .unwrap();

        // Should have namespace and using directive as children
        assert!(ast.translation_unit.children.len() >= 2);

        // Find the using directive
        let using_dir = ast.translation_unit.children.iter().find(|c| {
            matches!(&c.kind, ClangNodeKind::UsingDirective { .. })
        });

        assert!(using_dir.is_some(), "Expected UsingDirective");
        match &using_dir.unwrap().kind {
            ClangNodeKind::UsingDirective { namespace } => {
                assert_eq!(namespace, &vec!["foo"]);
            }
            _ => panic!("Expected UsingDirective"),
        }
    }

    #[test]
    fn test_parse_using_nested_namespace() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                namespace outer {
                    namespace inner {
                        int x;
                    }
                }
                using namespace outer::inner;
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the using directive
        let using_dir = ast.translation_unit.children.iter().find(|c| {
            matches!(&c.kind, ClangNodeKind::UsingDirective { .. })
        });

        assert!(using_dir.is_some(), "Expected UsingDirective");
        match &using_dir.unwrap().kind {
            ClangNodeKind::UsingDirective { namespace } => {
                assert_eq!(namespace, &vec!["outer", "inner"]);
            }
            _ => panic!("Expected UsingDirective"),
        }
    }

    #[test]
    fn test_parse_constructor_with_initializer_list() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                class Test {
                public:
                    int x;
                    int y;
                    Test(int a, int b) : x(a), y(b) { }
                };
                "#,
                "test.cpp",
            )
            .unwrap();

        // Find the class
        let class = ast.translation_unit.children.iter().find(|c| {
            matches!(&c.kind, ClangNodeKind::RecordDecl { name, .. } if name == "Test")
        });
        assert!(class.is_some(), "Expected Test class");
        let class = class.unwrap();

        // Find the constructor
        let ctor = class.children.iter().find(|c| {
            matches!(&c.kind, ClangNodeKind::ConstructorDecl { .. })
        });
        assert!(ctor.is_some(), "Expected constructor");
        let ctor = ctor.unwrap();

        // Should have MemberRef nodes for x and y
        let member_refs: Vec<_> = ctor.children.iter().filter(|c| {
            matches!(&c.kind, ClangNodeKind::MemberRef { .. })
        }).collect();
        assert_eq!(member_refs.len(), 2, "Expected 2 MemberRef nodes for x and y");

        // Verify the member names
        let member_names: Vec<String> = member_refs.iter().filter_map(|c| {
            if let ClangNodeKind::MemberRef { name } = &c.kind {
                Some(name.clone())
            } else {
                None
            }
        }).collect();
        assert!(member_names.contains(&"x".to_string()));
        assert!(member_names.contains(&"y".to_string()));
    }
}
