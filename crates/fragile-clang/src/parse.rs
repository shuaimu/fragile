//! Clang AST parsing using libclang.

use crate::ast::{
    AccessSpecifier, BinaryOp, CastKind, ClangAst, ClangNode, ClangNodeKind, ConstructorKind,
    Requirement, SourceLocation, UnaryOp,
};
use crate::types::CppType;
use miette::{miette, Result};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;

/// Parser that uses libclang to parse C++ source files.
pub struct ClangParser {
    index: clang_sys::CXIndex,
    /// Additional include paths for header files
    include_paths: Vec<String>,
}

impl ClangParser {
    /// Create a new Clang parser with default settings.
    pub fn new() -> Result<Self> {
        Self::with_include_paths(Vec::new())
    }

    /// Create a new Clang parser with custom include paths.
    pub fn with_include_paths(include_paths: Vec<String>) -> Result<Self> {
        unsafe {
            let index = clang_sys::clang_createIndex(0, 0);
            if index.is_null() {
                return Err(miette!("Failed to create clang index"));
            }
            Ok(Self {
                index,
                include_paths,
            })
        }
    }

    /// Create a Clang parser with system C++ standard library include paths.
    /// This enables parsing code that includes headers like `<vector>`, `<string>`.
    pub fn with_system_includes() -> Result<Self> {
        // Common include paths for C++ standard library
        let system_paths = Self::detect_system_include_paths();
        Self::with_include_paths(system_paths)
    }

    /// Detect system C++ include paths by querying clang.
    fn detect_system_include_paths() -> Vec<String> {
        // Common paths for libstdc++ (GCC) and libc++ (LLVM)
        let possible_paths = vec![
            // GCC libstdc++ paths (common on Linux)
            "/usr/include/c++/14".to_string(),
            "/usr/include/c++/13".to_string(),
            "/usr/include/c++/12".to_string(),
            "/usr/include/c++/11".to_string(),
            "/usr/lib/gcc/x86_64-linux-gnu/14/include".to_string(),
            "/usr/lib/gcc/x86_64-linux-gnu/13/include".to_string(),
            // Platform-specific includes
            "/usr/include/x86_64-linux-gnu/c++/14".to_string(),
            "/usr/include/x86_64-linux-gnu/c++/13".to_string(),
            "/usr/include/x86_64-linux-gnu".to_string(),
            // Standard includes
            "/usr/include".to_string(),
            "/usr/local/include".to_string(),
            // LLVM/Clang includes
            "/usr/lib/llvm-19/lib/clang/19/include".to_string(),
            "/usr/lib/llvm-18/lib/clang/18/include".to_string(),
        ];

        // Filter to only existing paths
        possible_paths
            .into_iter()
            .filter(|p| std::path::Path::new(p).exists())
            .collect()
    }

    /// Build compiler arguments including include paths.
    fn build_compiler_args(&self) -> Vec<CString> {
        let mut args = vec![
            CString::new("-x").unwrap(),
            CString::new("c++").unwrap(),
            CString::new("-std=c++20").unwrap(),
            // Suppress some warnings that may cause issues with system headers
            CString::new("-w").unwrap(),
            // Don't limit the number of errors
            CString::new("-ferror-limit=0").unwrap(),
            // Disable builtin limits on stack depth for templates
            CString::new("-ftemplate-depth=1024").unwrap(),
        ];

        // Add include paths - user paths first
        for path in &self.include_paths {
            args.push(CString::new(format!("-I{}", path)).unwrap());
        }

        args
    }

    /// Parse a C++ source file into a Clang AST.
    pub fn parse_file(&self, path: &Path) -> Result<ClangAst> {
        let path_str = path.to_string_lossy();
        let c_path = CString::new(path_str.as_ref())
            .map_err(|_| miette!("Invalid path: {}", path_str))?;

        // Compiler arguments including include paths
        let args = self.build_compiler_args();
        let c_args: Vec<*const i8> = args.iter().map(|s| s.as_ptr()).collect();

        unsafe {
            // Use DetailedPreprocessingRecord to get better AST coverage,
            // and KeepGoing to continue past errors in system headers
            let options = clang_sys::CXTranslationUnit_DetailedPreprocessingRecord
                | clang_sys::CXTranslationUnit_KeepGoing;

            let tu = clang_sys::clang_parseTranslationUnit(
                self.index,
                c_path.as_ptr(),
                c_args.as_ptr(),
                c_args.len() as i32,
                ptr::null_mut(),
                0,
                options,
            );

            if tu.is_null() {
                return Err(miette!("Failed to parse file: {}", path_str));
            }

            // Check for errors - only fail on errors in user code, not system headers
            let num_diagnostics = clang_sys::clang_getNumDiagnostics(tu);
            let mut user_errors = Vec::new();

            for i in 0..num_diagnostics {
                let diag = clang_sys::clang_getDiagnostic(tu, i);
                let severity = clang_sys::clang_getDiagnosticSeverity(diag);

                if severity >= clang_sys::CXDiagnostic_Error {
                    let spelling = clang_sys::clang_getDiagnosticSpelling(diag);
                    let msg = cx_string_to_string(spelling);

                    // Get location for better error reporting
                    let location = clang_sys::clang_getDiagnosticLocation(diag);
                    let mut file: clang_sys::CXFile = ptr::null_mut();
                    let mut line: u32 = 0;
                    let mut column: u32 = 0;
                    let mut offset: u32 = 0;
                    clang_sys::clang_getExpansionLocation(
                        location,
                        &mut file,
                        &mut line,
                        &mut column,
                        &mut offset,
                    );

                    let file_name = if !file.is_null() {
                        let fname = clang_sys::clang_getFileName(file);
                        cx_string_to_string(fname)
                    } else {
                        String::from("<unknown>")
                    };

                    // Check if this error is from a system header
                    let is_system_header = clang_sys::clang_Location_isInSystemHeader(location) != 0;

                    if !is_system_header {
                        user_errors.push(format!(
                            "{}:{}:{}: {}",
                            file_name, line, column, msg
                        ));
                    }
                }
                clang_sys::clang_disposeDiagnostic(diag);
            }

            // Only fail if there are errors in user code
            if !user_errors.is_empty() {
                clang_sys::clang_disposeTranslationUnit(tu);
                return Err(miette!(
                    "Clang errors in user code:\n{}",
                    user_errors.join("\n")
                ));
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

        // Compiler arguments including include paths
        let args = self.build_compiler_args();
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

                // CXCursor_FunctionTemplate = 30
                30 => {
                    let name = cursor_spelling(cursor);
                    let (template_params, parameter_pack_indices) = self.get_template_type_params_with_packs(cursor);

                    // Get the templated function's type info
                    // Use template-aware type conversion to detect template parameters
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let return_type = self.convert_type_with_template_ctx(
                        clang_sys::clang_getResultType(cursor_type),
                        &template_params,
                    );

                    // Extract params from ParmVarDecl children (clang_Cursor_getNumArguments
                    // returns 0 for templates)
                    let params = self.get_function_template_params(cursor, &template_params);

                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;

                    // Extract requires clause if present (C++20)
                    let requires_clause = self.get_requires_clause(cursor);

                    ClangNodeKind::FunctionTemplateDecl {
                        name,
                        template_params,
                        return_type,
                        params,
                        is_definition,
                        parameter_pack_indices,
                        requires_clause,
                    }
                }

                // CXCursor_ClassTemplate = 31
                31 => {
                    let name = cursor_spelling(cursor);
                    let (template_params, parameter_pack_indices) = self.get_template_type_params_with_packs(cursor);

                    // Determine if this is a class or struct by checking the templated decl
                    // The spelling includes "class" or "struct" prefix
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let type_spelling = clang_sys::clang_getTypeSpelling(cursor_type);
                    let type_name = cx_string_to_string(type_spelling);
                    let is_class = type_name.starts_with("class ") || !type_name.starts_with("struct ");

                    // Extract requires clause if present (C++20)
                    let requires_clause = self.get_requires_clause(cursor);

                    ClangNodeKind::ClassTemplateDecl {
                        name,
                        template_params,
                        is_class,
                        parameter_pack_indices,
                        requires_clause,
                    }
                }

                // CXCursor_ClassTemplatePartialSpecialization = 32
                32 => {
                    let name = cursor_spelling(cursor);
                    let (template_params, parameter_pack_indices) = self.get_template_type_params_with_packs(cursor);

                    // Get the specialization arguments
                    let specialization_args = self.get_template_specialization_args(cursor);

                    // Determine if this is a class or struct
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let type_spelling = clang_sys::clang_getTypeSpelling(cursor_type);
                    let type_name = cx_string_to_string(type_spelling);
                    let is_class = type_name.starts_with("class ") || !type_name.starts_with("struct ");

                    ClangNodeKind::ClassTemplatePartialSpecDecl {
                        name,
                        template_params,
                        specialization_args,
                        is_class,
                        parameter_pack_indices,
                    }
                }

                // CXCursor_TemplateTypeParameter = 27
                27 => {
                    let name = cursor_spelling(cursor);
                    // Check if this is a parameter pack (variadic template parameter)
                    // The cursor is variadic if it represents "typename... Args"
                    let is_pack = clang_sys::clang_Cursor_isVariadic(cursor) != 0;
                    // TODO: Extract proper depth and index from cursor
                    ClangNodeKind::TemplateTypeParmDecl {
                        name,
                        depth: 0,
                        index: 0,
                        is_pack,
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
                    let storage_class = clang_sys::clang_Cursor_getStorageClass(cursor);
                    let is_static = storage_class == clang_sys::CX_SC_Static;

                    // Check if this is a static member inside a class
                    let parent = clang_sys::clang_getCursorSemanticParent(cursor);
                    let parent_kind = clang_sys::clang_getCursorKind(parent);
                    let is_class_member = parent_kind == clang_sys::CXCursor_ClassDecl
                        || parent_kind == clang_sys::CXCursor_StructDecl;

                    if is_class_member && is_static {
                        // Static data member - treat as a static field
                        let access = self.get_access_specifier(cursor);
                        ClangNodeKind::FieldDecl { name, ty, access, is_static: true }
                    } else {
                        // Regular variable declaration
                        let has_init = false; // Will be determined by children
                        ClangNodeKind::VarDecl { name, ty, has_init }
                    }
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
                    // Regular field declarations are never static
                    ClangNodeKind::FieldDecl { name, ty, access, is_static: false }
                }

                clang_sys::CXCursor_CXXMethod => {
                    let name = cursor_spelling(cursor);
                    let cursor_type = clang_sys::clang_getCursorType(cursor);
                    let return_type = self.convert_type(clang_sys::clang_getResultType(cursor_type));
                    let params = self.extract_params(cursor);
                    let is_definition = clang_sys::clang_isCursorDefinition(cursor) != 0;
                    let is_static = clang_sys::clang_CXXMethod_isStatic(cursor) != 0;
                    let is_virtual = clang_sys::clang_CXXMethod_isVirtual(cursor) != 0;
                    let is_pure_virtual = clang_sys::clang_CXXMethod_isPureVirtual(cursor) != 0;
                    let (is_override, is_final) = self.get_override_final_attrs(cursor);
                    let access = self.get_access_specifier(cursor);
                    ClangNodeKind::CXXMethodDecl {
                        name,
                        return_type,
                        params,
                        is_definition,
                        is_static,
                        is_virtual,
                        is_pure_virtual,
                        is_override,
                        is_final,
                        access,
                    }
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

                // CXCursor_TypeAliasDecl = 36 (using X = Y;)
                clang_sys::CXCursor_TypeAliasDecl => {
                    let name = cursor_spelling(cursor);
                    let underlying_type = self.get_typedef_underlying_type(cursor);
                    ClangNodeKind::TypeAliasDecl { name, underlying_type }
                }

                // CXCursor_TypedefDecl = 20 (typedef Y X;)
                clang_sys::CXCursor_TypedefDecl => {
                    let name = cursor_spelling(cursor);
                    // Skip implicit/builtin typedefs (like __int128_t)
                    let loc = clang_sys::clang_getCursorLocation(cursor);
                    if clang_sys::clang_Location_isFromMainFile(loc) == 0 {
                        // Not from main file, likely a builtin typedef
                        ClangNodeKind::Unknown("implicit_typedef".to_string())
                    } else {
                        let underlying_type = self.get_typedef_underlying_type(cursor);
                        ClangNodeKind::TypedefDecl { name, underlying_type }
                    }
                }

                // CXCursor_TypeAliasTemplateDecl = 601 (template<typename T> using X = Y<T>;)
                601 => {
                    let name = cursor_spelling(cursor);
                    let template_params = self.get_template_type_params(cursor);
                    let underlying_type = self.get_type_alias_template_underlying_type(cursor, &template_params);
                    ClangNodeKind::TypeAliasTemplateDecl {
                        name,
                        template_params,
                        underlying_type,
                    }
                }

                clang_sys::CXCursor_MemberRef => {
                    let name = cursor_spelling(cursor);
                    ClangNodeKind::MemberRef { name }
                }

                clang_sys::CXCursor_FriendDecl => {
                    // Friend declaration - examine children to determine type
                    let (friend_class, friend_function) = self.get_friend_info(cursor);
                    ClangNodeKind::FriendDecl { friend_class, friend_function }
                }

                clang_sys::CXCursor_CXXBaseSpecifier => {
                    // Base class specifier (inheritance)
                    let base_type = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    let access = self.get_access_specifier(cursor);
                    let is_virtual = clang_sys::clang_isVirtualBase(cursor) != 0;
                    ClangNodeKind::CXXBaseSpecifier { base_type, access, is_virtual }
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
                clang_sys::CXCursor_SwitchStmt => ClangNodeKind::SwitchStmt,
                clang_sys::CXCursor_CaseStmt => {
                    // Evaluate the case constant
                    // The first child of CaseStmt is the constant expression
                    // Visit children to find the constant expression
                    extern "C" fn find_case_value(
                        child: clang_sys::CXCursor,
                        _parent: clang_sys::CXCursor,
                        data: clang_sys::CXClientData,
                    ) -> clang_sys::CXChildVisitResult {
                        unsafe {
                            let child_kind = clang_sys::clang_getCursorKind(child);
                            // First child should be the case constant (or ConstantExpr wrapper)
                            if child_kind == clang_sys::CXCursor_IntegerLiteral {
                                let eval = clang_sys::clang_Cursor_Evaluate(child);
                                if !eval.is_null() {
                                    let value_ptr = data as *mut i128;
                                    *value_ptr = clang_sys::clang_EvalResult_getAsInt(eval) as i128;
                                    clang_sys::clang_EvalResult_dispose(eval);
                                    return clang_sys::CXChildVisit_Break;
                                }
                            }
                            clang_sys::CXChildVisit_Recurse
                        }
                    }

                    let mut case_value: i128 = 0;
                    clang_sys::clang_visitChildren(
                        cursor,
                        find_case_value,
                        &mut case_value as *mut i128 as clang_sys::CXClientData,
                    );

                    ClangNodeKind::CaseStmt { value: case_value }
                }
                clang_sys::CXCursor_DefaultStmt => ClangNodeKind::DefaultStmt,

                // C++ Exception Handling
                clang_sys::CXCursor_CXXTryStmt => ClangNodeKind::TryStmt,

                clang_sys::CXCursor_CXXCatchStmt => {
                    // Get the exception type from the first child if it's a VarDecl
                    let exception_ty = self.get_catch_exception_type(cursor);
                    ClangNodeKind::CatchStmt { exception_ty }
                }

                clang_sys::CXCursor_CXXThrowExpr => {
                    // Get the type being thrown from the child expression
                    let exception_ty = self.get_throw_exception_type(cursor);
                    ClangNodeKind::ThrowExpr { exception_ty }
                }

                // C++ RTTI
                clang_sys::CXCursor_CXXTypeidExpr => {
                    let result_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::TypeidExpr { result_ty }
                }

                clang_sys::CXCursor_CXXDynamicCastExpr => {
                    // The result type is the target type of the cast
                    let target_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    ClangNodeKind::DynamicCastExpr { target_ty }
                }

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

                // C++20 Concepts
                // CXCursor_ConceptDecl = 604
                604 => {
                    let name = cursor_spelling(cursor);
                    let template_params = self.get_template_type_params(cursor);
                    let constraint_expr = self.get_concept_constraint_expr(cursor);

                    ClangNodeKind::ConceptDecl {
                        name,
                        template_params,
                        constraint_expr,
                    }
                }

                // CXCursor_RequiresExpr = 279
                279 => {
                    let params = self.get_requires_expr_params(cursor);
                    let requirements = self.get_requirements(cursor);

                    ClangNodeKind::RequiresExpr {
                        params,
                        requirements,
                    }
                }

                // CXCursor_ConceptSpecializationExpr = 602
                602 => {
                    let (concept_name, template_args) = self.get_concept_specialization_info(cursor);

                    ClangNodeKind::ConceptSpecializationExpr {
                        concept_name,
                        template_args,
                    }
                }

                // C++20 Coroutines - libclang maps these to UnexposedExpr/UnexposedStmt
                // We detect them by tokenizing and looking for co_await, co_yield, co_return keywords
                clang_sys::CXCursor_UnexposedExpr => {
                    if let Some(coroutine_kind) = self.try_parse_coroutine_expr(cursor) {
                        coroutine_kind
                    } else {
                        // Fall back to Unknown for non-coroutine unexposed expressions
                        let kind_spelling = clang_sys::clang_getCursorKindSpelling(kind);
                        ClangNodeKind::Unknown(cx_string_to_string(kind_spelling))
                    }
                }

                clang_sys::CXCursor_UnexposedStmt => {
                    if let Some(coroutine_kind) = self.try_parse_coroutine_stmt(cursor) {
                        coroutine_kind
                    } else {
                        // Fall back to Unknown for non-coroutine unexposed statements
                        let kind_spelling = clang_sys::clang_getCursorKindSpelling(kind);
                        ClangNodeKind::Unknown(cx_string_to_string(kind_spelling))
                    }
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
                    let is_rvalue = kind == clang_sys::CXType_RValueReference;
                    CppType::Reference {
                        referent: Box::new(self.convert_type(referent)),
                        is_const,
                        is_rvalue,
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

    /// Get binary operator from cursor by tokenizing and finding the operator token.
    fn get_binary_op(&self, cursor: clang_sys::CXCursor) -> BinaryOp {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = std::ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            let mut result = BinaryOp::Add; // Default

            // Binary operators have the pattern: left_expr OP right_expr
            // We look for punctuation tokens that are operators
            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let token_kind = clang_sys::clang_getTokenKind(token);

                // CXToken_Punctuation = 1
                if token_kind == 1 {
                    let token_spelling = clang_sys::clang_getTokenSpelling(tu, token);
                    let token_str = cx_string_to_string(token_spelling);

                    result = match token_str.as_str() {
                        "+" => BinaryOp::Add,
                        "-" => BinaryOp::Sub,
                        "*" => BinaryOp::Mul,
                        "/" => BinaryOp::Div,
                        "%" => BinaryOp::Rem,
                        "&" => BinaryOp::And,
                        "|" => BinaryOp::Or,
                        "^" => BinaryOp::Xor,
                        "<<" => BinaryOp::Shl,
                        ">>" => BinaryOp::Shr,
                        "==" => BinaryOp::Eq,
                        "!=" => BinaryOp::Ne,
                        "<" => BinaryOp::Lt,
                        "<=" => BinaryOp::Le,
                        ">" => BinaryOp::Gt,
                        ">=" => BinaryOp::Ge,
                        "&&" => BinaryOp::LAnd,
                        "||" => BinaryOp::LOr,
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
                        _ => continue, // Not an operator we recognize, keep looking
                    };
                    // Found an operator, but keep going to find compound operators
                    // (e.g., << comes before <)
                    // Actually, Clang tokenizes compound operators as single tokens,
                    // so break on first match
                    break;
                }
            }

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            result
        }
    }

    /// Get unary operator from cursor by tokenizing and finding the operator token.
    fn get_unary_op(&self, cursor: clang_sys::CXCursor) -> UnaryOp {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = std::ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            let mut result = UnaryOp::Minus; // Default

            // Unary operators: prefix (++x, --x, -x, +x, !x, ~x, *x, &x)
            //                  postfix (x++, x--)
            // We look at the first and last punctuation tokens
            let mut first_punct: Option<String> = None;
            let mut last_punct: Option<String> = None;

            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let token_kind = clang_sys::clang_getTokenKind(token);

                // CXToken_Punctuation = 1
                if token_kind == 1 {
                    let token_spelling = clang_sys::clang_getTokenSpelling(tu, token);
                    let token_str = cx_string_to_string(token_spelling);

                    if first_punct.is_none() {
                        first_punct = Some(token_str.clone());
                    }
                    last_punct = Some(token_str);
                }
            }

            // For prefix operators, the operator is first
            // For postfix operators (++, --), the operator is last
            // We try to detect based on position
            if let Some(ref op) = first_punct {
                result = match op.as_str() {
                    "++" => {
                        // Could be prefix or postfix, check if it's at the start
                        // If first_punct == last_punct and there are other tokens,
                        // it's likely prefix
                        UnaryOp::PreInc
                    }
                    "--" => UnaryOp::PreDec,
                    "-" => UnaryOp::Minus,
                    "+" => UnaryOp::Plus,
                    "!" => UnaryOp::LNot,
                    "~" => UnaryOp::Not,
                    "*" => UnaryOp::Deref,
                    "&" => UnaryOp::AddrOf,
                    _ => UnaryOp::Minus,
                };
            }

            // Check for postfix operators
            if let Some(ref op) = last_punct {
                if first_punct.as_ref() != last_punct.as_ref() {
                    // Different operators at start and end - last one might be postfix
                    match op.as_str() {
                        "++" => result = UnaryOp::PostInc,
                        "--" => result = UnaryOp::PostDec,
                        _ => {}
                    }
                }
            }

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            result
        }
    }

    /// Try to parse a coroutine expression (co_await or co_yield) from an UnexposedExpr.
    /// Returns Some(ClangNodeKind) if this is a coroutine expression, None otherwise.
    fn try_parse_coroutine_expr(&self, cursor: clang_sys::CXCursor) -> Option<ClangNodeKind> {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            if tokens.is_null() || num_tokens == 0 {
                return None;
            }

            // Get the first token to check for co_await or co_yield
            let first_token = *tokens;
            let first_spelling = clang_sys::clang_getTokenSpelling(tu, first_token);
            let first_str = cx_string_to_string(first_spelling);

            let result = match first_str.as_str() {
                "co_await" => {
                    // Get the result type from the cursor type
                    let result_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // For operand type, we'll use the result type as a placeholder
                    // In practice, we'd need to examine the child expression
                    let operand_ty = self.get_coroutine_operand_type(cursor);
                    Some(ClangNodeKind::CoawaitExpr {
                        operand_ty,
                        result_ty,
                    })
                }
                "co_yield" => {
                    // Get the result type from the cursor type
                    let result_ty = self.convert_type(clang_sys::clang_getCursorType(cursor));
                    // For value type, examine the child expression
                    let value_ty = self.get_coroutine_operand_type(cursor);
                    Some(ClangNodeKind::CoyieldExpr { value_ty, result_ty })
                }
                _ => None,
            };

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            result
        }
    }

    /// Try to parse a coroutine statement (co_return) from an UnexposedStmt.
    /// Returns Some(ClangNodeKind) if this is a coroutine statement, None otherwise.
    fn try_parse_coroutine_stmt(&self, cursor: clang_sys::CXCursor) -> Option<ClangNodeKind> {
        unsafe {
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            if tokens.is_null() || num_tokens == 0 {
                return None;
            }

            // Get the first token to check for co_return
            let first_token = *tokens;
            let first_spelling = clang_sys::clang_getTokenSpelling(tu, first_token);
            let first_str = cx_string_to_string(first_spelling);

            let result = if first_str == "co_return" {
                // Check if there's a value being returned
                // If num_tokens > 2 (co_return + ; + something), there's a value
                let value_ty = if num_tokens > 2 {
                    Some(self.get_coroutine_operand_type(cursor))
                } else {
                    None
                };
                Some(ClangNodeKind::CoreturnStmt { value_ty })
            } else {
                None
            };

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            result
        }
    }

    /// Get the operand type for a coroutine expression by examining its first child.
    fn get_coroutine_operand_type(&self, cursor: clang_sys::CXCursor) -> CppType {
        unsafe {
            // Visit children to find the operand expression
            struct OperandTypeData {
                ty: CppType,
                parser: *const ClangParser,
            }

            extern "C" fn find_operand_type(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(data as *mut OperandTypeData);
                    let child_type = clang_sys::clang_getCursorType(child);
                    data.ty = (*data.parser).convert_type(child_type);
                    clang_sys::CXChildVisit_Break
                }
            }

            let mut data = OperandTypeData {
                ty: CppType::Void,
                parser: self as *const ClangParser,
            };

            clang_sys::clang_visitChildren(
                cursor,
                find_operand_type,
                &mut data as *mut OperandTypeData as clang_sys::CXClientData,
            );

            data.ty
        }
    }

    /// Get the exception type for a catch statement.
    /// Returns None for `catch(...)` (catch all).
    fn get_catch_exception_type(&self, cursor: clang_sys::CXCursor) -> Option<CppType> {
        unsafe {
            // The first child of a catch statement is the exception declaration (VarDecl)
            // or nothing for catch(...)
            struct CatchTypeData {
                ty: Option<CppType>,
                parser: *const ClangParser,
            }

            extern "C" fn find_catch_type(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(data as *mut CatchTypeData);
                    let child_kind = clang_sys::clang_getCursorKind(child);

                    // Look for VarDecl which contains the exception type
                    if child_kind == clang_sys::CXCursor_VarDecl {
                        let child_type = clang_sys::clang_getCursorType(child);
                        data.ty = Some((*data.parser).convert_type(child_type));
                        return clang_sys::CXChildVisit_Break;
                    }
                    clang_sys::CXChildVisit_Continue
                }
            }

            let mut data = CatchTypeData {
                ty: None,
                parser: self as *const ClangParser,
            };

            clang_sys::clang_visitChildren(
                cursor,
                find_catch_type,
                &mut data as *mut CatchTypeData as clang_sys::CXClientData,
            );

            data.ty
        }
    }

    /// Get the exception type for a throw expression.
    /// Returns None for `throw;` (rethrow).
    fn get_throw_exception_type(&self, cursor: clang_sys::CXCursor) -> Option<CppType> {
        unsafe {
            // The child of a throw expression is the expression being thrown
            // No child means it's a rethrow (throw;)
            struct ThrowTypeData {
                ty: Option<CppType>,
                parser: *const ClangParser,
            }

            extern "C" fn find_throw_type(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(data as *mut ThrowTypeData);
                    let child_type = clang_sys::clang_getCursorType(child);
                    data.ty = Some((*data.parser).convert_type(child_type));
                    clang_sys::CXChildVisit_Break
                }
            }

            let mut data = ThrowTypeData {
                ty: None,
                parser: self as *const ClangParser,
            };

            clang_sys::clang_visitChildren(
                cursor,
                find_throw_type,
                &mut data as *mut ThrowTypeData as clang_sys::CXClientData,
            );

            data.ty
        }
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

    /// Get override and final attributes from a method cursor.
    /// Returns (is_override, is_final).
    fn get_override_final_attrs(&self, cursor: clang_sys::CXCursor) -> (bool, bool) {
        unsafe {
            struct AttrInfo {
                is_override: bool,
                is_final: bool,
            }
            let mut info = AttrInfo {
                is_override: false,
                is_final: false,
            };
            let info_ptr: *mut AttrInfo = &mut info;

            extern "C" fn attr_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut AttrInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_CXXOverrideAttr = 405
                    // CXCursor_CXXFinalAttr = 404
                    match kind {
                        404 => info.is_final = true,
                        405 => info.is_override = true,
                        _ => {}
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(cursor, attr_visitor, info_ptr as clang_sys::CXClientData);

            (info.is_override, info.is_final)
        }
    }

    /// Convert a type with template parameter awareness.
    ///
    /// If the type spelling matches a known template parameter, returns a
    /// TemplateParam variant instead of Named.
    fn convert_type_with_template_ctx(
        &self,
        ty: clang_sys::CXType,
        template_params: &[String],
    ) -> CppType {
        unsafe {
            let kind = ty.kind;

            // First, handle structural types (pointers, references, arrays) by
            // recursively processing the inner type with template context
            match kind {
                clang_sys::CXType_Pointer => {
                    let pointee = clang_sys::clang_getPointeeType(ty);
                    let is_const = clang_sys::clang_isConstQualifiedType(pointee) != 0;
                    return CppType::Pointer {
                        pointee: Box::new(self.convert_type_with_template_ctx(pointee, template_params)),
                        is_const,
                    };
                }

                clang_sys::CXType_LValueReference | clang_sys::CXType_RValueReference => {
                    let referent = clang_sys::clang_getPointeeType(ty);
                    let is_const = clang_sys::clang_isConstQualifiedType(referent) != 0;
                    let is_rvalue = kind == clang_sys::CXType_RValueReference;
                    return CppType::Reference {
                        referent: Box::new(self.convert_type_with_template_ctx(referent, template_params)),
                        is_const,
                        is_rvalue,
                    };
                }

                clang_sys::CXType_ConstantArray => {
                    let element = clang_sys::clang_getArrayElementType(ty);
                    let size = clang_sys::clang_getArraySize(ty) as usize;
                    return CppType::Array {
                        element: Box::new(self.convert_type_with_template_ctx(element, template_params)),
                        size: Some(size),
                    };
                }

                clang_sys::CXType_IncompleteArray => {
                    let element = clang_sys::clang_getArrayElementType(ty);
                    return CppType::Array {
                        element: Box::new(self.convert_type_with_template_ctx(element, template_params)),
                        size: None,
                    };
                }

                _ => {}
            }

            // For non-structural types, check the spelling
            let spelling = clang_sys::clang_getTypeSpelling(ty);
            let type_name = cx_string_to_string(spelling);

            // Strip "const " prefix for matching purposes
            // The const qualifier is already captured in the parent type
            let base_name = type_name.trim_start_matches("const ").to_string();

            // Check if this is a template parameter by name
            if let Some(index) = template_params.iter().position(|p| p == &base_name) {
                return CppType::TemplateParam {
                    name: base_name,
                    depth: 0,
                    index: index as u32,
                };
            }

            // Check for internal template parameter format: "type-parameter-{depth}-{index}"
            // libclang uses this format for template parameters in partial specializations
            if base_name.starts_with("type-parameter-") {
                let parts: Vec<_> = base_name["type-parameter-".len()..].split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(depth), Ok(index)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                        // Map the index to the template parameter name
                        let name = if (index as usize) < template_params.len() {
                            template_params[index as usize].clone()
                        } else {
                            base_name.clone()
                        };
                        return CppType::TemplateParam {
                            name,
                            depth,
                            index,
                        };
                    }
                }
            }

            // Check for dependent types (types that contain template params)
            let is_dependent = template_params.iter().any(|p| base_name.contains(p));

            if is_dependent {
                // For now, store dependent types with their full spelling
                // A more sophisticated approach would parse and reconstruct the type
                CppType::DependentType { spelling: type_name }
            } else {
                self.convert_type(ty)
            }
        }
    }

    /// Get function parameters from a function template cursor.
    ///
    /// For function templates, `clang_Cursor_getNumArguments` returns 0, so we need
    /// to extract parameters from the ParmVarDecl children.
    fn get_function_template_params(
        &self,
        cursor: clang_sys::CXCursor,
        template_params: &[String],
    ) -> Vec<(String, CppType)> {
        unsafe {
            struct ParamData<'a> {
                params: Vec<(String, CppType)>,
                parser: &'a ClangParser,
                template_params: &'a [String],
            }

            let mut data = ParamData {
                params: Vec::new(),
                parser: self,
                template_params,
            };
            let data_ptr: *mut ParamData = &mut data;

            extern "C" fn param_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                client_data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(client_data as *mut ParamData);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_ParmDecl = 10
                    if kind == 10 {
                        let name = cursor_spelling(child);
                        let ty = clang_sys::clang_getCursorType(child);
                        let cpp_type = data
                            .parser
                            .convert_type_with_template_ctx(ty, data.template_params);
                        data.params.push((name, cpp_type));
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(
                cursor,
                param_visitor,
                data_ptr as clang_sys::CXClientData,
            );

            data.params
        }
    }

    /// Get template type parameters from a function/class template cursor.
    /// Returns the names of the template type parameters (e.g., ["T", "U"]).
    fn get_template_type_params(&self, cursor: clang_sys::CXCursor) -> Vec<String> {
        let (names, _) = self.get_template_type_params_with_packs(cursor);
        names
    }

    /// Get template type parameters with pack info from a function/class template cursor.
    /// Returns (parameter names, indices of parameter packs).
    ///
    /// Note: We need to tokenize each template type parameter's extent to detect "..."
    /// because libclang doesn't expose isParameterPack() in its C API.
    fn get_template_type_params_with_packs(&self, cursor: clang_sys::CXCursor) -> (Vec<String>, Vec<usize>) {
        unsafe {
            // Get the translation unit for tokenization
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);

            struct ParamInfo {
                names: Vec<String>,
                pack_indices: Vec<usize>,
                tu: clang_sys::CXTranslationUnit,
            }
            let mut info = ParamInfo {
                names: Vec::new(),
                pack_indices: Vec::new(),
                tu,
            };
            let info_ptr: *mut ParamInfo = &mut info;

            extern "C" fn param_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut ParamInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_TemplateTypeParameter = 27
                    if kind == 27 {
                        let name = cursor_spelling(child);
                        if !name.is_empty() {
                            let index = info.names.len();

                            // Tokenize the extent to detect "..." token
                            let extent = clang_sys::clang_getCursorExtent(child);
                            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
                            let mut num_tokens: u32 = 0;

                            clang_sys::clang_tokenize(info.tu, extent, &mut tokens, &mut num_tokens);

                            let mut is_pack = false;
                            for i in 0..num_tokens {
                                let token = *tokens.add(i as usize);
                                let token_spelling = clang_sys::clang_getTokenSpelling(info.tu, token);
                                let token_str = cx_string_to_string(token_spelling);
                                if token_str == "..." {
                                    is_pack = true;
                                    break;
                                }
                            }

                            if !tokens.is_null() {
                                clang_sys::clang_disposeTokens(info.tu, tokens, num_tokens);
                            }

                            if is_pack {
                                info.pack_indices.push(index);
                            }
                            info.names.push(name);
                        }
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(cursor, param_visitor, info_ptr as clang_sys::CXClientData);

            (info.names, info.pack_indices)
        }
    }

    /// Get the template specialization arguments from a partial specialization cursor.
    /// Returns the types used in the specialization pattern (e.g., [T, T] for Pair<T, T>).
    fn get_template_specialization_args(&self, cursor: clang_sys::CXCursor) -> Vec<CppType> {
        unsafe {
            // Get the specialized type (e.g., Pair<T, T>)
            let cursor_type = clang_sys::clang_getCursorType(cursor);

            // Get the number of template arguments
            let num_args = clang_sys::clang_Type_getNumTemplateArguments(cursor_type);
            if num_args < 0 {
                return Vec::new();
            }

            // Get the template parameters for this partial specialization
            let template_params = self.get_template_type_params(cursor);

            // Extract each template argument
            let mut args = Vec::new();
            for i in 0..num_args {
                let arg_type = clang_sys::clang_Type_getTemplateArgumentAsType(cursor_type, i as u32);
                // Use template-aware conversion to detect template parameter references
                let cpp_type = self.convert_type_with_template_ctx(arg_type, &template_params);
                args.push(cpp_type);
            }

            args
        }
    }

    /// Get friend information from a FriendDecl cursor.
    /// Returns (friend_class, friend_function) where one will be Some and the other None.
    fn get_friend_info(&self, cursor: clang_sys::CXCursor) -> (Option<String>, Option<String>) {
        unsafe {
            // Structure to pass both options through the visitor
            struct FriendInfo {
                friend_class: Option<String>,
                friend_function: Option<String>,
            }
            let mut info = FriendInfo {
                friend_class: None,
                friend_function: None,
            };
            let info_ptr: *mut FriendInfo = &mut info;

            extern "C" fn friend_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut FriendInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    match kind {
                        // Friend class declaration
                        // CXCursor_ClassDecl = 4, CXCursor_StructDecl = 2
                        // Also check for ClassTemplate (31) which can appear for forward-declared friends
                        clang_sys::CXCursor_ClassDecl
                        | clang_sys::CXCursor_StructDecl
                        | clang_sys::CXCursor_ClassTemplate => {
                            let name = cursor_spelling(child);
                            if !name.is_empty() {
                                info.friend_class = Some(name);
                            }
                        }
                        // Friend function declaration
                        clang_sys::CXCursor_FunctionDecl => {
                            let name = cursor_spelling(child);
                            if !name.is_empty() {
                                info.friend_function = Some(name);
                            }
                        }
                        // For types, sometimes the type reference appears instead
                        clang_sys::CXCursor_TypeRef => {
                            let name = cursor_spelling(child);
                            if !name.is_empty() && info.friend_class.is_none() {
                                // Strip "class " or "struct " prefix if present
                                let name = name
                                    .strip_prefix("class ")
                                    .or_else(|| name.strip_prefix("struct "))
                                    .unwrap_or(&name)
                                    .to_string();
                                info.friend_class = Some(name);
                            }
                        }
                        _ => {}
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(cursor, friend_visitor, info_ptr as clang_sys::CXClientData);

            (info.friend_class, info.friend_function)
        }
    }

    // ========== C++20 Concepts Support ==========

    /// Get the requires clause from a template cursor, if present.
    /// Returns the constraint expression as a string, or None if no requires clause.
    fn get_requires_clause(&self, cursor: clang_sys::CXCursor) -> Option<String> {
        unsafe {
            struct RequiresInfo {
                constraint: Option<String>,
                tu: clang_sys::CXTranslationUnit,
            }
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let mut info = RequiresInfo {
                constraint: None,
                tu,
            };
            let info_ptr: *mut RequiresInfo = &mut info;

            extern "C" fn requires_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut RequiresInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_RequiresExpr = 279, CXCursor_ConceptSpecializationExpr = 602
                    // These are the only cursors that represent requires clauses
                    if kind == 279 || kind == 602 {
                        // Found a constraint - extract the text from the source range
                        let extent = clang_sys::clang_getCursorExtent(child);
                        let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
                        let mut num_tokens: u32 = 0;

                        clang_sys::clang_tokenize(info.tu, extent, &mut tokens, &mut num_tokens);

                        let mut constraint_parts = Vec::new();
                        for i in 0..num_tokens {
                            let token = *tokens.add(i as usize);
                            let spelling = clang_sys::clang_getTokenSpelling(info.tu, token);
                            constraint_parts.push(cx_string_to_string(spelling));
                        }

                        if !tokens.is_null() {
                            clang_sys::clang_disposeTokens(info.tu, tokens, num_tokens);
                        }

                        if !constraint_parts.is_empty() {
                            info.constraint = Some(constraint_parts.join(" "));
                            return clang_sys::CXChildVisit_Break;
                        }
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(cursor, requires_visitor, info_ptr as clang_sys::CXClientData);

            info.constraint
        }
    }

    /// Get the constraint expression from a concept declaration.
    fn get_concept_constraint_expr(&self, cursor: clang_sys::CXCursor) -> String {
        unsafe {
            // Tokenize the entire cursor extent to get the constraint expression
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let extent = clang_sys::clang_getCursorExtent(cursor);
            let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
            let mut num_tokens: u32 = 0;

            clang_sys::clang_tokenize(tu, extent, &mut tokens, &mut num_tokens);

            let mut all_tokens = Vec::new();
            for i in 0..num_tokens {
                let token = *tokens.add(i as usize);
                let spelling = clang_sys::clang_getTokenSpelling(tu, token);
                all_tokens.push(cx_string_to_string(spelling));
            }

            if !tokens.is_null() {
                clang_sys::clang_disposeTokens(tu, tokens, num_tokens);
            }

            // The constraint is everything after "=" in the concept definition
            // e.g., "template < typename T > concept Integral = __is_integral ( T )"
            // We want: "__is_integral ( T )"
            if let Some(eq_pos) = all_tokens.iter().position(|t| t == "=") {
                all_tokens[eq_pos + 1..].join(" ")
            } else {
                String::new()
            }
        }
    }

    /// Get parameters from a requires expression.
    fn get_requires_expr_params(&self, cursor: clang_sys::CXCursor) -> Vec<(String, CppType)> {
        unsafe {
            struct ParamData<'a> {
                params: Vec<(String, CppType)>,
                parser: &'a ClangParser,
            }
            let mut data = ParamData {
                params: Vec::new(),
                parser: self,
            };
            let data_ptr: *mut ParamData = &mut data;

            extern "C" fn param_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                client_data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(client_data as *mut ParamData);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_ParmDecl = 10
                    if kind == 10 {
                        let name = cursor_spelling(child);
                        let ty = clang_sys::clang_getCursorType(child);
                        let cpp_type = data.parser.convert_type(ty);
                        data.params.push((name, cpp_type));
                    }

                    // Only visit direct children for parameters
                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(cursor, param_visitor, data_ptr as clang_sys::CXClientData);

            data.params
        }
    }

    /// Get requirements from a requires expression.
    fn get_requirements(&self, cursor: clang_sys::CXCursor) -> Vec<Requirement> {
        unsafe {
            struct ReqData<'a> {
                requirements: Vec<Requirement>,
                tu: clang_sys::CXTranslationUnit,
                #[allow(dead_code)]
                parser: &'a ClangParser,
            }
            let tu = clang_sys::clang_Cursor_getTranslationUnit(cursor);
            let mut data = ReqData {
                requirements: Vec::new(),
                tu,
                parser: self,
            };
            let data_ptr: *mut ReqData = &mut data;

            extern "C" fn req_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                client_data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let data = &mut *(client_data as *mut ReqData);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // Skip parameter declarations
                    if kind == 10 {
                        return clang_sys::CXChildVisit_Continue;
                    }

                    // Extract the text of each requirement from tokens
                    let extent = clang_sys::clang_getCursorExtent(child);
                    let mut tokens: *mut clang_sys::CXToken = ptr::null_mut();
                    let mut num_tokens: u32 = 0;

                    clang_sys::clang_tokenize(data.tu, extent, &mut tokens, &mut num_tokens);

                    let mut token_strs = Vec::new();
                    for i in 0..num_tokens {
                        let token = *tokens.add(i as usize);
                        let spelling = clang_sys::clang_getTokenSpelling(data.tu, token);
                        token_strs.push(cx_string_to_string(spelling));
                    }

                    if !tokens.is_null() {
                        clang_sys::clang_disposeTokens(data.tu, tokens, num_tokens);
                    }

                    if !token_strs.is_empty() {
                        let expr = token_strs.join(" ");

                        // Detect requirement type based on tokens
                        if token_strs.first().map(|s| s.as_str()) == Some("typename") {
                            // Type requirement
                            data.requirements.push(Requirement::Type {
                                type_name: token_strs[1..].join(" "),
                            });
                        } else if token_strs.first().map(|s| s.as_str()) == Some("{") {
                            // Compound requirement
                            let is_noexcept = token_strs.contains(&"noexcept".to_string());
                            let return_constraint = if let Some(arrow_pos) = token_strs.iter().position(|t| t == "->") {
                                Some(token_strs[arrow_pos + 1..].join(" "))
                            } else {
                                None
                            };
                            data.requirements.push(Requirement::Compound {
                                expr,
                                is_noexcept,
                                return_constraint,
                            });
                        } else if token_strs.first().map(|s| s.as_str()) == Some("requires") {
                            // Nested requirement
                            data.requirements.push(Requirement::Nested {
                                constraint: token_strs[1..].join(" "),
                            });
                        } else {
                            // Simple requirement
                            data.requirements.push(Requirement::Simple { expr });
                        }
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(cursor, req_visitor, data_ptr as clang_sys::CXClientData);

            data.requirements
        }
    }

    /// Get concept specialization information (concept name and template arguments).
    fn get_concept_specialization_info(&self, cursor: clang_sys::CXCursor) -> (String, Vec<CppType>) {
        unsafe {
            // Get the concept name from the cursor spelling
            let concept_name = cursor_spelling(cursor);

            // Get template arguments from the type
            let cursor_type = clang_sys::clang_getCursorType(cursor);
            let num_args = clang_sys::clang_Type_getNumTemplateArguments(cursor_type);

            let mut template_args = Vec::new();
            if num_args > 0 {
                for i in 0..num_args {
                    let arg_type = clang_sys::clang_Type_getTemplateArgumentAsType(cursor_type, i as u32);
                    template_args.push(self.convert_type(arg_type));
                }
            }

            (concept_name, template_args)
        }
    }

    // ========== Type Alias Support ==========

    /// Get the underlying type of a typedef or type alias declaration.
    fn get_typedef_underlying_type(&self, cursor: clang_sys::CXCursor) -> CppType {
        unsafe {
            let typedef_type = clang_sys::clang_getTypedefDeclUnderlyingType(cursor);
            self.convert_type(typedef_type)
        }
    }

    /// Get the underlying type of a type alias template declaration.
    /// This needs special handling because the underlying type may reference template parameters.
    fn get_type_alias_template_underlying_type(
        &self,
        cursor: clang_sys::CXCursor,
        template_params: &[String],
    ) -> CppType {
        unsafe {
            // For type alias templates, we need to find the TypeAliasDecl child
            // and get its underlying type with template parameter context
            struct AliasInfo {
                underlying_type: Option<CppType>,
                parser: *const ClangParser,
                template_params: *const [String],
            }

            let mut info = AliasInfo {
                underlying_type: None,
                parser: self,
                template_params: template_params,
            };
            let info_ptr: *mut AliasInfo = &mut info;

            extern "C" fn alias_visitor(
                child: clang_sys::CXCursor,
                _parent: clang_sys::CXCursor,
                data: clang_sys::CXClientData,
            ) -> clang_sys::CXChildVisitResult {
                unsafe {
                    let info = &mut *(data as *mut AliasInfo);
                    let kind = clang_sys::clang_getCursorKind(child);

                    // CXCursor_TypeAliasDecl = 36
                    if kind == clang_sys::CXCursor_TypeAliasDecl {
                        let typedef_type = clang_sys::clang_getTypedefDeclUnderlyingType(child);
                        let parser = &*info.parser;
                        let template_params = &*info.template_params;
                        info.underlying_type = Some(parser.convert_type_with_template_ctx(
                            typedef_type,
                            template_params,
                        ));
                        return clang_sys::CXChildVisit_Break;
                    }

                    clang_sys::CXChildVisit_Continue
                }
            }

            clang_sys::clang_visitChildren(cursor, alias_visitor, info_ptr as clang_sys::CXClientData);

            info.underlying_type.unwrap_or(CppType::Void)
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
